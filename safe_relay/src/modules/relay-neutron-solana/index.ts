import Manager from "../index";
import {Logger} from "pino";
import {NeutronConfig, SolanaConfig} from "../../lib/config";
import {CosmWasmClient} from "@cosmjs/cosmwasm-stargate";

export default class RelaySolana implements Manager {
    private logger: Logger;
    private solana: SolanaConfig;
    private neutron: NeutronConfig;

    private cosmWasmClient?: CosmWasmClient;

    constructor(logger: Logger, solanaConfig: SolanaConfig, neutronConfig: NeutronConfig) {
        this.logger = logger;
        this.solana = solanaConfig;
        this.neutron = neutronConfig;
    }

    async init(): Promise<void> {
        /* Neutron */
        this.cosmWasmClient = await CosmWasmClient.connect(this.neutron.rpc);
    }

    async tick(): Promise<void> {
        console.log("tick()");
    }
}