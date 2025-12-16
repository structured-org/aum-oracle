import dotenv from 'dotenv';
import pino from 'pino';
import {getLogger} from './lib/logger';
import {ServiceInitializer} from './service/serviceInitializer';
import {ModuleManager} from './service/moduleManager';
import {Config, getConfig} from './lib/config';

dotenv.config();

class Service {
    private readonly logger: pino.Logger;
    private config?: Config;
    private moduleManager?: ModuleManager;
    private workHandler?: NodeJS.Timeout;

    constructor() {
        this.logger = getLogger('debug');
    }

    async initialize() {
        const config = await getConfig(this.logger);
        const initializer = new ServiceInitializer(this.logger);
        this.moduleManager = await initializer.initialize();
        this.config = config;
    }

    start() {
        if (!this.config || !this.moduleManager) {
            throw new Error('Service not initialized');
        }
        this.logger.info('Starting coordinator service...');
        this.workHandler = setInterval(
            () => this.performWork(),
            this.config.serviceConfig!.callInterval * 1000,
        );
        this.logger.info('Coordinator service started successfully');
    }

    private async performWork(): Promise<void> {
        if (!this.config || !this.moduleManager) {
            return;
        }
        try {
            await this.moduleManager.runModules();
        } catch (error) {
            this.logger.error({error}, 'Error during work execution');
        }
    }
}

async function main() {
    const service = new Service();
    try {
        await service.initialize();
        service.start();
    } catch (error) {
        console.error('Failed to start service:', error);
        process.exit(1);
    }
}

main();