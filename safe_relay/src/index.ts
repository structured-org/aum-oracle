import toml from "toml";
import SafeApiKit from '@safe-global/api-kit'
import { createPublicClient, decodeFunctionData, encodeFunctionData, http } from 'viem';
import { mainnet } from 'viem/chains';
import { mnemonicToAccount, type HDAccount } from 'viem/accounts';
import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import type { Logger } from "pino";
import cron from "node-cron";
import { getLogger } from "./logger";
import { OperationType, type MetaTransactionData } from "@safe-global/types-kit";
import Safe from "@safe-global/protocol-kit";

const RECEIVER_ABI = require("./Receiver.abi.json");

function toBigIntTimes10Pow(str: string, decimals = 18): BigInt {
  str = str.trim();
  if (str.length === 0) {
    return 0n;
  }
  const [intPart = "", fracPart = ""] = str.split(".");
  const combined = intPart + fracPart.padEnd(decimals, "0");
  const normalized = combined.slice(0, intPart.length + decimals);
  return BigInt(normalized);
}

class Bot {
    #account: HDAccount
    #privateKey: string | undefined;
    #safeAddress: string; 
    #receiverAddress: string; 
    #safeApi: SafeApiKit | undefined;
    #safeClient: Safe | undefined;
    #cosmWasmClient: CosmWasmClient | undefined;
    #safeApiKey: string;
    #ethereumRpcUrl: string = process.env.ETHEREUM_RPC_URL || '';
    #neutronRpcUrl: string = process.env.NEUTRON_RPC_URL || ''
    #twaerContract: string = process.env.TWAER_CONTRACT || ''
    #logger: Logger<never>
    #ethClient: ReturnType<typeof createPublicClient>;
    #verifyValuesOnVote: boolean;
    #cronExpression: string;
    #msParticipantIndex: number | undefined;
    #delayTimeMs: number;

    constructor() {
        const configFile = process.env.CONFIG_FILE || 'config.toml';
        const config = toml.parse(require('fs').readFileSync(configFile, 'utf-8'));
        this.#account =  mnemonicToAccount(config.safe_relayer.mnemonic);
        this.#safeAddress = config.safe_relayer.ethereum.safe_address;
        this.#receiverAddress = config.safe_relayer.ethereum.receiver_address;
        this.#safeApiKey = config.safe_relayer.ethereum.safe_api_key;
        this.#ethereumRpcUrl = config.safe_relayer.ethereum.rpc;
        this.#neutronRpcUrl = config.safe_relayer.neutron.rpc;
        this.#twaerContract = config.safe_relayer.neutron.twaer_contract;
        this.#logger = getLogger(config.safe_relayer.log_level);
        this.#ethClient = createPublicClient({
                            chain: mainnet,
                            transport: http(this.#ethereumRpcUrl),
                          });
        this.#verifyValuesOnVote = config.safe_relayer.verify_values_on_vote;
        this.#delayTimeMs = config.safe_relayer.delay_time_seconds * 1000;
        this.#cronExpression = config.safe_relayer.interval_cron;
    }

    async init() {
        const pk = await this.#account.getHdKey().privateKey;
        this.#privateKey = pk ? `0x${Buffer.from(pk).toString('hex')}` : undefined;
        this.#safeApi = new SafeApiKit({chainId: 1n, apiKey: this.#safeApiKey });
        this.#safeClient = await Safe.init({
            provider: this.#ethereumRpcUrl,
            signer: this.#privateKey,
            safeAddress: this.#safeAddress,
        });
        const safeInfo = await this.#safeApi.getSafeInfo(this.#safeAddress);
        this.#msParticipantIndex = safeInfo.owners.findIndex(owner => owner.toLowerCase() === this.#account.address.toLowerCase());
        if (this.#msParticipantIndex === -1) {
            throw new Error("Account is not an in the list of participants in Safe");
        }
        this.#cosmWasmClient = await CosmWasmClient.connect(this.#neutronRpcUrl);
        this.#delayTimeMs = this.#delayTimeMs * this.#msParticipantIndex;
        cron.schedule(this.#cronExpression, async () =>{ this.tick(); });
    }


    async tick() {
        this.#logger.trace("Starting tick in %d ms", this.#delayTimeMs);
        await new Promise(resolve => setTimeout(resolve, this.#delayTimeMs));
        this.#logger.info("Executing tick");
        const neutronData = await this.getTWAERData();
        const ethereumData = await this.getReceverData();
        const pendingProposals = await this.getPendingProposals();
        if (pendingProposals.results.length > 0) {
            this.#logger.info("There are pending proposals. Skipping new proposal submission.");
            const proposals = pendingProposals.results.filter(p => (p.to === this.#receiverAddress) && ((p.confirmations || []).every(c => c.owner.toLowerCase() !== this.#account.address.toLowerCase())));
            if (proposals.length > 0) {
                this.#logger.trace("There is a proposal to vote on.");
                const proposal = proposals[0];
                if (!proposal) return;
                if (this.#verifyValuesOnVote && proposal) {
                    const dataFromProposal = decodeFunctionData({
                        abi: RECEIVER_ABI,
                        data: proposal.data as `0x${string}`,
                    });
                    this.#logger.trace("Data from Proposal: %o", dataFromProposal);
                    if (dataFromProposal.functionName !== 'publish') {
                        this.#logger.error("Unexpected function name in proposal: %s", dataFromProposal.functionName);
                        return;
                    }
                    const [erFromProposal, tsFromProposal] = dataFromProposal.args as [BigInt, BigInt];
                    if (erFromProposal !== neutronData.er || Number(tsFromProposal) !== neutronData.ts) {
                        this.#logger.warn("Data in proposal does not match Neutron data. Neutron ER: %o, TS: %d; Proposal ER: %d, TS: %d", neutronData.er, neutronData.ts, Number(erFromProposal), Number(tsFromProposal));
                        return;
                    }
                    this.#logger.info("Data in proposal matches Neutron data");
                }
                this.#logger.info("Voting on proposal ID: %s", proposal.safeTxHash);
                await this.confirmProposal(proposal.safeTxHash); 
            }
            const readyProposals = pendingProposals.results.filter(p => (p.to === this.#receiverAddress) && ((p.confirmations || []).length === p.confirmationsRequired));
            if (readyProposals.length > 0) {
                this.#logger.info("There are proposals ready to be executed.");
                const readyProposal = readyProposals[0];
                if (!readyProposal) return;
                this.#logger.info("Ready proposal ID: %s", readyProposal.safeTxHash);
                await this.executeProposal(readyProposal.safeTxHash);
            }

        } else {
            if (neutronData.ts > ethereumData.ts || neutronData.er !== ethereumData.er) {
                this.#logger.info("New data available. Neutron TS: %d, Ethereum TS: %d", neutronData.ts, ethereumData.ts);
                await this.submitProposal(neutronData.er, neutronData.ts);
            }
        }
        return;
    }

    async submitProposal(er: BigInt, ts: number) {
        this.#logger.info("Submitting proposal to update Receiver contract with er: %o, ts: %d", er, ts);
        if (!this.#ethClient) throw new Error("Ethereum client not initialized");
        if (!this.#privateKey) throw new Error("Private key not available");
        if (!this.#safeApi) throw new Error("Safe API not initialized");
        if (!this.#safeClient) throw new Error("Safe client not initialized");
        
        const safeTransactionData: MetaTransactionData = {
            to: this.#receiverAddress,
            value: "0",
            data: encodeFunctionData({
                abi: RECEIVER_ABI,
                functionName: 'publish',
                args: [er, BigInt(ts)],
            }),
            operation: OperationType.Call,
        };

        this.#logger.debug("Safe Transaction Data: %o", safeTransactionData);

        const safeTransaction = await this.#safeClient.createTransaction({transactions: [safeTransactionData]});
        this.#logger.trace("Safe Transaction: %o", safeTransaction);
        
        const safeTxHash = await this.#safeClient.getTransactionHash(safeTransaction);
        this.#logger.trace("Safe Transaction Hash: %s", safeTxHash);

        const senderSignature = await this.#safeClient.signHash(safeTxHash)
        this.#logger.trace("Sender Signature: %o", senderSignature);

        await this.#safeApi.proposeTransaction({
            safeAddress: this.#safeAddress,
            safeTransactionData: safeTransaction.data,
            safeTxHash,
            senderAddress: this.#account.address,
            senderSignature: senderSignature.data
        });
    }

    async confirmProposal(safeTxHash: string) {
        this.#logger.info("Confirming proposal with Safe Tx Hash: %s", safeTxHash);
        if (!this.#safeApi) throw new Error("Safe API not initialized");
        const client = await Safe.init({
            provider: this.#ethereumRpcUrl,
            signer: this.#privateKey,
            safeAddress: this.#safeAddress,
        });
        const signature = await client.signHash(safeTxHash)
        this.#logger.trace("Confirmation Signature: %o", signature);
        await this.#safeApi.confirmTransaction(safeTxHash, signature.data);
    }

    async executeProposal(safeTxHash: string) {
        this.#logger.info("Executing proposal with Safe Tx Hash: %s", safeTxHash);
        if (!this.#safeApi) throw new Error("Safe API not initialized");
        const client = await Safe.init({
            provider: this.#ethereumRpcUrl,
            signer: this.#privateKey,
            safeAddress: this.#safeAddress,
        });
        const safeTx = await this.#safeApi.getTransaction(safeTxHash);
        const executeTxResponse = await client.executeTransaction(safeTx);
        this.#logger.trace("Execute Transaction Response: %o", executeTxResponse);
    }

    async getPendingProposals() {
        if (!this.#safeApi) throw new Error("Safe API not initialized");
        const proposals = await this.#safeApi.getPendingTransactions(this.#safeAddress, {});
        return proposals;
    }

    async getTWAERData(): Promise<{er: BigInt, ts: number}> {
        this.#logger.info("Querying TWAER data from Neutron");
        if (!this.#cosmWasmClient) throw new Error("CosmWasmClient not initialized");
        const result = await this.#cosmWasmClient.queryContractSmart(this.#twaerContract, {
            "get_twaer": {}
        });
        this.#logger.trace("TWAER Query Result: %o", result);
        return {er: toBigIntTimes10Pow(result.twaer) , ts: result.published_at};
    }

    async getReceverData(): Promise<{er: BigInt, ts: number}> {
        this.#logger.info("Querying Receiver data from Ethereum");
        if (!this.#ethClient) throw new Error("Ethereum client not initialized");
        const [er, ts] = (await this.#ethClient.readContract({
            address: this.#receiverAddress as `0x${string}`,
            abi: RECEIVER_ABI,
            functionName: 'getLatest',
            args: [],
        })) as [BigInt, BigInt];
        this.#logger.trace("Receiver Query Result: %o", [er, ts]);
        return {er, ts: Number(ts) };
    }
}

(async () => {
    const bot = new Bot();
    await bot.init();
    await bot.tick();
})();