import cosmopark, { CosmoparkConfig } from '@neutron-org/cosmopark';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import {
  StargateClient,
} from '@cosmjs/stargate';
import { waitFor } from './helpers/waitFor';
import {
  CosmoparkNetworkConfig,
} from '@neutron-org/cosmopark/lib/types';
import { Suite } from 'vitest';

const keys = [
  'master',
  'hermes',
  'ibcrelayer',
  'demowallet1',
  'demowallet2',
  'neutronqueryrelayer',
  'demo1',
  'demo2',
  'demo3',
] as const;

const networkConfigs = {
  neutron: {
    "binary": "neutrond",
    "chain_id": "ntrntest",
    "denom": "untrn",
    "image": "neutron-test:0.0.1",
    "prefix": "neutron",
    "loglevel": "debug",
    "trace": true,
    "public": true,
    "validators": 1,
    "validators_balance": [
      "1900000000",
      "100000000"
    ],
    "upload": [
      "./artifacts/contracts",
      "./artifacts/contracts_thirdparty",
      "./artifacts/scripts/init-neutrond.sh"
    ],
    "post_init": [
      "CHAINID=ntrntest CHAIN_DIR=/opt /opt/init-neutrond.sh"
    ],
    "genesis_opts": {
      "app_state.auction.params.proposer_fee": "0.25",
      "app_state.bank.denom_metadata": [
        {
          "description": "The native staking token of the Neutron network",
          "denom_units": [
            {
              "denom": "untrn",
              "exponent": 0,
              "aliases": [
                "microntrn"
              ]
            },
            {
              "denom": "ntrn",
              "exponent": 6,
              "aliases": [
                "NTRN"
              ]
            }
          ],
          "base": "untrn",
          "display": "ntrn",
          "name": "Neutron",
          "symbol": "NTRN"
        }
      ],
      "app_state.contractmanager.params.sudo_call_gas_limit": "1000000",
      "app_state.cron.params.limit": 5,
      "app_state.feemarket.params.min_base_gas_price": "0.0025",
      "app_state.feemarket.params.max_learning_rate": "0.5",
      "app_state.feemarket.params.max_block_utilization": "1000000000",
      "app_state.feemarket.params.fee_denom": "untrn",
      "app_state.feemarket.params.enabled": false,
      "app_state.feemarket.params.distribute_fees": true,
      "app_state.feemarket.state.base_gas_price": "0.0025",
      "app_state.globalfee.params.minimum_gas_prices": [
        {
          "denom": "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
          "amount": "0"
        },
        {
          "denom": "untrn",
          "amount": "0"
        }
      ],
      "app_state.globalfee.params.bypass_min_fee_msg_types": [
        "/ibc.core.channel.v1.Msg/RecvPacket",
        "/ibc.core.channel.v1.Msg/Acknowledgement",
        "/ibc.core.client.v1.Msg/UpdateClient"
      ],
      "app_state.globalfee.params.max_total_bypass_min_fee_msg_gas_usage": "1000000",
      "app_state.marketmap.params.market_authorities": [
        "neutron1hxskfdxpp5hqgtjj6am6nkjefhfzj359x0ar3z"
      ],
      "app_state.marketmap.params.admin": "neutron1hxskfdxpp5hqgtjj6am6nkjefhfzj359x0ar3z",
      "app_state.slashing.params.signed_blocks_window": "140000",
      "app_state.slashing.params.min_signed_per_window": "0.050000000000000000",
      "app_state.slashing.params.slash_fraction_double_sign": "0.010000000000000000",
      "app_state.slashing.params.slash_fraction_downtime": "0.000100000000000000",
      "app_state.staking.params.bond_denom": "untrn",
      "consensus.params.block.max_gas": "1000000000",
      "consensus.params.abci.vote_extensions_enable_height": "1"
    },
    "config_opts": {
      "consensus.timeout_commit": "500ms",
      "consensus.timeout_propose": "500ms"
    },
    "app_opts": {
      "api.enable": "true",
      "api.address": "tcp://0.0.0.0:1317",
      "api.swagger": "true",
      "grpc.enable": "true",
      "grpc.address": "0.0.0.0:9090",
      "minimum-gas-prices": "0.0025untrn",
      "rosetta.enable": "true",
      "telemetry.prometheus-retention-time": 1000,
      "oracle.enabled": true,
      "oracle.oracle_address": "oracle:8080"
    }
  },
};

type Keys = (typeof keys)[number];

const awaitFirstBlock = (rpc: string): Promise<void> =>
  waitFor(async () => {
    try {
      const controller = new AbortController();
      setTimeout(() => controller.abort(), 1000);
      await fetch(rpc, {
        method: 'GET',
        signal: controller.signal,
      });
      const client = await StargateClient.connect(rpc);
      const block = await client.getBlock();
      if (block.header.height > 1) {
        return true;
      }
    } catch (e) {
      return false;
    }
  }, 50_000);

export const generateWallets = (): Promise<Record<Keys, string>> =>
  keys.reduce(
    async (acc, key) => {
      const accObj = await acc;
      const wallet = await DirectSecp256k1HdWallet.generate(12, {
        prefix: 'neutron',
      });
      accObj[key] = wallet.mnemonic;
      return accObj;
    },
    Promise.resolve({} as Record<Keys, string>),
  );

type NetworkOptsType = Partial<Record<keyof typeof networkConfigs | '*', any>>;
const getNetworkConfig = (
  network: string,
  opts: NetworkOptsType = {},
): CosmoparkNetworkConfig => {
  let config = networkConfigs[network];
  const extOpts = opts['*'] || opts[network] || {};
  for (const [key, value] of Object.entries(extOpts)) {
    if (typeof value === 'object') {
      config = { ...config, [key]: { ...config[key], ...value } };
    } else {
      config = { ...config, [key]: value };
    }
  }
  return config;
};

function isSuite(t: any): t is Suite {
  return t && t.type === 'suite' && t.suite;
}

export const setupPark = async (
  t: Readonly<Suite | File>,
  networks: string[] = [],
  opts?: NetworkOptsType, // Key is path to the param, value is Record of network name and value
): Promise<cosmopark> => {
  const context = ((t: Readonly<Suite | File>) => {
    if (isSuite(t)) {
      return t.suite.file.filepath
        ?.split('/')
        .pop()!
        .split('.')[0]
        .replace(/[-_]/g, '');
    } else {
      throw new Error('Invalid context');
    }
  })(t);
  console.log('generating wallets');
  const wallets = await generateWallets();
  console.log('wallets generated');
  const config: CosmoparkConfig = {
    context,
    networks: {},
    master_mnemonic: wallets.master,
    loglevel: 'info',
    wallets: {
      demo1: { mnemonic: wallets.demo1, balance: '1000000000' },
      demo2: { mnemonic: wallets.demo2, balance: '1000000000' },
      demo3: { mnemonic: wallets.demo3, balance: '1000000000' },
    },
    custom_containers: [
      {
        name: 'aum-messenger-1',
        image: `aum-messenger-test:0.0.1`,
        entrypoint: '/aum_messenger/aum-messenger --config /aum_messenger/config.yaml',
        ports: ['3001:3000'],
        depends_on: ['neutron_val1'],
        volumes: ['./aum-messenger-1-config.yaml:/aum_messenger/config.yaml'],
      },
      {
        name: 'aum-messenger-2',
        image: `aum-messenger-test:0.0.1`,
        entrypoint: '/aum_messenger/aum-messenger --config /aum_messenger/config.yaml',
        ports: ['3002:3000'],
        depends_on: ['neutron_val1'],
        volumes: ['./aum-messenger-2-config.yaml:/aum_messenger/config.yaml'],
      },
      {
        name: 'aum-messenger-3',
        image: `aum-messenger-test:0.0.1`,
        entrypoint: '/aum_messenger/aum-messenger --config /aum_messenger/config.yaml',
        ports: ['3003:3000'],
        depends_on: ['neutron_val1'],
        volumes: ['./aum-messenger-3-config.yaml:/aum_messenger/config.yaml'],
      },
    ],
  };
  for (const network of networks) {
    console.log('getting network config for', network);
    config.networks[network] = getNetworkConfig(network, opts);
  }
  console.log('creating cosmopark with config', JSON.stringify(config));
  console.log('it will take a couple of minutes to run a node...');
  const instance = await cosmopark.create(config);
  console.log('cosmopark created, waiting for first block');
  await Promise.all(
    Object.entries(instance.ports).map(([network, ports]) =>
      awaitFirstBlock(`http://127.0.0.1:${ports.rpc}`).catch((e) => {
        console.log(`Failed to await first block for ${network}: ${e}`);
        throw e;
      }),
    ),
  );
  return instance;
};
