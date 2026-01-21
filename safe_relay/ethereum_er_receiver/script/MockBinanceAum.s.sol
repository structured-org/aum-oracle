// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script, console2} from 'forge-std/Script.sol';
import {MockBinanceAum, BinanceData, Position, SpotBalance} from '../src/MockBinanceAum.sol';

contract MockBinanceAumDeploy is Script {
  function run() external {
    uint256 pk = vm.envUint('PRIVATE_KEY');
    address publisher = vm.envAddress('PUBLISHER');

    vm.startBroadcast(pk);
    MockBinanceAum mockBinanceAum = new MockBinanceAum(publisher);
    vm.stopBroadcast();

    console2.log('MockBinanceAum deployed at:', address(mockBinanceAum));

    Position[] memory positions = new Position[](4);
    positions[0] = Position({symbol: 'BNBUSDT', amount: -200, pnl: 186824336151});
    positions[1] = Position({symbol: 'BTCUSDT', amount: 56946, pnl: -309564864061083});
    positions[2] = Position({symbol: 'ETHUSDT', amount: -217162, pnl: 7557557504747});
    positions[3] = Position({symbol: 'SOLUSDT', amount: -2462759, pnl: 21765658623684});
    SpotBalance[] memory spotBalances = new SpotBalance[](6);
    spotBalances[0] = SpotBalance({asset: 'BNB', amount: 245761416});
    spotBalances[1] = SpotBalance({asset: 'BTC', amount: 10});
    spotBalances[2] = SpotBalance({asset: 'ETH', amount: 40});
    spotBalances[3] = SpotBalance({asset: 'SOL', amount: 1200});
    spotBalances[4] = SpotBalance({asset: 'USDT', amount: 5304});
    spotBalances[5] = SpotBalance({asset: 'WBTC', amount: 0});
    BinanceData memory binanceData = BinanceData({
      unimmr: 4008178305,
      positions: positions,
      umBalanceUsdt: -73948276537,
      spotBalances: spotBalances,
      pmAccountActualEquity: 371856611173986,
      withdrawableUsdt: 432970815153374
    });

    vm.startBroadcast(pk);
    mockBinanceAum.setData(9353, 1766079280, binanceData);
    vm.stopBroadcast();
  }
}
