import Manager from "../index";
import {Logger} from "pino";
import {EthereumConfig, NeutronConfig} from "../../lib/config";
import SafeMultisig, {SafeMultisigConfig} from "../../lib/multisig/safe";
import Safe from "@safe-global/protocol-kit";
import {mnemonicToAccount} from 'viem/accounts';
import SafeApiKit from "@safe-global/api-kit";

export default class RelayEthereum implements Manager {
    private readonly logger: Logger;
    private ethereum: EthereumConfig;
    private neutron: NeutronConfig;

    private safeMultisig: SafeMultisig;

    constructor(logger: Logger, ethereumConfig: EthereumConfig, neutronConfig: NeutronConfig) {
        this.logger = logger;
        this.ethereum = ethereumConfig;
        this.neutron = neutronConfig;
    }


    async init(): Promise<void> {
        const account = mnemonicToAccount(this.ethereum.mnemonic);
        const pk = account.getHdKey().privateKey;
        const safeConfig: SafeMultisigConfig = {
            safeClient: await Safe.init({
                provider: this.ethereum.rpc,
                signer: `0x${Buffer.from(pk).toString('hex')}`,
                safeAddress: this.ethereum.safeAddress,
            }),
            safeApi: new SafeApiKit({chainId: 1n, apiKey: this.ethereum.safeApiKey}),
            receiverAddress: this.ethereum.receiverAddress,
            signer: account
        };
        this.safeMultisig = new SafeMultisig(this.ethereum.safeAddress, this.logger, safeConfig);
    }

    async tick(): Promise<void> {
        console.log("tick()");
    }
}