# Oracle Consensus Package

This package provides a robust and configurable framework for establishing consensus among a set of messengers, particularly designed for use within CosmWasm smart contracts. It enables decentralized data aggregation and validation, ensuring that data used by the oracle system is agreed upon by a predefined set of participants.

## Features

*   **Configurable Consensus Parameters**: Define the number of participating messengers, the threshold of submissions required for consensus, the acceptable delta (in parts per million) for data values to be considered equal, and the length of each consensus round.
*   **Round-Based Consensus**: Data submission and consensus calculation are organized into distinct rounds, with mechanisms to manage round progression and identify when a round has passed.
*   **Oracle Data Submission**: Messengers can submit their data for a given round. The system prevents double submissions within the same round.
*   **Automated Consensus Calculation**: The package automatically attempts to form a consensus when a round concludes or when all participating messengers have submitted their data.
*   **Generic Data Support**: The core consensus logic is generic, allowing any data type that implements the `ConsensusData` trait (requiring `Serialize`, `DeserializeOwned`, and `Clone`) to be used for consensus.
*   **Specialized Numerical Consensus**: Includes a helper function (`consensus_on_items`) specifically designed for calculating consensus on `SignedDecimal256` values, finding the median of a statistically significant subset of submitted data.
*   **Error Handling**: Comprehensive error types are defined to cover various scenarios, including standard CosmWasm errors, double submissions, invalid round numbers, and arithmetic overflows.

## Core Components

### `Config`
Defines the parameters for the consensus mechanism:
*   `messengers`: A list of `Addr` (CosmWasm addresses) that are authorized to submit data.
*   `threshold`: The minimum number of oracle submissions required to reach a consensus.
*   `data_delta_ppm`: The maximum allowed percentage difference (in parts per million) between data points for them to be considered part of the same consensus group.
*   `round_length`: The duration of each consensus round in seconds.

### `Round`
Represents a specific consensus round, tracking its `round` number and `start` timestamp. It provides utility methods for checking round status and calculating future rounds.

### `ConsensusData` Trait
A generic trait that must be implemented by any data type intended for consensus. It includes the `try_consensus` method, which defines how a single consensus value is derived from a collection of submitted data points.

### `State`
Manages the persistent state of the consensus mechanism using `cw_storage_plus`. It stores the current `pending_round`, the `config`, `pending_data` (data submitted by messengers for the current round), and the `last_published_data` (the most recently agreed-upon consensus value).

### `publish_data` Function
The primary entry point for messengers to submit their data. This function handles:
*   Validating the submission (e.g., preventing double submissions).
*   Processing data for passed rounds.
*   Attempting to form consensus based on the `threshold` and `data_delta_ppm`.
*   Updating the `last_published_data` and advancing the `pending_round` when consensus is reached.

### `ConsensusOutcome`
A structure that encapsulates the data submitted by oracles (and for which the consensus was reached), including the `round` number for which the consensus was reached, `timestamp` when the consensus was reached, and the `data` itself.

### `consensus_on_items`
A utility function for `SignedDecimal256` values that implements a specific algorithm to find a consensus value. It sorts the submitted data and identifies the largest subset of values that fall within the `data_delta_ppm` range, then calculates the median of that subset.

### `ConsensusError`
An enum defining specific error conditions that can occur within the consensus process, such as `DoubleSubmission`, `InvalidRound`, and various arithmetic errors.

## Usage
This package is intended to be integrated into CosmWasm smart contracts that require a robust and decentralized method for aggregating and validating data from multiple oracle sources. By configuring the `Config` parameters, contract developers can tailor the consensus mechanism to their specific needs and security requirements.
