import * as anchor from '@coral-xyz/anchor';
import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import {
  instantiate2Address,
  SigningCosmWasmClient,
} from '@cosmjs/cosmwasm-stargate';
import { Client as NeutronClient } from '@neutron-org/client-ts';
import { AccountData, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { setupPark } from '../testSuite';
import Cosmopark from '@neutron-org/cosmopark';
import { LAMPORTS_PER_SOL } from '@solana/web3.js';
import path from 'path';
import fs from 'fs';
import { execSync } from 'child_process';
import { GasPrice } from '@cosmjs/stargate';
import { fromHex, toAscii } from '@cosmjs/encoding';
import idl from '../../../solana-contracts/target/idl/oracle_data_receiver.json';
import { OracleDataReceiver } from '../../../solana-contracts/target/types/oracle_data_receiver';
import { sleep } from '../helpers/sleep';
import { waitFor } from '../helpers/waitFor';

const run = (cmd: string, cwd: string = process.cwd()) => {
  execSync(cmd, { stdio: 'inherit', cwd });
};

describe('Solana', () => {
  const context: {
    park?: Cosmopark;
    wallet?: DirectSecp256k1HdWallet;
    account?: AccountData;
    client?: SigningCosmWasmClient;
    neutronClient?: InstanceType<typeof NeutronClient>;
  } = {};

  let testsKeypairPath: string;
  let programKeypairPath: string;
  let program: anchor.Program<OracleDataReceiver>;

  let solanaTestsKeypair: anchor.web3.Keypair;
  let programKeypair: anchor.web3.Keypair;
  let instanceKeypair: anchor.web3.Keypair;
  let aumMessenger1Keypair: anchor.web3.Keypair;
  let aumMessenger2Keypair: anchor.web3.Keypair;
  let aumMessenger3Keypair: anchor.web3.Keypair;
  let configPDA: anchor.web3.PublicKey;

  let solanaProvider: anchor.AnchorProvider;

  beforeAll(async (t) => {
    console.log('setting up park');
    context.park = await setupPark(t, ['neutron'], {
      'aum-messenger-1': './configs/aum-messenger-1-config-prod.yaml',
      'aum-messenger-2': './configs/aum-messenger-2-config-prod.yaml',
      'aum-messenger-3': './configs/aum-messenger-3-config-prod.yaml',
    });
    console.log('park has been setup');

    solanaProvider = anchor.AnchorProvider.local();

    testsKeypairPath = path.join(__dirname, '../../keypairs/tests_wallet.json');

    solanaTestsKeypair = anchor.web3.Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(
          fs.readFileSync(testsKeypairPath, {
            encoding: 'utf-8',
          }),
        ),
      ),
    );

    aumMessenger1Keypair = anchor.web3.Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(
          fs.readFileSync(
            path.join(__dirname, '../../keypairs/aum-messenger-1-keypair.json'),
            {
              encoding: 'utf-8',
            },
          ),
        ),
      ),
    );

    aumMessenger2Keypair = anchor.web3.Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(
          fs.readFileSync(
            path.join(__dirname, '../../keypairs/aum-messenger-2-keypair.json'),
            {
              encoding: 'utf-8',
            },
          ),
        ),
      ),
    );

    aumMessenger3Keypair = anchor.web3.Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(
          fs.readFileSync(
            path.join(__dirname, '../../keypairs/aum-messenger-3-keypair.json'),
            {
              encoding: 'utf-8',
            },
          ),
        ),
      ),
    );

    instanceKeypair = anchor.web3.Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(
          fs.readFileSync(
            path.join(
              __dirname,
              '../../keypairs/arbitrary-data-instance-keypair.json',
            ),
            {
              encoding: 'utf-8',
            },
          ),
        ),
      ),
    );

    context.wallet = await DirectSecp256k1HdWallet.fromMnemonic(
      context.park.config.wallets.predefined.mnemonic,
      {
        prefix: 'neutron',
      },
    );

    context.account = (await context.wallet.getAccounts())[0];

    context.client = await SigningCosmWasmClient.connectWithSigner(
      `http://127.0.0.1:${context.park.ports.neutron.rpc}`,
      context.wallet,
      {
        gasPrice: GasPrice.fromString('0.025untrn'),
      },
    );
  });

  afterAll(async () => {
    // await context.park.stop();
  });

  it('top up working accounts', async () => {
    const { blockhash, lastValidBlockHeight } =
      await solanaProvider.connection.getLatestBlockhash('confirmed');

    let signature = await solanaProvider.connection.requestAirdrop(
      solanaTestsKeypair.publicKey,
      100 * LAMPORTS_PER_SOL, // 100 SOL
    );

    await solanaProvider.connection.confirmTransaction(
      {
        signature,
        blockhash,
        lastValidBlockHeight,
      },
      'confirmed',
    );

    let balanceLamports = await solanaProvider.connection.getBalance(
      solanaTestsKeypair.publicKey,
    );
    let balanceSOL = balanceLamports / LAMPORTS_PER_SOL;

    expect(balanceSOL).toEqual(100);

    signature = await solanaProvider.connection.requestAirdrop(
      aumMessenger1Keypair.publicKey,
      100 * LAMPORTS_PER_SOL, // 100 SOL
    );
    await solanaProvider.connection.confirmTransaction(
      {
        signature,
        blockhash,
        lastValidBlockHeight,
      },
      'confirmed',
    );

    balanceLamports = await solanaProvider.connection.getBalance(
      aumMessenger1Keypair.publicKey,
    );
    balanceSOL = balanceLamports / LAMPORTS_PER_SOL;

    expect(balanceSOL).toEqual(100);

    signature = await solanaProvider.connection.requestAirdrop(
      aumMessenger2Keypair.publicKey,
      100 * LAMPORTS_PER_SOL, // 100 SOL
    );
    await solanaProvider.connection.confirmTransaction(
      {
        signature,
        blockhash,
        lastValidBlockHeight,
      },
      'confirmed',
    );

    balanceLamports = await solanaProvider.connection.getBalance(
      aumMessenger2Keypair.publicKey,
    );
    balanceSOL = balanceLamports / LAMPORTS_PER_SOL;

    expect(balanceSOL).toEqual(100);

    signature = await solanaProvider.connection.requestAirdrop(
      aumMessenger3Keypair.publicKey,
      100 * LAMPORTS_PER_SOL, // 100 SOL
    );
    await solanaProvider.connection.confirmTransaction(
      {
        signature,
        blockhash,
        lastValidBlockHeight,
      },
      'confirmed',
    );

    balanceLamports = await solanaProvider.connection.getBalance(
      aumMessenger3Keypair.publicKey,
    );
    balanceSOL = balanceLamports / LAMPORTS_PER_SOL;

    expect(balanceSOL).toEqual(100);
  });

  it('deploy oracle data receiver', async () => {
    const programName = 'oracle_data_receiver';
    programKeypairPath = path.join(
      __dirname,
      `../../../solana-contracts/keypairs/${programName}-keypair.json`,
    );

    programKeypair = anchor.web3.Keypair.fromSecretKey(
      Buffer.from(
        JSON.parse(
          fs.readFileSync(programKeypairPath, {
            encoding: 'utf-8',
          }),
        ),
      ),
    );

    const binaryPath = path.join(
      __dirname,
      `../../../solana-contracts/target/deploy/${programName}.so`,
    );

    const programPath = path.join(
      __dirname,
      '../../../solana-contracts/programs/oracle-data-receiver',
    );

    run('anchor build', programPath);

    run(
      `solana program deploy ${binaryPath} --program-id ${programKeypairPath} --fee-payer ${testsKeypairPath} --url http://127.0.0.1:8899`,
    );

    expect(
      (await solanaProvider.connection.getAccountInfo(programKeypair.publicKey))
        .executable,
    ).toBeTruthy();
  });

  it('initialize Oracle Data Receiver', async () => {
    await sleep(5000); // Wait for program to be fully deployed

    program = new anchor.Program<OracleDataReceiver>(
      idl as anchor.Idl,
      solanaProvider,
    );

    [configPDA] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from('config'), instanceKeypair.publicKey.toBuffer()],
      program.programId,
    );

    const ix = await program.methods
      .initialize({
        roundTime: 3,
        owner: solanaTestsKeypair.publicKey,
      })
      .accounts({
        signer: solanaTestsKeypair.publicKey,
        instanceKey: instanceKeypair.publicKey,
      })
      .instruction();

    const tx = new anchor.web3.Transaction().add(ix);
    await solanaProvider.simulate(tx, [solanaTestsKeypair, instanceKeypair]);

    await solanaProvider.sendAndConfirm(tx, [
      solanaTestsKeypair,
      instanceKeypair,
    ]);
  });

  it('add oracles to Oracle Data Receiver', async () => {
    await program.methods
      .addOracle(aumMessenger1Keypair.publicKey)
      .accounts({
        config: configPDA,
        signer: solanaTestsKeypair.publicKey,
      })
      .signers([solanaTestsKeypair])
      .rpc();

    await program.methods
      .addOracle(aumMessenger2Keypair.publicKey)
      .accounts({
        config: configPDA,
        signer: solanaTestsKeypair.publicKey,
      })
      .signers([solanaTestsKeypair])
      .rpc();

    await program.methods
      .addOracle(aumMessenger3Keypair.publicKey)
      .accounts({
        config: configPDA,
        signer: solanaTestsKeypair.publicKey,
      })
      .signers([solanaTestsKeypair])
      .rpc();

    const config = await program.account.config.fetch(configPDA);

    const oracles = config.oracles.map((pk: anchor.web3.PublicKey) =>
      pk.toBase58(),
    );

    expect(oracles).toEqual([
      aumMessenger1Keypair.publicKey.toBase58(),
      aumMessenger2Keypair.publicKey.toBase58(),
      aumMessenger3Keypair.publicKey.toBase58(),
    ]);
  });

  it('deploy arbitrary data test contract', async () => {
    const { client, account } = context;
    const res = await client.upload(
      account.address,
      Uint8Array.from(
        fs.readFileSync(
          path.join(
            __dirname,
            '../../artifacts/contracts/arbitrary_data_test.wasm',
          ),
        ),
      ),
      1.5,
    );
    expect(res.codeId).toBeGreaterThan(0);

    const arbitraryDataContractCodeId = res.codeId;

    const arbitraryDataContractAddress = instantiate2Address(
      fromHex(res.checksum),
      account.address,
      toAscii('s'),
      'neutron',
    );

    const result = await client.instantiate2(
      account.address,
      arbitraryDataContractCodeId,
      toAscii('s'),
      {},
      'oracles-arbitry-data-test',
      1.5,
    );

    expect(result.contractAddress).toEqual(arbitraryDataContractAddress);
  });

  describe('Consensus', () => {
    describe('Happy path', () => {
      let consensusPDA: anchor.web3.PublicKey;

      it('check oracle data receiver consensus state', async () => {
        [consensusPDA] = anchor.web3.PublicKey.findProgramAddressSync(
          [Buffer.from('consensus'), instanceKeypair.publicKey.toBuffer()],
          program.programId,
        );

        await waitFor(
          async () => {
            const consensus =
              await program.account.consensus.fetch(consensusPDA);

            return consensus.timestamp > 0;
          },
          100_000,
          1_000,
        );

        const consensus = await program.account.consensus.fetch(consensusPDA);

        const testData = JSON.parse(Buffer.from(consensus.data).toString());

        expect(testData).toEqual({
          frist_value: 'first_value',
          second_value: 'second_value',
        });
      });
    });
  });
});
