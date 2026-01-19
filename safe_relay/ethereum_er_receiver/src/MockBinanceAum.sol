// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

struct Position {
  bytes32 symbol;
  int256 amount; // SignedDecimal256: 18 decimals
  int256 pnl; // SignedDecimal256: 18 decimals
}

struct SpotBalance {
  bytes32 asset;
  int256 amount; // SignedDecimal256: 18 decimals
}

struct BinanceData {
  int256 unimmr; // SignedDecimal256: 18 decimals
  Position[] positions;
  int256 umBalanceUsdt; // SignedDecimal256: 18 decimals
  SpotBalance[] spotBalances;
  int256 pmAccountActualEquity; // SignedDecimal256: 18 decimals
  int256 withdrawableUsdt; // SignedDecimal256: 18 decimals
}

struct ConsensusOutcome {
  uint64 round;
  uint64 timestamp;
  bytes data;
}

contract MockBinanceAum {
  address publisher;
  uint64 private round;
  uint64 private timestamp;
  bytes private data;

  error NotPublisher();

  constructor(address _publisher) {
    publisher = _publisher;
  }

  modifier onlyPublisher() {
    if (msg.sender != publisher) revert NotPublisher();
    _;
  }

  function getData() external view returns (ConsensusOutcome memory, bool) {
    ConsensusOutcome memory co = ConsensusOutcome({
      round: round,
      timestamp: timestamp,
      data: data
    });
    return (co, true);
  }

  function setPublisher(address _publisher) external onlyPublisher {
    publisher = _publisher;
  }

  function setData(
    uint64 _round,
    uint64 _timestamp,
    BinanceData calldata _data
  ) external onlyPublisher {
    round = _round;
    timestamp = _timestamp;
    data = abi.encode(_data);
  }
}
