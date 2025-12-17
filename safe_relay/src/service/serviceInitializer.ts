import pino from 'pino';
import { getConfig } from '../lib/config';
import { ModuleManager } from './moduleManager';

export class ServiceInitializer {
  private readonly logger: pino.Logger;

  constructor(logger: pino.Logger) {
    this.logger = logger;
  }

  async initialize(): Promise<ModuleManager> {
    try {
      const config = await getConfig(this.logger);
      const moduleManager = new ModuleManager(this.logger);
      await moduleManager.registerModules(config);
      return moduleManager;
    } catch (error) {
      this.logger.error({ error }, 'Service initialization failed');
      throw error;
    }
  }
}