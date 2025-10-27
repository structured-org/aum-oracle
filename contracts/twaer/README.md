# TWAER Contract

The **TWAER** (Time-Weighted Average Exchange Rate) contract calculates and maintains time-weighted average exchange rates for maxBTC tokens. It aggregates AUM (Assets Under Management) data from multiple oracle contracts and combines it with token supply information to compute exchange rates over configurable time windows.

The contract provides two main functionalities:
1. **Data Collection**: Frequently records instant exchange rates based on current AUM and supply data;
2. **Rate Publication**: Periodically publishes TWA exchange rates.

The TWA calculation uses the formula: `TWA = Σ(rate_i × duration_i) / Σ(duration_i)` where each rate is weighted by the time it was active.

## Execute Messages

### `RecordEr {}`

Calculates the current exchange rate based on AUM and supply data, stores it in the exchange rate history, and updates the internal TWA aggregator. Should be called frequently to maintain accurate time-weighted data.

**Permissions**: Owner or recorder

### `PublishTwaer {}`

Calculates and publishes a TWA exchange rate that can be retrieved via `GetTwaer` queries. Publication is rate-limited by the `twaer_immutability_seconds` configuration parameter.

**Permissions**: Owner or publisher

### `UpdateConfig { new_config }`

Allows modification of contract configuration. All fields are optional for partial updates.

**Permissions**: Owner only

### `ResetTwaerTo { value }`

Resets the historical and aggregator values and sets the TWAER to a specific value. This clears all exchange rate history and starts fresh with the provided rate.

**Permissions**: Owner only

### `SetMockedMaxbtcSupply { value }`

Sets the mocked maxBTC supply which is used if the real token supply is not available (before token minting). When mocked supply is set, the contract uses this value instead of querying the actual token supply.

**Permissions**: Owner only

### `RemoveERDatapoint { er_timestamp }`

Removes a specific exchange rate datapoint from history and updates the TWA aggregator accordingly.

**Permissions**: Owner only

### `Unmock {}`

Removes the mocked supply from the contract. After calling this, the contract will use the real supply of maxBTC tokens from the blockchain.

**Permissions**: Owner only

## Query Messages

### `GetConfig {}`

Returns the contract's current configuration.

### `GetAum {}`

Queries all configured AUM oracles and returns the sum of their reported values.

### `GetTwaer {}`

Returns the published TWA exchange rate. The rate is only updated when `PublishTwaer` is explicitly called. Response includes the rate and publication timestamp.

### `PredictTwaer {}`

Calculates what the TWA exchange rate would be if `PublishTwaer` were called at this moment. This provides a real-time preview based on current exchange rate history without actually publishing the rate.

### `ErWindowInfo {}`

Returns information about the exchange rate history window, including the window start/end timestamps, total number of data points, and the actual data points themselves.

### `Ownership {}`

Returns the current ownership information of the contract (from `cw_ownable`).
