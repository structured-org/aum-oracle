import { z } from 'zod';

const safeRelayerEthereum = z.object({
  rpc: z.string(),
  binanceAum: z.string(),
});

const safeRelayerSolana = z.object({
  rpc: z.string(),
  oracleProgramId: z.string(),
  multisigAddress: z.string(),
  proposalDelay: z.number(), // Seconds
  vaultPda: z.string(),
  aumOracleSol: z.string(),
});

const safeRelayerServiceConfig = z.object({
  logLevel: z.string(),
  callInterval: z.number(), // Seconds
});

export const safeRelayerConfigSchema = z.object({
  serviceConfig: safeRelayerServiceConfig,
  ethereum: safeRelayerEthereum,
  solana: safeRelayerSolana,
});

export const envSchema = z.object({
  CONFIG_PATH: z.string(),
  ETHEREUM_MNEMONIC: z.string().optional(),
  SOLANA_SEED_PATH: z.string().optional(),
  SOLANA_MNEMONIC: z.string().optional(),
  SAFE_API_KEY: z.string().optional(),
});

type CommonConfigProperties = { mnemonic?: string; seed?: Uint8Array };
export type ServiceConfig = z.infer<typeof safeRelayerServiceConfig>;
export type SolanaConfig = z.infer<typeof safeRelayerSolana> &
  CommonConfigProperties;
export type EthereumConfig = z.infer<typeof safeRelayerEthereum>;

export interface ProcessedConfig {
  serviceConfig?: ServiceConfig;
  ethereum?: EthereumConfig;
  solana?: SolanaConfig;
}

export type TomlConfigData = z.infer<typeof safeRelayerConfigSchema>;
export type EnvData = z.infer<typeof envSchema>;
