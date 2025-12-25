import pino from 'pino';
import Module from '../modules/index';
import { Config } from '../lib/config';
import RelayEthereum from '../modules/relay-neutron-ethereum';
import RelaySolana from '../modules/relay-neutron-solana';

export class ModuleManager {
  private readonly logger: pino.Logger;
  private readonly modules: Module[] = [];

  constructor(logger: pino.Logger) {
    this.logger = logger;
  }

  async registerModules(config: Config): Promise<void> {
    this.logger.info('Registering modules...');
    const modulesInits = [];
    if (config.ethereum?.enabled) {
      const relayEthereum = new RelayEthereum(
        this.logger.child({ ctx: 'RelayEthereum' }),
        config.ethereum!,
        config.neutron!,
      );
      modulesInits.push(relayEthereum.init());
      this.modules.push(relayEthereum);
      this.logger.info('RelayEthereum registered');
    }
    if (config.solana?.enabled) {
      const relaySolana = new RelaySolana(
        this.logger.child({ ctx: 'RelaySolana' }),
        config.solana!,
        config.neutron!,
      );
      modulesInits.push(relaySolana.init());
      this.modules.push(relaySolana);
      this.logger.info('RelaySolana registered');
    }
    this.logger.info(`Total modules registered: ${this.modules.length}`);
    await Promise.all(modulesInits);
  }

  async runModules(): Promise<void> {
    const moduleTicks = [];
    for (const module of this.modules) {
      try {
        this.logger.info(`Running ${module.constructor.name} module...`);
        moduleTicks.push(module.tick());
      } catch (error) {
        console.log(error);
        this.logger.error(
          { error: error, module: module.constructor.name },
          'Error running module',
        );
      }
    }
    await Promise.all(moduleTicks)
  }
}
