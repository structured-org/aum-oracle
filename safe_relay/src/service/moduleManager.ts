import pino from 'pino';
import Module from '../modules/index';
import {Config} from '../lib/config';
import RelayEthereum from "../modules/relay-neutron-ethereum";

export class ModuleManager {
    private readonly logger: pino.Logger;
    private readonly modules: Module[] = [];

    constructor(logger: pino.Logger) {
        this.logger = logger;
    }

    async registerModules(config: Config): Promise<void> {
        this.logger.info('Registering modules...');
        const relayEthereum = new RelayEthereum(
            this.logger.child({ctx: 'RelayEthereum'}),
            config.ethereum!,
            config.neutron!,
        );
        await relayEthereum.init();
        this.modules.push(relayEthereum);
        this.logger.info('RelayEthereum registered');
        this.logger.info(`Total modules registered: ${this.modules.length}`);
    }

    async runModules(): Promise<void> {
        for (const module of this.modules) {
            try {
                this.logger.info(`Running ${module.constructor.name} module...`);
                await module.tick();
            } catch (error) {
                console.log(error);
                this.logger.error(
                    {error: error.toString(), module: module.constructor.name},
                    'Error running module',
                );
            }
        }
    }
}