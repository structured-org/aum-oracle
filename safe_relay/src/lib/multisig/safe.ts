import type Multisig from "./index.ts";
import SafeApiKit from "@safe-global/api-kit";
import type {Logger} from "pino";
import Safe from "@safe-global/protocol-kit";
import {type MetaTransactionData, OperationType} from "@safe-global/types-kit";
import {encodeFunctionData} from "viem";
import RECEIVER_ABI from "../../generic/Receiver.abi.json";
import type {HDAccount} from "viem/accounts";

export type SafeMultisigConfig = {
    safeClient: Safe;
    safeApi: SafeApiKit;
    receiverAddress: string;
    signer: HDAccount;
}

export default class SafeMultisig implements Multisig {
    multisigAddress: string;
    private config: SafeMultisigConfig;
    private logger: Logger;

    constructor(multisigAddress: string, logger: Logger, config: SafeMultisigConfig) {
        this.multisigAddress = multisigAddress;
        this.config = config;
        this.logger = logger;
    }

    async submitProposal(payload: Buffer, timestamp: number): Promise<string | null> {
        const er = (payload.readBigInt64BE());
        this.logger.info(`exchangeRate -- ${er}`);
        this.logger.info(`timestamp -- ${timestamp}`);

        const safeTransactionData: MetaTransactionData = {
            to: this.config.receiverAddress,
            value: "0",
            data: encodeFunctionData({
                abi: RECEIVER_ABI,
                functionName: 'publish',
                args: [payload.readBigInt64BE(), BigInt(timestamp)],
            }),
            operation: OperationType.Call,
        };

        this.logger.debug("Safe Transaction Data: %o", safeTransactionData);

        const safeTransaction = await this.config.safeClient.createTransaction({transactions: [safeTransactionData]});
        this.logger.trace("Safe Transaction: %o", safeTransaction);

        const safeTxHash = await this.config.safeClient.getTransactionHash(safeTransaction);
        this.logger.trace("Safe Transaction Hash: %s", safeTxHash);

        const senderSignature = await this.config.safeClient.signHash(safeTxHash)
        this.logger.trace("Sender Signature: %o", senderSignature);

        await this.config.safeApi.proposeTransaction({
            safeAddress: this.multisigAddress,
            safeTransactionData: safeTransaction.data,
            safeTxHash,
            senderAddress: this.config.signer.address,
            senderSignature: senderSignature.data
        });
        return null;
    }

    async executeProposal(id: string): Promise<string> {
        /* Safe id is a string, we assume that the caller should provide the string */
        const identifier = String(id);
        this.logger.info(`txHash -- ${identifier}`);

        const safeTx = await this.config.safeApi.getTransaction(identifier);
        const executeTxResponse = await this.config.safeClient.executeTransaction(safeTx);
        this.logger.trace("Execute Transaction Response: %o", executeTxResponse);
        return executeTxResponse.hash;
    }

    async getPendingProposals() {
        return await this.config.safeApi.getPendingTransactions(this.multisigAddress, {});
    }

    private async confirmProposal(safeTxHash: string) {
        this.logger.info("Confirming proposal with Safe Tx Hash: %s", safeTxHash);
        const signature = await this.config.safeClient.signHash(safeTxHash);

        this.logger.trace("Confirmation Signature: %o", signature);
        await this.config.safeApi.confirmTransaction(safeTxHash, signature.data);
    }
}