import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';

export const JUPITER_CONTRACT =
  'neutron1t2tcfr92kgh6j3vg7e70csrgwpwqtdcg2m0umvya6922xype5trqdn6ahj';
export const BINANCE_CONTRACT =
  'neutron1u6shqnk4k5fza2exaet4gyczc4td8drnnclgu0ngn00lurmlu3tqpwjugm';

export async function queryAum(
  contract: string,
  client: CosmWasmClient,
): Promise<number> {
  const res = await client.queryContractSmart(contract, { get_aum: {} });
  return +res.aum_in_wbtc;
}

export async function queryLastPublishedData<T>(
  contract: string,
  client: CosmWasmClient,
): Promise<ConsensusOutcome<T>> {
  const res = await client.queryContractSmart(contract, {
    get_data: {},
  });
  console.log('queryLastPublishedData: ' + JSON.stringify(res));
  return res.last_published_data;
}

export class ConsensusOutcome<T> {
  round: number;
  timestamp: number;
  data: T;
}

export class SolanaData {
  custody_assets: any;
  aum_usd: string;
  total_jlp_supply: string;
  total_jlp_supply_decimals: number;
  strategy_jlp_balance: string;
  strategy_jlp_balance_decimals: number;
}

export class BinanceData {
  unimmr: string;
  positions: any[];
  um_balance_usdt: string;
  spot_balances: any[];
  pm_account_actual_equity: string;
  withdrawable_usdt: string;
}
