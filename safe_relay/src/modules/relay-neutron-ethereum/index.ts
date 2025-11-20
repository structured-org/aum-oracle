import Manager from "../index";
import {Logger} from "pino";
import {EthereumConfig, NeutronConfig} from "../../lib/config";
import SafeMultisig, {SafeMultisigConfig} from "../../lib/multisig/safe";
import Safe from "@safe-global/protocol-kit";
import {mnemonicToAccount} from 'viem/accounts';
import SafeApiKit from "@safe-global/api-kit";
import {CosmWasmClient} from "@cosmjs/cosmwasm-stargate";
import {createPublicClient, http, PublicClient} from "viem";
import {mainnet} from 'viem/chains';

import RECEIVER_ABI from "../../generic/Receiver.abi.json";

export default class RelayEthereum implements Manager {
    private readonly logger: Logger;
    private ethereum: EthereumConfig;
    private neutron: NeutronConfig;

    private cosmWasmClient?: CosmWasmClient;
    private ethClient?: PublicClient;
    private safeMultisig?: SafeMultisig;

    constructor(logger: Logger, ethereumConfig: EthereumConfig, neutronConfig: NeutronConfig) {
        this.logger = logger;
        this.ethereum = ethereumConfig;
        this.neutron = neutronConfig;
    }

    async init(): Promise<void> {
        /* Neutron */
        this.cosmWasmClient = await CosmWasmClient.connect(this.neutron.rpc);
        /* Ethereum */
        const account = mnemonicToAccount(this.ethereum.mnemonic);
        const pk = account.getHdKey().privateKey;
        const safeConfig: SafeMultisigConfig = {
            safeClient: await Safe.init({
                provider: this.ethereum.rpc,
                signer: `0x${Buffer.from(pk!).toString('hex')}`,
                safeAddress: this.ethereum.safeAddress,
            }),
            safeApi: new SafeApiKit({chainId: 1n, apiKey: this.ethereum.safeApiKey}),
            receiverAddress: this.ethereum.receiverAddress,
            signer: account
        };
        this.safeMultisig = new SafeMultisig(this.ethereum.safeAddress, this.logger, safeConfig);
        this.ethClient = createPublicClient({
            chain: mainnet,
            transport: http(this.ethereum.rpc),
        }) as PublicClient;
    }

    async tick(): Promise<void> {
    }

    private async getTWAERData(): Promise<{ er: BigInt, ts: number }> {
        this.logger.info("Querying TWAER data from Neutron");
        const result = await this.cosmWasmClient?.queryContractSmart(this.neutron.twaerContract, {
            "get_twaer": {}
        });
        this.logger.trace("TWAER Query Result: %o", result);
        return {er: this.toBigIntTimes10Pow(result.twaer), ts: result.published_at};
    }

    private async getReceverData(): Promise<{ er: BigInt, ts: number }> {
        this.logger.info("Querying Receiver data from Ethereum");
        const [er, ts] = (await this.ethClient?.readContract({
            address: this.ethereum.receiverAddress as `0x${string}`,
            abi: RECEIVER_ABI,
            functionName: 'getLatest',
            args: [],
            authorizationList: undefined
        })) as [BigInt, BigInt];
        this.logger.trace("Receiver Query Result: %o", [er, ts]);
        return {er, ts: Number(ts)};
    }

    private toBigIntTimes10Pow(str: string, decimals = 18): BigInt {
        str = str.trim();
        if (str.length === 0) {
            return 0n;
        }

        const [intPart = "", fracPart = ""] = str.split(".");
        const combined = intPart + fracPart.padEnd(decimals, "0");
        const normalized = combined.slice(0, intPart.length + decimals);
        return BigInt(normalized);
    }
}