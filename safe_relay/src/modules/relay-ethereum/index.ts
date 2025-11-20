import Manager from "../index";
import {Logger} from "pino";
import {EthereumConfig, NeutronConfig} from "../../lib/config";

export default class RelayEthereum implements Manager {
    private logger: Logger;
    private ethereum: EthereumConfig;
    private neutron: NeutronConfig;

    constructor(logger: Logger, ethereumConfig: EthereumConfig, neutronConfig: NeutronConfig) {
        this.logger = logger;
        this.ethereum = ethereumConfig;
        this.neutron = neutronConfig;
    }


    async init(): Promise<void> {
        console.log("init()");
    }

    async tick(): Promise<void> {
        console.log("tick()");
    }
}