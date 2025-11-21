import Manager from "../index";
import {Logger} from "pino";
import {NeutronConfig, SolanaConfig} from "../../lib/config";
import {CosmWasmClient} from "@cosmjs/cosmwasm-stargate";
import SquadsMultisig, {SquadsMultisigConfig} from "../../lib/multisig/squads";
import {AnchorProvider, web3} from "@project-serum/anchor";
import {Keypair} from "@solana/web3.js";
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


export default class RelaySolana implements Manager {
    private logger: Logger;
    private solana: SolanaConfig;
    private neutron: NeutronConfig;

    private cosmWasmClient?: CosmWasmClient;
    private squadsMultisig?: SquadsMultisig;

    constructor(logger: Logger, solanaConfig: SolanaConfig, neutronConfig: NeutronConfig) {
        this.logger = logger;
        this.solana = solanaConfig;
        this.neutron = neutronConfig;
    }

    async init(): Promise<void> {
        /* Neutron */
        this.cosmWasmClient = await CosmWasmClient.connect(this.neutron.rpc);
        /* Solana */
        process.env.ANCHOR_WALLET = this.solana.seedPath;
        const squadsMultisigConfig: SquadsMultisigConfig = {
            anchorProvider: AnchorProvider.local(this.solana.rpc, {
                commitment: 'confirmed',
                skipPreflight: true,
            }),
            multisigAddress: new web3.PublicKey(this.solana.multisigAddress),
            vaultPda: new web3.PublicKey(this.solana.vaultPda),
            keypair: web3.Keypair.fromSecretKey(
                Buffer.from(
                    JSON.parse(
                        fs.readFileSync(this.solana.seedPath!, {
                            encoding: 'utf-8',
                        }),
                    ),
                ),
            )
        }
        process.env.ANCHOR_WALLET = undefined;
        this.squadsMultisig = new SquadsMultisig(
            this.logger,
            squadsMultisigConfig
        )
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