# TWAER Contract

The **TWAER** (Time-Weighted Average Exchange Rate) contract calculates and maintains time-weighted average exchange rates for maxBTC tokens. It aggregates AUM (Assets Under Management) data from multiple oracle contracts and combines it with token supply information to compute exchange rates over configurable time windows.

The contract provides two main functionalities:
1. **Data Collection**: Frequently records instant exchange rates based on current AUM and supply data;
2. **Rate Publication**: Periodically publishes TWA exchange rates.

The TWA calculation uses the formula: `TWA = Σ(rate_i × duration_i) / Σ(duration_i)` where each rate is weighted by the time it was active.

## Execute Messages

### `RecordEr {}`

Calculates the current exchange rate based on AUM and supply data, stores it in the exchange rate history, and updates the internal TWA aggregator. Should be called frequently to maintain accurate time-weighted data.

**Permissions**: Permissionless

### `PublishTwaer {}`

Calculates and publishes a TWA exchange rate that can be retrieved via `GetTwaer` queries.

**Permissions**: Owner only

### `UpdateConfig { new_config }`

Allows modification of contract configuration. All fields are optional for partial updates.

**Permissions**: Owner only

## Query Messages

### `GetConfig {}`

Returns the contract's current configuration.

### `GetAum {}`

Queries all configured AUM oracles and returns the sum of their reported values.

### `GetTwaer {}`

Returns the published TWA exchange rate. The rate is only updated when `PublishTwaer` is explicitly called. Response includes the rate and publication timestamp.

### `PredictTwaer {}`

Calculates what the TWA exchange rate rate would be if `PublishTwaer` were called at this moment. Useful for decision-making about when to publish official rates.
