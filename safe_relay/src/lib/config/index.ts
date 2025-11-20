import {z} from 'zod';
import pino, {type Logger} from 'pino';
import path from 'node:path';
import {
    safeRelayerConfigSchema,
    envSchema,
    type ProcessedConfig,
    type TomlConfigData,
    type EnvData,
} from './schema';
import {tomlAdapter} from "zod-config/toml-adapter";
import {loadConfig} from "zod-config";

export class Config {
    private readonly logger: pino.Logger;
    private tomlData: TomlConfigData | undefined;
    private processedConfig: ProcessedConfig | undefined;
    private envData: EnvData;

    constructor(logger: pino.Logger) {
        this.logger = logger.child({context: 'Config'});
        this.envData = this.validateEnvData();
        this.logger.info('Configuration loaded and validated successfully');
    }

    async init() {
        this.tomlData = await this.localTomlConfig(this.envData.CONFIG_PATH);
        this.processedConfig = this.createProcessedConfig();
    }

    private validateEnvData(): EnvData {
        let envSchemaParsed: EnvData;
        try {
            envSchemaParsed = envSchema.parse(process.env);
        } catch (error) {
            if (error instanceof z.ZodError) {
                const errorMessages = error.issues.map(
                    (issue) => `${issue.path.join('.')}: ${issue.message}`,
                );
                this.logger.error(
                    {errors: errorMessages},
                    'Environment variables validation failed',
                );
                throw new Error(
                    `Environment variables validation failed:\n${errorMessages.join('\n')}`,
                );
            }
            throw error;
        }
        return envSchemaParsed;
    }

    private async localTomlConfig(configPath: string): Promise<TomlConfigData> {
        try {
            const fullPath = path.resolve(__dirname, configPath);
            return await loadConfig({
                schema: safeRelayerConfigSchema,
                adapters: tomlAdapter({path: fullPath})
            });
        } catch (error) {
            if (error instanceof z.ZodError) {
                const errorMessages = error.issues.map(
                    (issue) => `${issue.path.join('.')}: ${issue.message}`,
                );
                this.logger.error(
                    {errors: errorMessages},
                    'TOML configuration validation failed',
                );
                throw new Error(
                    `TOML configuration validation failed:\n${errorMessages.join('\n')}`,
                );
            }

            this.logger.error(
                {error, path: configPath},
                'Failed to load TOML configuration',
            );
            throw new Error(
                `Failed to load TOML configuration from ${configPath}: ${error}`,
            );
        }
    }

    private createProcessedConfig(): ProcessedConfig {
        const config: ProcessedConfig = {
            serviceConfig: this.tomlData?.serviceConfig
        };

        {
            if (!this.envData.ETHEREUM_MNEMONIC) {
                throw new Error(
                    'ETHEREUM_MNEMONIC is required when EthereumClaim module is enabled',
                );
            }
            if (!this.envData.SAFE_API_KEY) {
                throw new Error(
                    'SAFE_API_KEY is required when EthereumClaim module is enabled',
                );
            }
            config.ethereum = {
                mnemonic: this.envData.ETHEREUM_MNEMONIC,
                safeAddress: this.tomlData?.ethereum.safeAddress!,
                safeApiKey: this.envData.SAFE_API_KEY,
                receiverAddress: this.tomlData?.ethereum.receiverAddress!,
                rpc: this.tomlData?.ethereum.rpc!,
            };
        }
        {
            config.neutron = {
                twaerContract: this.tomlData?.neutron.twaerContract!,
                rpc: this.tomlData?.ethereum.rpc!,
            };
        }

        return config;
    }

    get serviceConfig() {
        return this.tomlData?.serviceConfig;
    }

    get ethereum() {
        return this.tomlData?.ethereum;
    }

    get neutron() {
        return this.tomlData?.neutron;
    }
}

let configInstance: Config | null = null;
export const getConfig = (logger: Logger): Config => {
    if (!configInstance) {
        configInstance = new Config(logger);
    }
    return configInstance;
};

export * from './schema';