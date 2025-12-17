import Manager from '../index';
import { Logger } from 'pino';
import { NeutronConfig, SolanaConfig } from '../../lib/config';
import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import SquadsMultisig, { SquadsMultisigConfig } from '../../lib/multisig/squads';
import { AnchorProvider, BN, Program, setProvider, Wallet, web3 } from '@coral-xyz/anchor';
import * as fs from 'node:fs';
import { FixableBeetArgsStruct } from '@metaplex-foundation/beet';

type FixedDecimal = {
  number: BN,
  decimals: number
};

function toFixedDecimal(number: string): FixedDecimal {
  return {
    number: new BN(number.replace(/\./g, '')),
    decimals: number.split('.')[1]?.length ?? 0,
  };
}

type AumData = {
  last_published_data: {
    round: number | BN;
    timestamp: | BN;
    data: {
      unimmr: string | FixedDecimal;
      positions: {
        symbol: string;
        amount: string | FixedDecimal;
        pnl: string | FixedDecimal;
      }[];
      um_balance_usdt: string | FixedDecimal;
      spot_balances: {
        asset: string;
        amount: string | FixedDecimal;
      }[];
      pm_account_actual_equity: string | FixedDecimal;
      withdrawable_usdt: string | FixedDecimal;
    };
  };
};

export const PROGRAM_ID = new web3.PublicKey('orac315UJ2aQXvgiWjoFsZZDzEEuPaozYLDHf6epJzd');

export default class RelaySolana implements Manager {
  private logger: Logger;
  private solana: SolanaConfig;
  private neutron: NeutronConfig;

  private cosmWasmClient?: CosmWasmClient;
  private squadsMultisig?: SquadsMultisig;
  private provider?: AnchorProvider;
  private aumOracleProgram?: Program;
  private signer?: web3.Keypair;

  constructor(logger: Logger, solanaConfig: SolanaConfig, neutronConfig: NeutronConfig) {
    this.logger = logger;
    this.solana = solanaConfig;
    this.neutron = neutronConfig;
  }

  async init(): Promise<void> {
    /* Neutron */
    this.cosmWasmClient = await CosmWasmClient.connect(this.neutron.rpc);
    /* Solana */
    this.signer = web3.Keypair.fromSecretKey(Uint8Array.from(
      JSON.parse(fs.readFileSync(this.solana.seedPath!, 'utf-8')),
    ));
    this.provider = new AnchorProvider(new web3.Connection(this.solana.rpc, {
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
    this.squadsMultisig = new SquadsMultisig(
      this.logger,
      squadsMultisigConfig,
    );
    this.aumOracleProgram = new Program(
      await Program.fetchIdl(PROGRAM_ID, this.provider),
    );
  }

  async tick(): Promise<void> {
    const aumData = await this.getNeutronAumData();
    console.log(aumData);
    await this.publishDataIx(aumData);
  }

  private async publishDataIx(data: AumData): Promise<web3.TransactionInstruction> {
    /* For numbers > u64 or floats (hereby strings) we need to use BN for serialization */
    data.last_published_data.timestamp = new BN(data.last_published_data.timestamp);
    data.last_published_data.round = new BN(data.last_published_data.round);

    data.last_published_data.data.pm_account_actual_equity = toFixedDecimal(data.last_published_data.data.pm_account_actual_equity as string);
    data.last_published_data.data.unimmr = toFixedDecimal(data.last_published_data.data.unimmr as string);
    data.last_published_data.data.um_balance_usdt = toFixedDecimal(data.last_published_data.data.um_balance_usdt as string);
    data.last_published_data.data.withdrawable_usdt = toFixedDecimal(data.last_published_data.data.withdrawable_usdt as string);

    data.last_published_data.data.positions = data.last_published_data.data.positions.map((e) => ({
      pnl: toFixedDecimal(e.pnl as string),
      amount: toFixedDecimal(e.amount as string),
      symbol: e.symbol,
    }));
    data.last_published_data.data.spot_balances = data.last_published_data.data.spot_balances.map((e) => ({
      amount: toFixedDecimal(e.amount as string),
      asset: e.asset,
    }));

    return this.aumOracleProgram?.methods
      .publishData({
        data,
      })
      .accounts({
        signer: this.signer?.publicKey!,
        aumOracleConfig: new web3.PublicKey(this.solana.aumOracleSol),
      })
      .instruction()!;
  }

  private async getNeutronAumData(): Promise<AumData> {
    const result = await this.cosmWasmClient?.queryContractSmart(this.neutron.binanceAum, {
      'get_data': {},
    });
    this.logger.trace('TWAER Query Result: %o', result);
    return result as AumData;
  }
}