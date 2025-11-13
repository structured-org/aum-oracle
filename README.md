# AUM Oracle

A comprehensive oracle system for aggregating Assets Under Management (AUM) data from multiple sources including Binance and Jupiter/Solana. The system employs a consensus mechanism to ensure data reliability and accuracy.

## Project Overview

The AUM Oracle consists of multiple components working together to collect, validate, and provide AUM data:

- **Off-chain messengers** that collect data from external sources
- **CosmWasm contracts** that receive and process the data through consensus
- **Consensus packages** that implement robust data validation mechanisms
- **Integration tests** to ensure system reliability

## Project Structure

```
aum-oracle/
├── aum_messenger/           # Go daemon for data collection
├── recorder/                # GO daemon for periodic TWAER records creation in the TWAER contract
├── pkg/                     # Set of usefule packages that are being used by the daemons
├── packages/
│   ├── consensus/           # Rust consensus mechanism library
│   └── jupiter-aum-common/  # Common utilities for Jupiter AUM data
├── contracts/
│   ├── binance-aum-receiver/    # CosmWasm contract for Binance data
│   └── jupiter-aum-receiver/    # CosmWasm contract for Jupiter data
│   └── twaer/                   # CosmWasm contract for time weighted exchange rate calculations
├── integration_tests/       # End-to-end system tests
├── scripts/                # Deployment and utility scripts
├── Makefile                # Build and test automation
└── README.md               # This file
```

## Components

### 1. AUM Messenger (`aum_messenger/`)

A Go daemon that serves as the data collection layer of the Oracle system.

**Responsibilities:**
- Retrieve data from Binance (off-chain)
- Collect Jupiter/Solana data (other-chain)
- Submit data to on-chain Oracle Receiver contracts

**How to run:**
```bash
cd aum_messenger/
cp config.yaml.default config.yaml
# Edit config.yaml with your values
docker compose up
```

### 2. Recorder (`recorder/`)

A Go daemon for periodic TWAER records creation in the TWAER contract

**Responsibilities:**
- periodically execute '{"record_er": {}}' method in the TWAER contract.

**How to run:**
```bash
cd recorder/
cp config.yaml.default config.yaml
# Edit config.yaml with your values
docker compose up recorder -f ../docker-compose.yml
```

### 3. Consensus Package (`packages/consensus/`)

A robust Rust library providing consensus mechanisms for CosmWasm smart contracts.

**Features:**
- Configurable consensus parameters
- Round-based consensus with automatic progression
- Generic data support with specialized numerical consensus
- Comprehensive error handling

**Key Components:**
- `Config`: Defines consensus parameters (messengers, threshold, data delta, round length)
- `Round`: Manages consensus rounds and timing
- `ConsensusData` trait: Generic interface for consensus data types
- `State`: Persistent state management
- `publish_data`: Main entry point for data submission

**Testing:**
```bash
cargo test -p consensus
```

### 4. Jupiter AUM Common (`packages/jupiter-aum-common/`)

Common utilities and types for Jupiter AUM data processing.

**Testing:**
```bash
cargo test -p jupiter-aum-common
```

### 5. Binance AUM Receiver Contract (`contracts/binance-aum-receiver/`)

A CosmWasm contract for processing Binance AUM data through consensus.

**Functionality:**
- **Data Collection**: Receives BinanceData from authorized messengers
- **Consensus Mechanism**: Aggregates submissions and determines consensus values
- **AUM Calculation**: Calculates total AUM in BTC using external price oracles
- **Configuration Management**: Owner can update messengers, thresholds, and parameters

**Key Data Types:**
- Unrealized Initial Margin Maintenance Ratio (unimmr)
- Binance positions and PnL
- USDⓈ-M Futures account balance
- Spot balances across multiple assets
- Portfolio Margin account equity

**Testing:**
```bash
cargo test -p binance-aum-receiver
```

### 6. Jupiter AUM Receiver Contract (`contracts/jupiter-aum-receiver/`)

A CosmWasm contract for consensus-based aggregation of Jupiter AUM data.

**Functionality:**
- Receives structured AUM snapshots from authorized messengers
- Implements consensus mechanism for data validation
- Computes final AUM in wBTC
- Provides query interface for AUM data

**Execute Messages:**
- `UpdateConfig`: Update configuration (owner only)
- `PublishData`: Submit Jupiter AUM data (messengers)

**Query Messages:**
- `Config`: Get current configuration
- `GetData`: Get latest finalized data
- `GetAum`: Get latest AUM in wBTC
- `GetRoundInfo`: Get consensus round information

**Testing:**
```bash
cargo test -p jupiter-aum-receiver
```

### 7. Integration Tests (`integration_tests/`)

End-to-end tests for the entire AUM Oracle system.

See more info in the corresponding [folder](integration_tests/README.md)

### 8. Scripts (`scripts/`)

Utility scripts for deployment and contract interaction:
- `deploy_contracts.sh`: Deploy contracts to blockchain
- `query_contract.sh`: Query contract state and data

### 9. Go packages (`pkg/`)

Common helpers, structures and methods that are being used by the daemons.