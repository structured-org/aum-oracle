# binance-aum-oracle-contract

## Contract Functionality

The `binance-aum-receiver-contract` performs the following key functions:

### 1. Data Collection and Consensus

*   **Messenger Submissions:** Authorized messengers can submit `BinanceData` to the contract. This data includes detailed financial metrics such as:
    *   Unrealized Initial Margin Maintenance Ratio (`unimmr`)
    *   Binance positions (symbol, amount, PnL)
    *   USDⓈ-M Futures account balance in USDT (`um_balance_usdt`)
    *   Spot balances (asset, amount)
    *   Portfolio Margin account actual equity (`pm_account_actual_equity`)
    *   Withdrawable USDT (`withdrawable_usdt`)
*   **Data Validation:** Submitted data is rigorously validated to ensure it adheres to predefined requirements, including the presence of specific Binance positions and spot assets.
*   **Consensus Mechanism:** The contract employs a robust consensus mechanism. Multiple messenger submissions for the same data point are aggregated, and a consensus value is determined based on a configurable threshold and data delta. Only data that reaches consensus is considered valid and stored.

### 2. Configuration Management

*   **Instantiate:** Upon deployment, the contract is initialized with:
    *   An owner address.
    *   A list of authorized messenger addresses.
    *   Consensus parameters: threshold (number of messenger votes required for consensus), data delta (tolerance for value differences), and round length (duration for data submission rounds).
    *   Data validity periods for both consensus data and external price oracle data.
    *   Lists of required Binance positions and spot assets that messengers must report.
    *   The address of an external price oracle contract.
*   **Update Configuration:** The contract owner can update various configuration parameters, including the list of messengers, consensus settings, data validity periods, required assets/positions, and the price oracle contract address.

### 3. AUM Calculation and Querying

*   **AUM Calculation:** The contract calculates the total AUM in BTC. This involves:
    *   Retrieving the latest consensus-reached Binance data.
    *   Converting all spot balances from various assets (e.g., USDT, ETH) into their BTC equivalent using an external price oracle.
    *   Summing the converted spot balances with the Portfolio Margin account actual equity (also converted to BTC) to derive the total AUM in BTC.
*   **Data Querying:** Users can query the contract to:
    *   Retrieve the latest published `BinanceData` that achieved consensus.
    *   Get the calculated AUM in BTC.
    *   Obtain information about the current and next consensus rounds.
