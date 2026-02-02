import { z } from 'zod';
import pino, { type Logger } from 'pino';
import path from 'node:path';
import {
  safeRelayerConfigSchema,
  envSchema,
  type ProcessedConfig,
  type TomlConfigData,
  type EnvData,
} from './schema';
import { tomlAdapter } from 'zod-config/toml-adapter';
import { loadConfig } from 'zod-config';
import fs from "node:fs";

export class Config {
  private readonly logger: pino.Logger;
  private tomlData: TomlConfigData | undefined;
  private processedConfig: ProcessedConfig | undefined;
  private envData: EnvData;

  constructor(logger: pino.Logger) {
    this.logger = logger.child({ context: 'Config' });
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
          { errors: errorMessages },
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
        adapters: tomlAdapter({ path: fullPath }),
      });
    } catch (error) {
      if (error instanceof z.ZodError) {
        const errorMessages = error.issues.map(
          (issue) => `${issue.path.join('.')}: ${issue.message}`,
        );
        this.logger.error(
          { errors: errorMessages },
          'TOML configuration validation failed',
        );
        throw new Error(
          `TOML configuration validation failed:\n${errorMessages.join('\n')}`,
        );
      }

      this.logger.error(
        { error, path: configPath },
        'Failed to load TOML configuration',
      );
      throw new Error(
        `Failed to load TOML configuration from ${configPath}: ${error}`,
      );
    }
  }

  private createProcessedConfig(): ProcessedConfig {
    const config: ProcessedConfig = {
      serviceConfig: this.tomlData?.serviceConfig,
    };
    config.ethereum = {
      rpc: this.tomlData?.ethereum.rpc!,
      binanceAum: this.tomlData?.ethereum.binanceAum!,
    };
    if (!this.envData.SOLANA_SEED_PATH && !this.envData.SOLANA_MNEMONIC) {
      throw new Error(
        'SOLANA_SEED_PATH or SOLANA_MNEMONIC is required',
      );
    }
    config.solana = {
      rpc: this.tomlData?.solana.rpc!,
      proposalDelay: this.tomlData?.solana.proposalDelay!,
      oracleProgramId: this.tomlData?.solana.oracleProgramId!,
      multisigAddress: this.tomlData?.solana.multisigAddress!,
      vaultPda: this.tomlData?.solana.vaultPda!,
      aumOracleSol: this.tomlData?.solana.aumOracleSol!,
      seed: Uint8Array.from(
        JSON.parse(fs.readFileSync(this.envData.SOLANA_SEED_PATH!, 'utf-8')),
      ),
      mnemonic: this.envData.SOLANA_MNEMONIC,
    };

    return config;
  }

  get serviceConfig() {
    return this.processedConfig?.serviceConfig;
  }

  get ethereum() {
    return this.processedConfig?.ethereum;
  }

  get solana() {
    return this.processedConfig?.solana;
  }
}

let configInstance: Config | null = null;
export const getConfig = async (logger: Logger): Promise<Config> => {
  if (!configInstance) {
    configInstance = new Config(logger);
    await configInstance.init();
  }
  return configInstance;
};

export * from './schema';
