import Manager from '../index';
import { Logger } from 'pino';
import { EthereumConfig, SolanaConfig } from '../../lib/config';
import { createPublicClient, http, PublicClient, decodeAbiParameters } from 'viem';
import { mainnet } from 'viem/chains';
import SquadsMultisig, {
    SquadsMultisigConfig,
} from '../../lib/multisig/squads';
import {
    AnchorProvider,
    BN,
    Program,
    setProvider,
    Wallet,
    web3,
} from '@coral-xyz/anchor';
import {
    Ms,
    proposalStatusToString,
    VaultTransaction,
} from '../../lib/multisig/squads/internal';
import { mnemonicToSeedSync, validateMnemonic } from 'bip39';
import { derivePath } from 'ed25519-hd-key';

import { AUM_DATA } from '../../generic/BinanceAumReceiverAumData';
import BINANCE_AUM_ABI from '../../generic/BinanceAumReceiver.abi.json';

type FixedDecimal = {
    number: BN;
    decimals: number;
};

function toFixedDecimal(number: bigint): FixedDecimal {
    return {
        number: new BN(String(number).replace(/\./g, '')),
        decimals: String(number).split('.')[1]?.length ?? 0,
    };
}

function fromFixedDecimal(number: FixedDecimal): bigint {
    return BigInt(number.number.toString(10)) * (10n ** BigInt(number.decimals));
}

type AumOracleState = {
    lastPublishedData: AumDataSolana;
    solanaPublicationTimestamp: BN;
};

type AumDataSolana = {
    round: BN;
    timestamp: BN;
    data: {
        unimmr: FixedDecimal;
        positions: {
            symbol: string;
            amount: FixedDecimal;
            pnl: FixedDecimal;
        }[];
        umBalanceUsdt: FixedDecimal;
        spotBalances: {
            asset: string;
            amount: FixedDecimal;
        }[];
        pmAccountActualEquity: FixedDecimal;
        withdrawableUsdt: FixedDecimal;
    };
};

type AumDataEthereum = {
    round: bigint;
    timestamp: bigint;
    data: {
        unimmr: bigint;
        positions: {
            symbol: string;
            amount: bigint;
            pnl: bigint;
        }[];
        umBalanceUsdt: bigint;
        spotBalances: {
            asset: string;
            amount: bigint;
        }[];
        pmAccountActualEquity: bigint;
        withdrawableUsdt: bigint;
    };
};

function keypairFromMnemonic(
    mnemonic: string,
    derivationPath = "m/44'/501'/0'/0'",
): web3.Keypair {
    if (!validateMnemonic(mnemonic)) {
        throw new Error('Invalid mnemonic');
    }
    const seed = mnemonicToSeedSync(mnemonic);
    const derived = derivePath(derivationPath, seed.toString('hex'));
    const derivedKey = derived.key;
    if (derivedKey.length !== 32) {
        throw new Error('Derived key is not 32 bytes');
    }
    return web3.Keypair.fromSeed(derivedKey);
}

function stringFromBytes32(input: `0x${string}`): string {
    return input.slice(2).match(/.{1,2}/g)?.filter((v) => v != '00').map((v) => String.fromCharCode(parseInt(v, 16))).join('')!;
}

export default class RelaySolana implements Manager {
    private logger: Logger;
    private ethereum: EthereumConfig;
    private solana: SolanaConfig;

    private ethClient?: PublicClient;
    private signer?: web3.Keypair;
    private provider?: AnchorProvider;
    private squadsMultisig?: SquadsMultisig;
    private aumOracleProgram?: Program;
    private msParticipantIndex?: number;

    constructor(
        logger: Logger,
        ethereumConfig: EthereumConfig,
        solanaConfig: SolanaConfig,
    ) {
        this.logger = logger;
        this.ethereum = ethereumConfig;
        this.solana = solanaConfig;
    }

    async init(): Promise<void> {
        this.logger.info(
            `RelaySolana proposal delay -- ${this.solana.proposalDelay}`,
        );

        /* Ethereum */
        this.ethClient = createPublicClient({
            chain: mainnet,
            transport: http(this.ethereum.rpc),
        }) as PublicClient;

        /* Solana */
        if (this.solana.seed) {
            this.signer = web3.Keypair.fromSecretKey(this.solana.seed);
        } else if (this.solana.mnemonic) {
            this.signer = keypairFromMnemonic(this.solana.mnemonic!);
        } else {
            throw new Error('Neither seedPath nor mnemonic were provided');
        }
        this.provider = new AnchorProvider(
            new web3.Connection(this.solana.rpc, {
                commitment: 'confirmed',
            }),
            new Wallet(this.signer),
            {
                commitment: 'confirmed',
                skipPreflight: true,
            },
        );
        setProvider(this.provider);
        const squadsMultisigConfig: SquadsMultisigConfig = {
            anchorProvider: this.provider,
            multisigAddress: new web3.PublicKey(this.solana.multisigAddress),
            vaultPda: new web3.PublicKey(this.solana.vaultPda),
            keypair: this.signer,
        };
        this.squadsMultisig = new SquadsMultisig(this.logger, squadsMultisigConfig);
        this.aumOracleProgram = new Program(
            await Program.fetchIdl(this.solana.oracleProgramId, this.provider),
        );
        const msData = await this.provider.connection.getAccountInfo(
            new web3.PublicKey(this.solana.multisigAddress),
        );
        const ms = Ms.deserialize(msData?.data!);
        this.msParticipantIndex = ms.members!.findIndex(
            (member) => member.key.toBase58() === this.signer?.publicKey.toBase58(),
        );
    }

    async createProposal() {
        const aumEthereumData = await this.getEthereumAumData();
        const aumSolanaData = await this.getSolanaAumData();

        /* Check the freshness and if the new state needs to be submitted, propose */
        if (
            aumSolanaData.lastPublishedData.timestamp.toString() !==
            aumEthereumData.timestamp.toString()
        ) {
            const ix = await this.publishDataIx(aumEthereumData);
            const txhash = await this.squadsMultisig?.submitProposal(ix);
            this.logger.info(`Submited new proposal -- ${txhash}`);
        }
    }

    async tick(): Promise<void> {
        await new Promise((resolve) =>
            setTimeout(resolve, this.solana.proposalDelay * this.msParticipantIndex! * 1000),
        );

        /* Get all the proposals that are either Active of Approved */
        const proposals = (await this.squadsMultisig?.getPendingProposals())!;
        if (proposals.length) {
            const proposal = proposals![0];
            const proposalStatus = proposalStatusToString(proposal.status!);

            /* If it is Approved, execute */
            if (proposalStatus === 'Approved') {
                try {
                    const txhash = await this.squadsMultisig?.executeProposal(
                        Number(proposal.transactionIndex?.toString())!,
                    );
                    this.logger.info(
                        `Executed proposal ${proposal.transactionIndex} -- ${txhash}`,
                    );
                } catch (e: any) {
                    // TODO: error message has been changed to `InvalidTimestamp` in the upstream Solana program.
                    //       once we deploy it, we will have to update the following regexp here.
                    if (/SameTimestamp/.test(e.message.toString())) {
                        this.logger.warn(
                            `Outdated timestamp proposal -- ${proposal.transactionIndex}`,
                        );
                        await this.createProposal();
                    } else {
                        this.logger.warn(
                            `Unknown error happened -- ${proposal.transactionIndex}`,
                        );
                    }
                }
            }

            /* If it is Active, then if we have not voted yet, vote */
            if (
                proposalStatus === 'Active' &&
                !proposal
                    .approved!.map((approved) => approved.toBase58())
                    .includes(this.signer?.publicKey.toBase58()!)
            ) {
                const batchIxs = await this.squadsMultisig?.getBatchIxs(
                    Number(proposal.transactionIndex!.toString()),
                );
                if (!await this.validateIxs(batchIxs!)) {
                    this.logger.warn(`Invalid proposal -- ${proposal.transactionIndex}`);
                    return;
                }

                const txhash = await this.squadsMultisig?.voteProposal(
                    Number(proposal.transactionIndex?.toString())!,
                );
                this.logger.info(
                    `Voted for proposal ${proposal.transactionIndex} -- ${txhash}`,
                );
            }
        } else {
            await this.createProposal();
        }
    }

    private async validateIxs(ixs: Array<VaultTransaction>): Promise<boolean> {
        const DISCRIMINATOR = "230,18,158,253,73,167,115,188";
        const aumEthereumData = await this.getEthereumAumData();
        return ixs.entries().every(([, transaction]) =>
            transaction.message?.instructions.every(instruction => {
                const buffer = Array.from(instruction.data);
                const publishParams = this.aumOracleProgram?.coder.types.decode('publishDataParams', Buffer.from(buffer).subarray(8));
                const aumSolanaData = publishParams.data;

                if (aumSolanaData.round.toNumber() != aumEthereumData.round
                    || aumSolanaData.timestamp.toNumber() != aumEthereumData.timestamp
                    || fromFixedDecimal(aumSolanaData.data.unimmr) != aumEthereumData.data.unimmr
                    || aumSolanaData.data.positions.length != aumEthereumData.data.positions.length
                    || fromFixedDecimal(aumSolanaData.data.umBalanceUsdt) != aumEthereumData.data.umBalanceUsdt
                    || aumSolanaData.data.spotBalances.length != aumEthereumData.data.spotBalances.length
                    || fromFixedDecimal(aumSolanaData.data.pmAccountActualEquity) != aumEthereumData.data.pmAccountActualEquity
                    || fromFixedDecimal(aumSolanaData.data.withdrawableUsdt) != aumEthereumData.data.withdrawableUsdt) {
                    return false;
                }

                for (let i in (aumSolanaData.data.positions as Array<any>)) {
                    if (aumSolanaData.data.positions[i].symbol != aumEthereumData.data.positions[i].symbol
                        || fromFixedDecimal(aumSolanaData.data.positions[i].amount) != aumEthereumData.data.positions[i].amount
                        || fromFixedDecimal(aumSolanaData.data.positions[i].pnl) != aumEthereumData.data.positions[i].pnl) {
                        return false;
                    }
                }

                for (let i in (aumSolanaData.data.spotBalances as Array<any>)) {
                    if (aumSolanaData.data.spotBalances[i].asset != aumEthereumData.data.spotBalances[i].asset
                        || fromFixedDecimal(aumSolanaData.data.spotBalances[i].amount) != aumEthereumData.data.spotBalances[i].amount) {
                        return false;
                    }
                }

                return Array.from(instruction.data.subarray(0, 8)).join(",") === DISCRIMINATOR;
            })
        );
    }

    private publishDataIx(
        dataEthereum: AumDataEthereum,
    ): Promise<web3.TransactionInstruction> {
        const dataSolana: AumDataSolana = {
            timestamp: new BN(dataEthereum.timestamp),
            round: new BN(dataEthereum.round),
            data: {
                pmAccountActualEquity: toFixedDecimal(
                    dataEthereum.data
                        .pmAccountActualEquity,
                ),
                unimmr: toFixedDecimal(
                    dataEthereum.data.unimmr,
                ),
                umBalanceUsdt: toFixedDecimal(
                    dataEthereum.data.umBalanceUsdt,
                ),
                withdrawableUsdt: toFixedDecimal(
                    dataEthereum.data.withdrawableUsdt,
                ),
                positions: dataEthereum.data.positions.map((e) => ({
                    pnl: toFixedDecimal(e.pnl),
                    amount: toFixedDecimal(e.amount),
                    symbol: e.symbol,
                })),
                spotBalances: dataEthereum.data.spotBalances.map(
                    (e) => ({
                        amount: toFixedDecimal(e.amount),
                        asset: e.asset,
                    }),
                ),
            },
        };
        // TODO: this method has been changed in the upstream aum-oracle Solana program.
        //       we will have to update it once we deploy a fresh version of the program.
        return this.aumOracleProgram!.methods.publishData({
            data: dataSolana,
        })
            .accounts({
                signer: this.solana.vaultPda,
                aumOracleConfig: new web3.PublicKey(this.solana.aumOracleSol),
            })
            .instruction()!;
    }

    private async getSolanaAumData(): Promise<AumOracleState> {
        const aumOracleConfig = new web3.PublicKey(this.solana.aumOracleSol);
        const configData =
            await this.provider?.connection.getAccountInfo(aumOracleConfig);
        const config = this.aumOracleProgram?.coder.accounts.decode(
            'aumOracleConfig',
            configData!.data,
        );
        const aumOracleState = web3.PublicKey.findProgramAddressSync(
            [Buffer.from('state'), config.instanceKey.toBuffer()],
            new web3.PublicKey(this.solana.oracleProgramId),
        )[0];
        const stateData =
            await this.provider?.connection.getAccountInfo(aumOracleState);
        const state = this.aumOracleProgram?.coder.accounts.decode(
            'aumOracleState',
            stateData!.data,
        );
        return state as AumOracleState;
    }

    private async getEthereumAumData(): Promise<AumDataEthereum> {
        const [{ round, timestamp, data }, success] = (await this.ethClient?.readContract({
            address: this.ethereum.binanceAum as `0x${string}`,
            abi: BINANCE_AUM_ABI,
            functionName: 'getData',
            args: [],
            authorizationList: undefined,
        })) as [{ round: bigint, timestamp: bigint, data: `0x${string}` }, boolean];
        if (!success) {
            throw new Error("AUM data is not yet published");
        }
        this.logger.trace('TWAER Query Result: %s %s %s', round, timestamp, data);

        const decodedData = decodeAbiParameters(AUM_DATA, data)[0] as any;

        const aumDataEthereum = {
            round,
            timestamp,
            data: {
                unimmr: decodedData.unimmr,
                positions: decodedData.positions.map((position: any) => {
                    return {
                        symbol: stringFromBytes32(position.symbol),
                        amount: position.amount,
                        pnl: position.pnl
                    };
                }),
                umBalanceUsdt: decodedData.umBalanceUsdt,
                spotBalances: decodedData.spotBalances.map((spotBalance: any) => {
                    return {
                        asset: stringFromBytes32(spotBalance.asset),
                        amount: spotBalance.amount,
                    };
                }),
                pmAccountActualEquity: decodedData.pmAccountActualEquity,
                withdrawableUsdt: decodedData.withdrawableUsdt,
            }
        }
        return aumDataEthereum;
    }
}
