import Manager from '../index';
import { Logger } from 'pino';
import { NeutronConfig, SolanaConfig } from '../../lib/config';
import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import SquadsMultisig, {
  SquadsMultisigConfig,
} from '../../lib/multisig/squads';
import {
  AnchorProvider,
  BN,
  Program,
  setProvider,
  Wallet,
  web3,
} from '@coral-xyz/anchor';
import * as fs from 'node:fs';
import {
  Ms,
  proposalStatusToString,
  VaultTransaction,
} from '../../lib/multisig/squads/internal';
import { mnemonicToSeedSync, validateMnemonic } from 'bip39';
import { derivePath } from 'ed25519-hd-key';

type FixedDecimal = {
  number: BN;
  decimals: number;
};

function toFixedDecimal(number: string): FixedDecimal {
  return {
    number: new BN(number.replace(/\./g, '')),
    decimals: number.split('.')[1]?.length ?? 0,
  };
}

type AumOracleState = {
  lastPublishedData: AumDataSolana;
  solanaPublicationTimestamp: BN;
};

type AumDataSolana = {
  round: BN;
  timestamp: BN;
  data: {
    unimmr: FixedDecimal;
    positions: {
      symbol: string;
      amount: FixedDecimal;
      pnl: FixedDecimal;
    }[];
    umBalanceUsdt: FixedDecimal;
    spotBalances: {
      asset: string;
      amount: FixedDecimal;
    }[];
    pmAccountActualEquity: FixedDecimal;
    withdrawableUsdt: FixedDecimal;
  };
};

type AumDataNeutron = {
  last_published_data: {
    round: number;
    timestamp: number;
    data: {
      unimmr: string;
      positions: {
        symbol: string;
        amount: string;
        pnl: string;
      }[];
      um_balance_usdt: string;
      spot_balances: {
        asset: string;
        amount: string;
      }[];
      pm_account_actual_equity: string;
      withdrawable_usdt: string;
    };
  };
};

function keypairFromMnemonic(
  mnemonic: string,
  derivationPath = "m/44'/501'/0'/0'",
): web3.Keypair {
  if (!validateMnemonic(mnemonic)) {
    throw new Error('Invalid mnemonic');
  }
  const seed = mnemonicToSeedSync(mnemonic);
  const derived = derivePath(derivationPath, seed.toString('hex'));
  const derivedKey = derived.key;
  if (derivedKey.length !== 32) {
    throw new Error('Derived key is not 32 bytes');
  }
  return web3.Keypair.fromSeed(derivedKey);
}

export default class RelaySolana implements Manager {
  private logger: Logger;
  private solana: SolanaConfig;
  private neutron: NeutronConfig;

  private msParticipantIndex?: number;
  private cosmWasmClient?: CosmWasmClient;
  private squadsMultisig?: SquadsMultisig;
  private provider?: AnchorProvider;
  private aumOracleProgram?: Program;
  private signer?: web3.Keypair;

  constructor(
    logger: Logger,
    solanaConfig: SolanaConfig,
    neutronConfig: NeutronConfig,
  ) {
    this.logger = logger;
    this.solana = solanaConfig;
    this.neutron = neutronConfig;
  }

  async init(): Promise<void> {
    this.logger.info(
      `RelaySolana proposal delay -- ${this.solana.proposalDelay}`,
    );

    /* Neutron */
    this.cosmWasmClient = await CosmWasmClient.connect(this.neutron.rpc);
    /* Solana */
    if(this.solana.seed) {
      this.signer = web3.Keypair.fromSecretKey(this.solana.seed);
    } else if (this.solana.mnemonic) {
      this.signer = keypairFromMnemonic(this.solana.mnemonic!);
    } else {
      throw new Error('Neither seedPath nor mnemonic were provided');
    }

    this.provider = new AnchorProvider(
      new web3.Connection(this.solana.rpc, {
        commitment: 'confirmed',
      }),
      new Wallet(this.signer),
      {
        commitment: 'confirmed',
        skipPreflight: true,
      },
    );
    /* Global provider allows work with functions like fetchIdl */
    setProvider(this.provider);

    const squadsMultisigConfig: SquadsMultisigConfig = {
      anchorProvider: this.provider,
      multisigAddress: new web3.PublicKey(this.solana.multisigAddress),
      vaultPda: new web3.PublicKey(this.solana.vaultPda),
      keypair: this.signer,
    };
    this.squadsMultisig = new SquadsMultisig(this.logger, squadsMultisigConfig);
    this.aumOracleProgram = new Program(
      await Program.fetchIdl(this.solana.oracleProgramId, this.provider),
    );

    /* Find the participant index. It is used alongside the 'proposalDelay' */
    const msData = await this.provider.connection.getAccountInfo(
      new web3.PublicKey(this.solana.multisigAddress),
    );
    const ms = Ms.deserialize(msData?.data!);
    this.msParticipantIndex = ms.members!.findIndex(
      (member) => member.key.toBase58() === this.signer?.publicKey.toBase58(),
    );
  }

  async createProposal() {
    const aumNeutronData = await this.getNeutronAumData();
    const aumSolanaData = await this.getSolanaAumData();

    /* Check the freshness and if the new state needs to be submitted, propose */
    if (
      aumSolanaData.lastPublishedData.timestamp.toString() !==
      aumNeutronData.last_published_data.timestamp.toString()
    ) {
      const ix = await this.publishDataIx(aumNeutronData);
      const txhash = await this.squadsMultisig?.submitProposal(ix);
      this.logger.info(`Submited new proposal -- ${txhash}`);
    }
  }

  async tick(): Promise<void> {
    await new Promise((resolve) =>
      setTimeout(resolve, this.solana.proposalDelay * this.msParticipantIndex! * 1000),
    );

    /* Get all the proposals that are either Active of Approved */
    const proposals = (await this.squadsMultisig?.getPendingProposals())!;
    if (proposals.length) {
      const proposal = proposals![0];
      const proposalStatus = proposalStatusToString(proposal.status!);

      /* If it is Approved, execute */
      if (proposalStatus === 'Approved') {
        try {
          const txhash = await this.squadsMultisig?.executeProposal(
            Number(proposal.transactionIndex?.toString())!,
          );
          this.logger.info(
            `Executed proposal ${proposal.transactionIndex} -- ${txhash}`,
          );
        } catch (e: any) {
          if (/SameTimestamp/.test(e.message.toString())) {
            this.logger.warn(
              `Outdated timestamp proposal -- ${proposal.transactionIndex}`,
            );
            await this.createProposal();
          } else {
            this.logger.warn(
              `Unknown error happened -- ${proposal.transactionIndex}`,
            );
          }
        }
      }

      /* If it is Active, then if we have not voted yet, vote */
      if (
        proposalStatus === 'Active' &&
        !proposal
          .approved!.map((approved) => approved.toBase58())
          .includes(this.signer?.publicKey.toBase58()!)
      ) {
        const batchIxs = await this.squadsMultisig?.getBatchIxs(
          Number(proposal.transactionIndex!.toString()),
        );
        if (!this.validateIxs(batchIxs!)) {
          this.logger.warn(`Invalid proposal -- ${proposal.transactionIndex}`);
          return;
        }

        const txhash = await this.squadsMultisig?.voteProposal(
          Number(proposal.transactionIndex?.toString())!,
        );
        this.logger.info(
          `Voted for proposal ${proposal.transactionIndex} -- ${txhash}`,
        );
      }
    } else {
      await this.createProposal();
    }
  }

  private validateIxs(ixs: Array<VaultTransaction>): boolean {
    const DISCRIMINATOR = "230,18,158,253,73,167,115,188";
    return ixs.entries().every(([, transaction]) =>
      transaction.message?.instructions.every(instruction =>
        Array.from(instruction.data.subarray(0, 8)).join(",") === DISCRIMINATOR
      )
    );
  }

  private publishDataIx(
    dataNeutron: AumDataNeutron,
  ): Promise<web3.TransactionInstruction> {
    const dataSolana: AumDataSolana = {
      timestamp: new BN(dataNeutron.last_published_data.timestamp),
      round: new BN(dataNeutron.last_published_data.round),
      data: {
        pmAccountActualEquity: toFixedDecimal(
          dataNeutron.last_published_data.data
            .pm_account_actual_equity,
        ),
        unimmr: toFixedDecimal(
          dataNeutron.last_published_data.data.unimmr,
        ),
        umBalanceUsdt: toFixedDecimal(
          dataNeutron.last_published_data.data.um_balance_usdt,
        ),
        withdrawableUsdt: toFixedDecimal(
          dataNeutron.last_published_data.data.withdrawable_usdt,
        ),
        positions: dataNeutron.last_published_data.data.positions.map((e) => ({
          pnl: toFixedDecimal(e.pnl),
          amount: toFixedDecimal(e.amount),
          symbol: e.symbol,
        })),
        spotBalances: dataNeutron.last_published_data.data.spot_balances.map(
          (e) => ({
            amount: toFixedDecimal(e.amount),
            asset: e.asset,
          }),
        ),
      },
    };
    return this.aumOracleProgram!.methods.publishData({
      data: dataSolana,
    })
      .accounts({
        signer: this.solana.vaultPda,
        aumOracleConfig: new web3.PublicKey(this.solana.aumOracleSol),
      })
      .instruction()!;
  }

  private async getSolanaAumData(): Promise<AumOracleState> {
    const aumOracleConfig = new web3.PublicKey(this.solana.aumOracleSol);
    const configData =
      await this.provider?.connection.getAccountInfo(aumOracleConfig);
    const config = this.aumOracleProgram?.coder.accounts.decode(
      'aumOracleConfig',
      configData!.data,
    );
    const aumOracleState = web3.PublicKey.findProgramAddressSync(
      [Buffer.from('state'), config.instanceKey.toBuffer()],
      new web3.PublicKey(this.solana.oracleProgramId),
    )[0];
    const stateData =
      await this.provider?.connection.getAccountInfo(aumOracleState);
    const state = this.aumOracleProgram?.coder.accounts.decode(
      'aumOracleState',
      stateData!.data,
    );
    return state as AumOracleState;
  }

  private async getNeutronAumData(): Promise<AumDataNeutron> {
    const result = await this.cosmWasmClient?.queryContractSmart(
      this.neutron.binanceAum,
      {
        get_data: {},
      },
    );
    this.logger.trace('TWAER Query Result: %o', result);
    return result as AumDataNeutron;
  }
}
