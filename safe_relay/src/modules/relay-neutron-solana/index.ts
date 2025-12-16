import Manager from "../index";
import {Logger} from "pino";
import {NeutronConfig, SolanaConfig} from "../../lib/config";
import {CosmWasmClient} from "@cosmjs/cosmwasm-stargate";
import SquadsMultisig, {SquadsMultisigConfig} from "../../lib/multisig/squads";
import {AnchorProvider, Program, setProvider, Wallet, web3} from "@coral-xyz/anchor";
import * as fs from "node:fs";

type AumData = {
    last_published_data: {
        round: number;
        timestamp: number;
        data: {
            unimmr: string;
            positions: {
                symbol: string;
                amount: string;
                pnl: string;
            }[];
            um_balance_usdt: string;
            spot_balances: {
                asset: string;
                amount: string;
            }[];
            pm_account_actual_equity: string;
            withdrawable_usdt: string;
        };
    };
};

export const PROGRAM_ID = new web3.PublicKey("orac315UJ2aQXvgiWjoFsZZDzEEuPaozYLDHf6epJzd");

export default class RelaySolana implements Manager {
    private logger: Logger;
    private solana: SolanaConfig;
    private neutron: NeutronConfig;

    private cosmWasmClient?: CosmWasmClient;
    private squadsMultisig?: SquadsMultisig;
    private provider?: AnchorProvider;
    private aumOracleProgram?: Program;

    constructor(logger: Logger, solanaConfig: SolanaConfig, neutronConfig: NeutronConfig) {
        this.logger = logger;
        this.solana = solanaConfig;
        this.neutron = neutronConfig;
    }

    async init(): Promise<void> {
        /* Neutron */
        this.cosmWasmClient = await CosmWasmClient.connect(this.neutron.rpc);
        /* Solana */
        const keypair = web3.Keypair.fromSecretKey(Uint8Array.from(
            JSON.parse(fs.readFileSync(this.solana.seedPath!, 'utf-8')),
        ));
        this.provider = new AnchorProvider(new web3.Connection(this.solana.rpc, {
                commitment: 'confirmed',
            }),
            new Wallet(keypair),
            {
                commitment: 'confirmed',
                skipPreflight: true,
            }
        );
        /* Global provider allows work with functions like fetchIdl */
        setProvider(this.provider);

        const squadsMultisigConfig: SquadsMultisigConfig = {
            anchorProvider: this.provider,
            multisigAddress: new web3.PublicKey(this.solana.multisigAddress),
            vaultPda: new web3.PublicKey(this.solana.vaultPda),
            keypair: keypair
        };
        this.squadsMultisig = new SquadsMultisig(
            this.logger,
            squadsMultisigConfig
        );
        this.aumOracleProgram = new Program(
            await Program.fetchIdl(PROGRAM_ID, this.provider)
        );
    }

    async tick(): Promise<void> {
        console.log(await this.getAumData());
    }

    private async getAumData(): Promise<AumData> {
        const result = await this.cosmWasmClient?.queryContractSmart(this.neutron.binanceAum, {
            "get_data": {}
        });
        this.logger.trace("TWAER Query Result: %o", result);
        return result as AumData;
    }
}