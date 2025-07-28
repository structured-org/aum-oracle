import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Client as NeutronClient } from '@neutron-org/client-ts';
import { AccountData, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice } from '@cosmjs/stargate';
import { setupPark } from '../testSuite';
import Cosmopark from '@neutron-org/cosmopark';
import { sleep } from '../helpers/sleep';
import {
  BinanceUMPosition,
  BinancePmAccountInfo,
  BinancePmAccountBalance,
  BinanceSpotAccountInfo,
  JupiterPerpsCustodyAccount,
  JupiterPoolAccount,
  SolanaUiTokenAmount,
  MockController,
} from '../helpers/messenger';
import { waitFor, waitSeconds } from '../helpers/waitFor';
import {
  BINANCE_CONTRACT,
  BinanceData,
  JUPITER_CONTRACT,
  queryLastPublishedData,
  SolanaData,
} from '../helpers/queries';
import { execSync } from 'child_process';

type MockData = {
  umPositions?: BinanceUMPosition[];
  pmAccountInfo?: BinancePmAccountInfo;
  pmAccountBalance?: BinancePmAccountBalance[];
  spotAccountInfo?: BinanceSpotAccountInfo;
  custodyInfos?: JupiterPerpsCustodyAccount[];
  poolInfo?: JupiterPoolAccount;
  tokenSupply?: SolanaUiTokenAmount;
  tokenAccountBalance?: SolanaUiTokenAmount;
};

export async function fetchMockData(
  mockController: MockController,
): Promise<MockData> {
  const custodyKeys = [
    '7xS2gz2bTp3fwCC7knJvUWTEU9Tycczu6VhJYKgi1wdz',
    'AQCGyheWPLeo6Qp9WpYS9m3Qj479t7R636N9ey1rEjEn',
    '5Pv3gM9JrFFH883SWAhvJC9RPYmo8UNxuFtv5bMMALkm',
    'G18jKKXQwBbrHeiK3C9MRXhkHsLHf7XgCSisykV46EZa',
    '4vkNeXiYEUizLdrpdPS1eC2mccyM4NUPRtERrk6ZETkk',
  ];
  const sampleTokenPublicKey = '11111111111111111111111111111111';

  const custodyInfos = await Promise.all(
    custodyKeys.map((key) => mockController.getJupiterPerpsCustodyInfo(key)),
  );

  return {
    umPositions: await mockController.getBinanceUmPositions(),
    pmAccountInfo: await mockController.getBinancePMAccountInfo(),
    pmAccountBalance: await mockController.getBinancePMAccountBalance(),
    spotAccountInfo: await mockController.getBinanceSpotAccountInfo(),
    custodyInfos,
    poolInfo: await mockController.getJupiterPoolInfo(),
    tokenSupply: await mockController.getSolanaTokenSupply(),
    tokenAccountBalance:
      await mockController.getSolanaTokenAccountBalance(sampleTokenPublicKey),
  };
}

describe('Consensus', () => {
  const context: {
    park?: Cosmopark;
    wallet?: DirectSecp256k1HdWallet;
    account?: AccountData;
    client?: SigningCosmWasmClient;
    neutronClient?: InstanceType<typeof NeutronClient>;
  } = {};

  beforeAll(async (t) => {
    console.log('setting up park');
    context.park = await setupPark(t, ['neutron']);
    console.log('park has been setup');

    context.wallet = await DirectSecp256k1HdWallet.fromMnemonic(
      'banner spread envelope side kite person disagree path silver will brother under couch edit food venture squirrel civil budget number acquire point work mass',
      {
        prefix: 'neutron',
      },
    );

    context.account = (await context.wallet.getAccounts())[0];
    context.neutronClient = new NeutronClient({
      apiURL: `http://127.0.0.1:${context.park.ports.neutron.rest}`,
      rpcURL: `127.0.0.1:${context.park.ports.neutron.rpc}`,
      prefix: 'neutron',
    });

    context.client = await SigningCosmWasmClient.connectWithSigner(
      `http://127.0.0.1:${context.park.ports.neutron.rpc}`,
      context.wallet,
      {
        gasPrice: GasPrice.fromString('0.025untrn'),
      },
    );

    console.log('waiting for mock controller to start');
    await sleep(10000);
  });

  afterAll(async () => {
    await context.park.stop();
  });

  describe('Consensus', () => {
    const mockController1 = new MockController(3001);
    const mockController2 = new MockController(3002);
    const mockController3 = new MockController(3003);
    const mockControllers = [mockController1, mockController2, mockController3];

    describe('Happy path', () => {
      it('publishes data as expected', async () => {
        await waitFor(
          async () => {
            const result = await queryLastPublishedData<SolanaData>(
              JUPITER_CONTRACT,
              context.client,
            );
            // wait until we have same changed aum usd gotten from contract
            return (
              result.data.aum_usd ===
              Math.trunc(1_531_381_751_507_034 / 1_000_000).toString()
            );
          },
          20_000,
          1_000,
          true,
        );
      });

      it('changes published data as expected', async () => {
        const data = await fetchMockData(mockController1);
        // let's change one property for consensus example
        data.poolInfo.aumUsd = (1_531_381_751_507_034 * 2).toString(); // let's double it

        for (const mockController of mockControllers) {
          await mockController.setJupiterPoolInfo(data.poolInfo);
        }

        await waitFor(
          async () => {
            const result = await queryLastPublishedData<SolanaData>(
              JUPITER_CONTRACT,
              context.client,
            );
            // wait until we have same changed aum usd gotten from contract
            return (
              result.data.aum_usd ===
              Math.trunc((1_531_381_751_507_034 * 2) / 1_000_000).toString()
            );
          },
          20_000,
          1_000,
        );
      });
    });

    describe('Malicious oracle', () => {
      it('one malicious oracle does not change published data as', async () => {
        const data = await fetchMockData(mockController1);
        const originalSupplyAmount = data.pmAccountInfo.actualEquity;
        data.pmAccountInfo.actualEquity = (
          +data.pmAccountInfo.actualEquity * 7
        ).toString(); // Too much
        await mockController3.setBinancePMAccountInfo(data.pmAccountInfo);

        // wait 2 rounds, in case that one oracle already published data in this round before the set info call (?)
        const result = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        const currentRound = result.round;
        await waitFor(
          async () => {
            const result = await queryLastPublishedData<any>(
              BINANCE_CONTRACT,
              context.client,
            );
            return result.round > currentRound + 1;
          },
          40_000,
          1_000,
        );

        const nextRoundResult = await queryLastPublishedData<BinanceData>(
          BINANCE_CONTRACT,
          context.client,
        );
        expect(nextRoundResult.data.pm_account_actual_equity).toEqual(
          originalSupplyAmount,
        );
      });

      afterAll(async () => {
        // restore mockController3 to original data
        const data = await fetchMockData(mockController1);
        await mockController3.setBinancePMAccountInfo(data.pmAccountInfo);
      });
    });

    describe('Oracles do not agree on data', () => {
      it('different data reported stops publishing data', async () => {
        const data = await fetchMockData(mockController1);
        const originalUnimmr = data.pmAccountInfo.uniMMR;
        await mockController2.setBinancePMAccountInfo({
          ...data.pmAccountInfo,
          uniMMR: (+originalUnimmr * 13).toString(),
        });
        await mockController3.setBinancePMAccountInfo({
          ...data.pmAccountInfo,
          uniMMR: (+originalUnimmr * 26).toString(),
        });

        const result = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        // wait for the possible next round in case some oracles already submitted data for this round
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > result.round;
          },
          20_000,
          2_000,
          false, // no exception, just wait
        );

        const resultBefore = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );

        // after multiple times new round should happen, publish data still should return old round
        await waitSeconds(20);

        const resultAfter = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );

        expect(resultAfter.round).toEqual(resultBefore.round);
        expect(resultAfter.data.unimmr).toEqual(resultBefore.data.unimmr);
      });
      // all three oracles report completely different data
      // sets checks on grouping: one oracle passes incorrect data on exact field like decimals,
      //    other values from this oracle does not count in eventual consensus

      afterAll(async () => {
        // set mockController3 to original data
        const data = await fetchMockData(mockController1);
        await mockController1.setBinancePMAccountInfo(data.pmAccountInfo);
        await mockController2.setBinancePMAccountInfo(data.pmAccountInfo);
        await mockController3.setBinancePMAccountInfo(data.pmAccountInfo);
      });
    });

    describe('Oracles paused (down) for some time', () => {
      it('one oracle does not stop data publishing', async () => {
        execSync(`docker pause consensus-aum-oracle-2-1`); // pause one oracle

        const resultBefore = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );

        // the next round should happen
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > resultBefore.round + 1;
          },
          40_000,
          1_000,
          true,
        );
      });

      it('two oracles stop data publishing', async () => {
        execSync(`docker pause consensus-aum-oracle-3-1`); // pause second oracle
        // even tho the oracle is paused first oracle can still change consensus right?
        // wait one round to be sure since one oracle can still change consensus one more time (when second oracle already submitted this round, first oracle is needed)
        const resultAfterPause = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > resultAfterPause.round;
          },
          20_000,
          2_000,
          false,
        );

        const resultBefore = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );

        // the next round should not happen
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > resultBefore.round + 1;
          },
          40_000,
          2_000,
          false,
        );

        const resultAfter = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        expect(resultAfter.round).toEqual(resultBefore.round);
      });

      it('all three oracles stop data publishing', async () => {
        execSync(`docker pause consensus-aum-oracle-1-1`); // pause first oracle
        const resultBefore = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );

        // the next round should not happen
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > resultBefore.round;
          },
          20_000,
          1_000,
          false,
        );

        const resultAfter = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        expect(resultAfter.round).toEqual(resultBefore.round);
      });

      it('restore two oracles, data publishing should resume', async () => {
        const resultBefore = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );

        execSync(`docker unpause consensus-aum-oracle-1-1`);
        execSync(`docker unpause consensus-aum-oracle-2-1`);

        // wait for the next round
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > resultBefore.round;
          },
          40_000,
          1_000,
          true,
        );

        execSync(`docker unpause consensus-aum-oracle-3-1`);
      });
    });

    describe('Data sources timeouts and problems', () => {
      it('requests to solana timeouts', async () => {
        for (const mockController of mockControllers) {
          await mockController.enableSolanaTimeout();
        }

        // wait one round to be sure (maybe some requests were already in progress)
        const resultAfterPause = await queryLastPublishedData<BinanceData>(
          JUPITER_CONTRACT,
          context.client,
        );
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<BinanceData>(
              JUPITER_CONTRACT,
              context.client,
            );
            return checkResult.round > resultAfterPause.round;
          },
          30_000,
          2_000,
          false,
        );

        const resultBefore = await queryLastPublishedData<any>(
          JUPITER_CONTRACT,
          context.client,
        );
        const binanceResultBefore = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        // wait for the next round
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              JUPITER_CONTRACT,
              context.client,
            );
            return checkResult.round > resultBefore.round;
          },
          40_000,
          1_000,
          false,
        );
        const resultAfter = await queryLastPublishedData<any>(
          JUPITER_CONTRACT,
          context.client,
        );
        expect(resultBefore.round).toEqual(resultAfter.round);

        // disable timeout, messenger should continue publishing data for solana
        for (const mockController of mockControllers) {
          await mockController.disableSolanaTimeout();
        }

        // binance should've worked all this time
        const binanceResultAfter = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        expect(binanceResultAfter.round).toBeGreaterThan(
          binanceResultBefore.round,
        );

        // should publish the next round
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              JUPITER_CONTRACT,
              context.client,
            );
            return checkResult.round > resultBefore.round;
          },
          40_000,
          1_000,
          true,
        );
      });

      it('requests to binance timeouts', async () => {
        // turn on 200-second timeout
        for (const mockController of mockControllers) {
          await mockController.enableBinanceTimeout();
        }

        // wait one round to be sure (maybe some requests were already in progress)
        const resultAfterPause = await queryLastPublishedData<BinanceData>(
          BINANCE_CONTRACT,
          context.client,
        );
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<BinanceData>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > resultAfterPause.round;
          },
          30_000,
          2_000,
          false,
        );

        const resultBefore = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        const jupiterResultBefore = await queryLastPublishedData<any>(
          JUPITER_CONTRACT,
          context.client,
        );
        // wait for the next round
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > resultBefore.round;
          },
          40_000,
          1_000,
          false,
        );
        const resultAfter = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        expect(resultBefore.round).toEqual(resultAfter.round);

        // disable timeout, messenger should continue publishing data for solana
        for (const mockController of mockControllers) {
          await mockController.disableBinanceTimeout();
        }

        // binance should've worked all this time
        const jupiterResultAfter = await queryLastPublishedData<any>(
          JUPITER_CONTRACT,
          context.client,
        );
        expect(jupiterResultAfter.round).toBeGreaterThan(
          jupiterResultBefore.round,
        );

        // should publish the next round
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > resultBefore.round;
          },
          40_000,
          1_000,
          true,
        );
      });

      it('all controllers timeout for all data sources', async () => {
        for (const mockController of mockControllers) {
          await mockController.enableBinanceTimeout();
          await mockController.enableSolanaTimeout();
        }

        // wait one round to be sure (maybe some requests were already in progress)
        const resultAfterPause = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        const resultAfterPause2 = await queryLastPublishedData<any>(
          JUPITER_CONTRACT,
          context.client,
        );
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              BINANCE_CONTRACT,
              context.client,
            );

            const checkResult2 = await queryLastPublishedData<any>(
              JUPITER_CONTRACT,
              context.client,
            );
            return (
              checkResult.round > resultAfterPause.round &&
              checkResult2.round > resultAfterPause2.round
            );
          },
          30_000,
          2_000,
          false,
        );

        // wait for the next round
        const resultBefore2 = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > resultBefore2.round;
          },
          40_000,
          1_000,
          false,
        );
        const resultAfter2 = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        expect(resultBefore2.round).toEqual(resultAfter2.round);

        // wait for the next round
        const resultBefore3 = await queryLastPublishedData<any>(
          JUPITER_CONTRACT,
          context.client,
        );
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<any>(
              JUPITER_CONTRACT,
              context.client,
            );
            return checkResult.round > resultBefore3.round;
          },
          40_000,
          1_000,
          false,
        );
        const resultAfter3 = await queryLastPublishedData<any>(
          JUPITER_CONTRACT,
          context.client,
        );
        expect(resultBefore3.round).toEqual(resultAfter3.round);

        // test that results are back after turning off timeouts
        // turn on 200-second timeout
        for (const mockController of mockControllers) {
          await mockController.disableBinanceTimeout();
          await mockController.disableSolanaTimeout();
        }

        // should get new published data
        const resultAfterDisableTimeout = await queryLastPublishedData<any>(
          BINANCE_CONTRACT,
          context.client,
        );
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<BinanceData>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > resultAfterDisableTimeout.round;
          },
          30_000,
          2_000,
          true,
        );

        const resultAfterDisableTimeout2 = await queryLastPublishedData<any>(
          JUPITER_CONTRACT,
          context.client,
        );
        await waitFor(
          async () => {
            const checkResult = await queryLastPublishedData<BinanceData>(
              BINANCE_CONTRACT,
              context.client,
            );
            return checkResult.round > resultAfterDisableTimeout2.round;
          },
          30_000,
          2_000,
          true,
        );
      });
    });
  });
});
