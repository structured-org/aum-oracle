# Oracle Messenger

Oracle Messenger is a Go daemon that serves as part of the Oracle system. Its responsibilities include retrieving information from other sources (off-chain data such as from Binance, and other-chain data such as Jupiter/Solana for Neutron) and submitting it to the on-chain Oracle Receiver contract.

## How to run

1. Copy `config.yaml.default` as `config.yaml`
2. Populate `config.yaml` with the respective values
3. Run `docker compose up aum_messenger -f ../docker-compose.yml`

## Configuration

- `binance_um_positions_list`: List of UM-positions to fetch from Binance, e.g. ["BTCUSDT", "ETHUSDT"], etc.
- `binance_spot_assets_list`: List of spot assets to fetch from Binance, e.g. ["USDT", "BTC"], etc.
- `jupiter_custodies`: Human readable asset name to custody program ID mapping of Jupiter custodies, e.g. {"WETH": "AQCGyheWPLeo6Qp9WpYS9m3Qj479t7R636N9ey1rEjEn"}
- `jupiter_jlp_pool`: Jupiter pool pubkey
- `solana_balances_list`: List of owner to assets mappings of Solana balances. Owners are represented as Solana addresses. Each asset is represented either as SPL token Mint address or SOL for balance in lamports. E.g. {"AYBzGpmCGLvLJQHAKEFZGBAm3R8yER7K4c3GZ2YgMEnf": ["27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4", "SOL"]}
- `solana_token_supply_list`: List of SPL token Mint addresses to fetch the total supply of, e.g. ["27G8MtK7VtTcCHkpASjSDdkWWYfoqT6ggEuKidVJidD4", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"]
- `clients`: Configuration for different clients
  - `neutron`: Neutron client configuration
    - `mnemonic`: Mnemonic phrase for the account
    - `gas_prices`: Gas prices for transactions
    - `gas_adjustment`: Gas adjustment multiplier
    - `chain_id`: Chain ID of the network
    - `node`: Node RPC endpoint
    - `node_conn_retries`: Number of retries for node connection
    - `node_conn_retry_delay`: Delay between node connection retries
  - `solana`: Solana client configuration
    - `rpc_endpoint`: RPC endpoint for Solana
  - `binance`: Binance client configuration
    - `api_key`: API key for the Binance API
    - `api_secret`: API secret for the Binance API
- `jupiter_aum_contract`: Jupiter AUM Oracle Receiver contract address
- `binance_aum_contract`: Binance AUM Oracle Receiver contract address
- `operational_config`: Configuration for the messenger's operational parameters
  - `failure_delay`: Delay taken when a messenger fails to fetch or submit data to prevent spamming
  - `pre_submit_delay`: Additional delay taken before submitting data to prevent submitting too soon due to possible time desync between messenger and chain
  - `fetch_data_timeout`: Timeout for fetch data operation
- `logger_level`: Level of the logger
- `mock_clients`: Whether the messenger should use mocked Solana, Jupiter and Binance clients or real ones
- `mock_controller_port`: Port of the mock controller server
