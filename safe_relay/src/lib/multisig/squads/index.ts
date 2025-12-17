import type Multisig from '../index';
import { AnchorProvider, web3 } from '@project-serum/anchor';
import * as multisig from '@sqds/multisig';
// @ts-ignore
import { Multisig as SquadsMultisigGenerated } from '@sqds/multisig/lib/generated';
import { Logger } from 'pino';
import { AddressLookupTableAccount } from '@solana/web3.js';
import { compileToWrappedMessageV0, createBatchAddTransactionInstruction, transactionMessageBeet } from './internal';

export type SquadsMultisigConfig = {
  anchorProvider: AnchorProvider;
  multisigAddress: web3.PublicKey;
  keypair: web3.Keypair;
  vaultPda: web3.PublicKey;
}

export default class SquadsMultisig implements Multisig {
  private logger: Logger;
  private squadsMultisigApp: SquadsMultisigConfig;

  constructor(logger: Logger, squadsMultisigConfig: SquadsMultisigConfig) {
    this.logger = logger;
    this.squadsMultisigApp = squadsMultisigConfig;
  }

  async submitProposal(ix: web3.TransactionInstruction): Promise<string | null> {
    const createBatchIx = await this.createBatchIx();
    const createProposalIx = await this.createProposalIx();
    const addInstructionIx = await this.batchAddIxV0(ix);
    const proposalActivateIx = await this.proposalActivateIx();
    const proposalApproveIx = await this.proposalApproveIx();
    const tx = new web3.Transaction().add(
      createBatchIx,
      createProposalIx,
      addInstructionIx,
      proposalActivateIx,
      proposalApproveIx,
    );
    return await this.squadsMultisigApp.anchorProvider.sendAndConfirm(tx, [this.squadsMultisigApp.keypair]);
  }

  async executeProposal(id: number): Promise<string> {
    return '';
  }

  async getPendingProposals(): Promise<any> {
    return [];
  }

  private async getMultisigInfo(): Promise<SquadsMultisigGenerated> {
    return await multisig.accounts.Multisig.fromAccountAddress(
      this.squadsMultisigApp.anchorProvider.connection,
      this.squadsMultisigApp.multisigAddress,
    );
  }

  private async batchAddIxV0(
    instruction: web3.TransactionInstruction,
    altData?: AddressLookupTableAccount,
  ): Promise<web3.TransactionInstruction> {
    const multisigInfo = await this.getMultisigInfo();
    const transactionIndex = Number(multisigInfo.transactionIndex) + 1;
    return this.batchAddByIndexIxV0(transactionIndex, 1, instruction, altData);
  }

  private async batchAddByIndexIxV0(
    index: number,
    proposalInstructionIndex: number,
    instruction: web3.TransactionInstruction,
    altData?: AddressLookupTableAccount,
  ): Promise<web3.TransactionInstruction> {
    const [proposalPda] = multisig.getProposalPda({
      multisigPda: this.squadsMultisigApp.multisigAddress,
      transactionIndex: BigInt(index),
      programId: multisig.PROGRAM_ID,
    });
    const [batchPda] = multisig.getTransactionPda({
      multisigPda: this.squadsMultisigApp.multisigAddress,
      index: BigInt(index),
      programId: multisig.PROGRAM_ID,
    });
    const [batchTransactionPda] = multisig.getBatchTransactionPda({
      multisigPda: this.squadsMultisigApp.multisigAddress,
      batchIndex: BigInt(index),
      transactionIndex: proposalInstructionIndex,
      programId: multisig.PROGRAM_ID,
    });
    const compiledMessage = compileToWrappedMessageV0({
      payerKey: this.squadsMultisigApp.vaultPda,
      recentBlockhash: (
        await this.squadsMultisigApp.anchorProvider.connection.getLatestBlockhash()
      ).blockhash,
      instructions: [instruction],
      addressLookupTableAccounts: altData ? [altData] : undefined,
    });
    const [transactionMessageBytes] = transactionMessageBeet.serialize({
      numSigners: compiledMessage.header.numRequiredSignatures,
      numWritableSigners:
        compiledMessage.header.numRequiredSignatures -
        compiledMessage.header.numReadonlySignedAccounts,
      numWritableNonSigners:
        compiledMessage.staticAccountKeys.length -
        compiledMessage.header.numRequiredSignatures -
        compiledMessage.header.numReadonlyUnsignedAccounts,
      accountKeys: compiledMessage.staticAccountKeys,
      instructions: compiledMessage.compiledInstructions.map((ix) => ({
        programIdIndex: ix.programIdIndex,
        accountIndexes: ix.accountKeyIndexes,
        data: Array.from(ix.data),
      })),
      addressTableLookups: compiledMessage.addressTableLookups,
    });
    this.logger.info(`Alt Data Defined -- ${altData ? true : false}`);
    this.logger.info(`Batch PDA -- ${batchPda.toBase58()}`);
    this.logger.info(`Proposal PDA -- ${proposalPda.toBase58()}`);
    this.logger.info(`Transaction PDA -- ${batchTransactionPda.toBase58()}`);
    this.logger.info(`Transaction Index -- ${index}`);
    return createBatchAddTransactionInstruction(
      {
        multisig: this.squadsMultisigApp.multisigAddress,
        member: this.squadsMultisigApp.keypair.publicKey,
        proposal: proposalPda,
        rentPayer: this.squadsMultisigApp.keypair.publicKey,
        batch: batchPda,
        transaction: batchTransactionPda,
      },
      {
        args: {
          ephemeralSigners: 0,
          transactionMessage: transactionMessageBytes,
        },
      },
      multisig.PROGRAM_ID,
    );
  }

  private async proposalActivateIx(): Promise<web3.TransactionInstruction> {
    const multisigInfo = await this.getMultisigInfo();
    const transactionIndex = Number(multisigInfo.transactionIndex) + 1;
    this.logger.info(`Proposal Activate Transaction Index -- ${transactionIndex}`);
    return multisig.instructions.proposalActivate({
      multisigPda: this.squadsMultisigApp.multisigAddress,
      member: this.squadsMultisigApp.keypair.publicKey,
      transactionIndex: BigInt(transactionIndex),
    });
  }

  private async proposalApproveIx(): Promise<web3.TransactionInstruction> {
    const multisigInfo = await this.getMultisigInfo();
    const transactionIndex = Number(multisigInfo.transactionIndex) + 1;
    this.logger.info(`Proposal Approve Transaction Index -- ${transactionIndex}`);
    return multisig.instructions.proposalApprove({
      multisigPda: this.squadsMultisigApp.multisigAddress,
      member: this.squadsMultisigApp.keypair.publicKey,
      transactionIndex: BigInt(transactionIndex),
    });
  }

  private async createProposalIx(): Promise<web3.TransactionInstruction> {
    const multisigInfo = await this.getMultisigInfo();
    const transactionIndex = Number(multisigInfo.transactionIndex) + 1;
    const proposalCreateInstruction = multisig.instructions.proposalCreate({
      multisigPda: this.squadsMultisigApp.multisigAddress,
      transactionIndex: BigInt(transactionIndex),
      creator: this.squadsMultisigApp.keypair.publicKey,
      isDraft: true,
    });
    this.logger.info(
      `Create Proposal Transaction Index -- ${transactionIndex}`,
    );
    return proposalCreateInstruction;
  }

  private async createBatchIx(): Promise<web3.TransactionInstruction> {
    const multisigInfo = await this.getMultisigInfo();
    const transactionIndex = Number(multisigInfo.transactionIndex) + 1;
    const batchCreateInstruction = multisig.instructions.batchCreate({
      batchIndex: BigInt(transactionIndex),
      creator: this.squadsMultisigApp.keypair.publicKey,
      multisigPda: this.squadsMultisigApp.multisigAddress,
      vaultIndex: 0,
    });
    this.logger.info(`Create Batch Transaction Index -- ${transactionIndex}`);
    return batchCreateInstruction;
  }
}