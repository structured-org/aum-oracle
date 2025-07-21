import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import {
  CosmWasmClient,
  SigningCosmWasmClient,
} from '@cosmjs/cosmwasm-stargate';
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
import { waitFor } from '../helpers/waitFor';

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

const JUPITER_CONTRACT = 'TODO';

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

  describe('Consensus', async () => {
    const sampleCustodyPublicKey =
      '5Pv3gM9JrFFH883SWAhvJC9RPYmo8UNxuFtv5bMMALkm';
    const sampleTokenPublicKey = '11111111111111111111111111111111';
    const mockController1 = new MockController(3001);
    const mockController2 = new MockController(3002);
    const mockController3 = new MockController(3003);

    describe('Happy path', () => {
      it('publishes data as expected', async () => {
        // TODO: check current data
        // was: 1531381751507034
        const result = await queryLastPublishedData(
          JUPITER_CONTRACT,
          context.client,
        );
        expect(result.data.aum_usd).toEqual((1_531_381_751_507_034).toString());

        const data = await fetchMockData(mockController1);
        // let's change one property for consensus example
        data.poolInfo.aumUsd = (1_531_381_751_507_034 * 2).toString(); // let's double it
        const mockControllers = [
          mockController1,
          mockController2,
          mockController3,
        ];
        for (const mockController of mockControllers) {
          await mockController.setJupiterPoolInfo(data.poolInfo);
        }

        await waitFor(
          async () => {
            const result = await queryLastPublishedData(
              JUPITER_CONTRACT,
              context.client,
            );
            // wait until we have same changed aum usd gotten from contract
            return result.data.aum_usd === data.poolInfo.aumUsd;
          },
          20_000, // 20-second timeout
          600, // 0.6-second interval
        );
      });
    });

    describe('Malicious oracles', () => {});

    describe('Data sources timeouts and problems', () => {});
  });
});

// async function queryAum(client: CosmWasmClient): Promise<number> {
//
// }

async function queryLastPublishedData(
  contract: string,
  client: CosmWasmClient,
): Promise<LastPublishedData> {
  const res = await client.queryContractSmart(contract, {
    get_data: {},
  });
  return res.last_published_data;
}

class LastPublishedData {
  round: number;
  timestamp: number;
  data: SolanaData;
}

class SolanaData {
  custody_assets: any;
  aum_usd: string;
  total_jlp_supply: string;
  total_jlp_supply_decimals: number;
  strategy_jlp_balance: string;
  strategy_jlp_balance_decimals: number;
}
