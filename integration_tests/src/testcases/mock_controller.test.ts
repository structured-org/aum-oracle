import { describe, expect, it, beforeAll, afterAll } from 'vitest';
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

describe('Mock controller', () => {
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

  describe('Mock Controller Integration Tests', () => {
    const sampleCustodyPublicKey =
      '5Pv3gM9JrFFH883SWAhvJC9RPYmo8UNxuFtv5bMMALkm';
    const sampleTokenPublicKey = '11111111111111111111111111111111';
    const mockController = new MockController(3001);

    const initialData: {
      umPositions?: BinanceUMPosition[];
      pmAccountInfo?: BinancePmAccountInfo;
      pmAccountBalance?: BinancePmAccountBalance[];
      spotAccountInfo?: BinanceSpotAccountInfo;
      custodyInfo?: JupiterPerpsCustodyAccount;
      poolInfo?: JupiterPoolAccount;
      tokenSupply?: SolanaUiTokenAmount;
      tokenAccountBalance?: SolanaUiTokenAmount;
    } = {};

    describe('GET - Fetch initial mock data', () => {
      it('should fetch initial Binance UM positions', async () => {
        const umPositions = await mockController.getBinanceUmPositions();
        expect(umPositions).toBeTruthy();
        initialData.umPositions = umPositions;
      });

      it('should fetch initial Binance PM account info', async () => {
        const pmAccountInfo = await mockController.getBinancePMAccountInfo();
        expect(pmAccountInfo).toBeTruthy();
        initialData.pmAccountInfo = pmAccountInfo;
      });

      it('should fetch initial Binance PM account balance', async () => {
        const pmAccountBalance =
          await mockController.getBinancePMAccountBalance();
        expect(pmAccountBalance).toBeTruthy();
        initialData.pmAccountBalance = pmAccountBalance;
      });

      it('should fetch initial Binance spot account info', async () => {
        const spotAccountInfo =
          await mockController.getBinanceSpotAccountInfo();
        expect(spotAccountInfo).toBeTruthy();
        initialData.spotAccountInfo = spotAccountInfo;
      });

      it('should fetch initial Jupiter perps custody info', async () => {
        const custodyInfo = await mockController.getJupiterPerpsCustodyInfo(
          sampleCustodyPublicKey,
        );
        expect(custodyInfo).toBeTruthy();
        initialData.custodyInfo = custodyInfo;
      });

      it('should fetch initial Jupiter pool info', async () => {
        const poolInfo = await mockController.getJupiterPoolInfo();
        expect(poolInfo).toBeTruthy();
        initialData.poolInfo = poolInfo;
      });

      it('should fetch initial Solana token supply', async () => {
        const tokenSupply = await mockController.getSolanaTokenSupply();
        expect(tokenSupply).toBeTruthy();
        initialData.tokenSupply = tokenSupply;
      });

      it('should fetch initial Solana token account balance', async () => {
        const tokenAccountBalance =
          await mockController.getSolanaTokenAccountBalance(
            sampleTokenPublicKey,
          );
        expect(tokenAccountBalance).toBeTruthy();
        initialData.tokenAccountBalance = tokenAccountBalance;
      });
    });

    const modifiedData: {
      umPositions?: BinanceUMPosition[];
      pmAccountInfo?: BinancePmAccountInfo;
      pmAccountBalance?: BinancePmAccountBalance[];
      spotAccountInfo?: BinanceSpotAccountInfo;
      custodyInfo?: JupiterPerpsCustodyAccount;
      poolInfo?: JupiterPoolAccount;
      tokenSupply?: SolanaUiTokenAmount;
      tokenAccountBalance?: SolanaUiTokenAmount;
    } = {};

    describe('POST - Set modified mock data', () => {
      beforeAll(() => {
        modifiedData.umPositions = initialData.umPositions.map((position) => ({
          ...position,
          positionAmt: (parseFloat(position.positionAmt) + 0.5).toString(),
          entryPrice: (parseFloat(position.entryPrice) + 100).toString(),
          markPrice: (parseFloat(position.markPrice) + 200).toString(),
          unrealizedProfit: (
            parseFloat(position.unrealizedProfit) + 50
          ).toString(),
          leverage: (parseInt(position.leverage) + 1).toString(),
          updateTime: Date.now(),
        }));

        modifiedData.pmAccountInfo = {
          ...initialData.pmAccountInfo,
          accountEquity: (
            parseFloat(initialData.pmAccountInfo.accountEquity) + 5000
          ).toString(),
          actualEquity: (
            parseFloat(initialData.pmAccountInfo.actualEquity) + 4800
          ).toString(),
          accountInitialMargin: (
            parseFloat(initialData.pmAccountInfo.accountInitialMargin) + 1000
          ).toString(),
          totalAvailableBalance: (
            parseFloat(initialData.pmAccountInfo.totalAvailableBalance) + 3800
          ).toString(),
          updateTime: Date.now(),
        };

        modifiedData.pmAccountBalance = initialData.pmAccountBalance.map(
          (balance) => ({
            ...balance,
            totalWalletBalance: (
              parseFloat(balance.totalWalletBalance) + 1000
            ).toString(),
            crossMarginFree: (
              parseFloat(balance.crossMarginFree) + 800
            ).toString(),
            umWalletBalance: (
              parseFloat(balance.umWalletBalance) + 500
            ).toString(),
            umUnrealizedPNL: (
              parseFloat(balance.umUnrealizedPNL) + 100
            ).toString(),
            updateTime: Date.now(),
          }),
        );

        modifiedData.spotAccountInfo = {
          ...initialData.spotAccountInfo,
          makerCommission: initialData.spotAccountInfo.makerCommission + 1,
          takerCommission: initialData.spotAccountInfo.takerCommission + 1,
          balances: initialData.spotAccountInfo.balances.map((balance) => ({
            ...balance,
            free: (parseFloat(balance.free) + 10).toString(),
            locked: (parseFloat(balance.locked) + 1).toString(),
          })),
          updateTime: Date.now(),
        };

        modifiedData.custodyInfo = {
          ...initialData.custodyInfo,
          decimals: initialData.custodyInfo.decimals + 1,
          targetRatioBps: initialData.custodyInfo.targetRatioBps + 100,
          assets: {
            ...initialData.custodyInfo.assets,
            feesReserves: initialData.custodyInfo.assets.feesReserves + 100000,
            owned: initialData.custodyInfo.assets.owned + 1000000,
            locked: initialData.custodyInfo.assets.locked + 500000,
          },
          pricing: {
            ...initialData.custodyInfo.pricing,
            maxLeverage: initialData.custodyInfo.pricing.maxLeverage + 1000,
            swapSpread: initialData.custodyInfo.pricing.swapSpread + 10,
          },
        };

        modifiedData.poolInfo = {
          ...initialData.poolInfo,
          name: initialData.poolInfo.name + ' (Modified)',
          aumUsd: (
            BigInt(initialData.poolInfo.aumUsd) +
            BigInt('1000000000000000000000000')
          ).toString(),
          fees: {
            ...initialData.poolInfo.fees,
            swapBps: initialData.poolInfo.fees.swapBps + 5,
            taxBps: initialData.poolInfo.fees.taxBps + 3,
            protocolShareBps: initialData.poolInfo.fees.protocolShareBps + 100,
          },
          poolApr: {
            ...initialData.poolInfo.poolApr,
            feeAprBps: initialData.poolInfo.poolApr.feeAprBps + 50,
            realizedFeeUsd:
              initialData.poolInfo.poolApr.realizedFeeUsd + 1000000,
            lastUpdated: Date.now(),
          },
        };

        modifiedData.tokenSupply = {
          ...initialData.tokenSupply,
          amount: (
            BigInt(initialData.tokenSupply.amount) + BigInt('1000000000')
          ).toString(),
        };

        modifiedData.tokenAccountBalance = {
          ...initialData.tokenAccountBalance,
          amount: (
            BigInt(initialData.tokenAccountBalance.amount) + BigInt('500000000')
          ).toString(),
        };
      });

      it('should set modified Binance UM positions', async () => {
        await mockController.setBinanceUmPositions(modifiedData.umPositions);
      });

      it('should set modified Binance PM account info', async () => {
        await mockController.setBinancePMAccountInfo(
          modifiedData.pmAccountInfo,
        );
      });

      it('should set modified Binance PM account balance', async () => {
        await mockController.setBinancePMAccountBalance(
          modifiedData.pmAccountBalance,
        );
      });

      it('should set modified Binance spot account info', async () => {
        await mockController.setBinanceSpotAccountInfo(
          modifiedData.spotAccountInfo,
        );
      });

      it('should set modified Jupiter perps custody info', async () => {
        await mockController.setJupiterPerpsCustodyInfo(
          sampleCustodyPublicKey,
          modifiedData.custodyInfo,
        );
      });

      it('should set modified Jupiter pool info', async () => {
        await mockController.setJupiterPoolInfo(modifiedData.poolInfo);
      });

      it('should set modified Solana token supply', async () => {
        await mockController.setSolanaTokenSupply(modifiedData.tokenSupply);
      });

      it('should set modified Solana token account balance', async () => {
        await mockController.setSolanaTokenAccountBalance(
          sampleTokenPublicKey,
          modifiedData.tokenAccountBalance,
        );
      });
    });

    describe('GET - Verify modified data persistence', () => {
      it('should verify modified Binance UM positions', async () => {
        const updatedUmPositions = await mockController.getBinanceUmPositions();
        expect(updatedUmPositions).toEqual(modifiedData.umPositions);
      });

      it('should verify modified Binance PM account info', async () => {
        const updatedPmAccountInfo =
          await mockController.getBinancePMAccountInfo();
        expect(updatedPmAccountInfo).toEqual(modifiedData.pmAccountInfo);
      });

      it('should verify modified Binance PM account balance', async () => {
        const updatedPmAccountBalance =
          await mockController.getBinancePMAccountBalance();
        expect(updatedPmAccountBalance).toEqual(modifiedData.pmAccountBalance);
      });

      it('should verify modified Binance spot account info', async () => {
        const updatedSpotAccountInfo =
          await mockController.getBinanceSpotAccountInfo();
        expect(updatedSpotAccountInfo).toEqual(modifiedData.spotAccountInfo);
      });

      it('should verify modified Jupiter perps custody info', async () => {
        const updatedCustodyInfo =
          await mockController.getJupiterPerpsCustodyInfo(
            sampleCustodyPublicKey,
          );
        expect(updatedCustodyInfo).toEqual(modifiedData.custodyInfo);
      });

      it('should verify modified Jupiter pool info', async () => {
        const updatedPoolInfo = await mockController.getJupiterPoolInfo();
        expect(updatedPoolInfo).toEqual(modifiedData.poolInfo);
      });

      it('should verify modified Solana token supply', async () => {
        const updatedTokenSupply = await mockController.getSolanaTokenSupply();
        expect(updatedTokenSupply).toEqual(modifiedData.tokenSupply);
      });

      it('should verify modified Solana token account balance', async () => {
        const updatedTokenBalance =
          await mockController.getSolanaTokenAccountBalance(
            sampleTokenPublicKey,
          );
        expect(updatedTokenBalance).toEqual(modifiedData.tokenAccountBalance);
      });
    });
  });
});
