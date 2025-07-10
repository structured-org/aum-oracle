# Jupiter AUM Oracle Contract

A CosmWasm contract for consensus-based aggregation of Jupiter AUM data.
Authorized oracles submit structured AUM snapshots.
When enough submissions converge, the data is finalized and used to compute AUM in wBTC.

## Execute Messages

### `UpdateConfig`
Updates general and consensus configuration. Only callable by the contract owner. All fields are optional.

### `PublishData`
Allows a registered oracle to publish Jupiter AUM data.
If consensus is reached, the data is finalized and exposed via queries.

## Query Messages

### `Config`
Returns the current general and consensus configuration.

### `GetData`
Returns the most recently finalized AUM data.

### `GetAum`
Returns the latest valid Jupiter AUM in wBTC. Errors if no data or if data is stale.

### `GetRoundInfo`
Returns info on the current pending round and the upcoming round schedule.
