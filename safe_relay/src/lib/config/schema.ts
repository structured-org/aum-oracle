import {z} from "zod";

const safeRelayerEthereum = z.object({
    rpc: z.string(),
    safeAddress: z.string(),
    receiverAddress: z.string(),
});

const safeRelayerNeutron = z.object({
    rpc: z.string(),
    twaerContract: z.string()
});

const safeRelayerServiceConfig = z.object({
    logLevel: z.string(),
    verifyValuesOnVote: z.boolean(),
    callInterval: z.number(), // Seconds
})

export const safeRelayerConfigSchema = z.object({
    serviceConfig: safeRelayerServiceConfig,
    ethereum: safeRelayerEthereum,
    neutron: safeRelayerNeutron
});

export const envSchema = z.object({
    CONFIG_PATH: z.string(),
    ETHEREUM_MNEMONIC: z.string().optional(),
    SAFE_API_KEY: z.string().optional(),
});

type CommonConfigProperties = { mnemonic: string };
export type ServiceConfig = z.infer<typeof safeRelayerServiceConfig>;
export type NeutronConfig = z.infer<typeof safeRelayerNeutron>;
export type EthereumConfig = z.infer<typeof safeRelayerEthereum> &
    CommonConfigProperties & { safeApiKey: string };

export interface ProcessedConfig {
    serviceConfig?: ServiceConfig;
    ethereum?: EthereumConfig;
    neutron?: NeutronConfig;
}

export type TomlConfigData = z.infer<typeof safeRelayerConfigSchema>;
export type EnvData = z.infer<typeof envSchema>;