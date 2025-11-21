import {z} from "zod";

const safeRelayerEthereum = z.object({
    rpc: z.string(),
    enabled: z.boolean(),
    safeAddress: z.string(),
    proposalDelay: z.number(), // Seconds
    receiverAddress: z.string(),
    verifyValuesOnVote: z.boolean(),
});

const safeRelayerNeutron = z.object({
    rpc: z.string(),
    twaerContract: z.string(),
    binanceAum: z.string()
});

const safeRelayerSolana = z.object({
    rpc: z.string(),
    enabled: z.boolean(),
    multisigAddress: z.string(),
    proposalDelay: z.number(), // Seconds
    vaultPda: z.string(),
});

const safeRelayerServiceConfig = z.object({
    logLevel: z.string(),
    callInterval: z.number(), // Seconds
})

export const safeRelayerConfigSchema = z.object({
    serviceConfig: safeRelayerServiceConfig,
    ethereum: safeRelayerEthereum,
    neutron: safeRelayerNeutron,
    solana: safeRelayerSolana
});

export const envSchema = z.object({
    CONFIG_PATH: z.string(),
    ETHEREUM_MNEMONIC: z.string().optional(),
    SOLANA_SEED_PATH: z.string().optional(),
    SAFE_API_KEY: z.string().optional(),
});

type CommonConfigProperties = { mnemonic?: string, seedPath?: string };
export type ServiceConfig = z.infer<typeof safeRelayerServiceConfig>;
export type NeutronConfig = z.infer<typeof safeRelayerNeutron>;
export type SolanaConfig = z.infer<typeof safeRelayerSolana> & CommonConfigProperties;
export type EthereumConfig = z.infer<typeof safeRelayerEthereum> &
    CommonConfigProperties & { safeApiKey: string };

export interface ProcessedConfig {
    serviceConfig?: ServiceConfig;
    ethereum?: EthereumConfig;
    neutron?: NeutronConfig;
    solana?: SolanaConfig;
}

export type TomlConfigData = z.infer<typeof safeRelayerConfigSchema>;
export type EnvData = z.infer<typeof envSchema>;