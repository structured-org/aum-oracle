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
import { FixableBeetArgsStruct } from '@metaplex-foundation/beet';

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

export const PROGRAM_ID = new web3.PublicKey(
  'orac315UJ2aQXvgiWjoFsZZDzEEuPaozYLDHf6epJzd',
);

export default class RelaySolana implements Manager {
  private logger: Logger;
  private solana: SolanaConfig;
  private neutron: NeutronConfig;

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
    /* Neutron */
    this.cosmWasmClient = await CosmWasmClient.connect(this.neutron.rpc);
    /* Solana */
    this.signer = web3.Keypair.fromSecretKey(
      Uint8Array.from(
        JSON.parse(fs.readFileSync(this.solana.seedPath!, 'utf-8')),
      ),
    );
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
      await Program.fetchIdl(PROGRAM_ID, this.provider),
    );
  }

  async tick(): Promise<void> {
    console.log(await this.squadsMultisig?.voteProposal(6));
    // const aumData = await this.getNeutronAumData();
    // const ix = await this.publishDataIx(aumData);
    // console.log(await this.squadsMultisig?.submitProposal(ix));
  }

  private async publishDataIx(
    dataNeutron: AumDataNeutron,
  ): Promise<web3.TransactionInstruction> {
    const dataSolana: AumDataSolana = {
      timestamp: new BN(dataNeutron.last_published_data.timestamp),
      round: new BN(dataNeutron.last_published_data.round),
      data: {
        pmAccountActualEquity: toFixedDecimal(
          dataNeutron.last_published_data.data
            .pm_account_actual_equity as string,
        ),
        unimmr: toFixedDecimal(
          dataNeutron.last_published_data.data.unimmr as string,
        ),
        umBalanceUsdt: toFixedDecimal(
          dataNeutron.last_published_data.data.um_balance_usdt as string,
        ),
        withdrawableUsdt: toFixedDecimal(
          dataNeutron.last_published_data.data.withdrawable_usdt as string,
        ),
        positions: dataNeutron.last_published_data.data.positions.map((e) => ({
          pnl: toFixedDecimal(e.pnl as string),
          amount: toFixedDecimal(e.amount as string),
          symbol: e.symbol,
        })),
        spotBalances: dataNeutron.last_published_data.data.spot_balances.map(
          (e) => ({
            amount: toFixedDecimal(e.amount as string),
            asset: e.asset,
          }),
        ),
      },
    };
    return this.aumOracleProgram?.methods
      .publishData({
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
      PROGRAM_ID,
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
