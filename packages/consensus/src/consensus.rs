use crate::consensus::ConsensusResult::ConsensusReached;
use cosmwasm_std::{Addr, Decimal, Env, SignedDecimal, StdError, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Describes the configuration of consensus
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    /// a list of oracles that can submit data for consensus
    pub oracles: Vec<Addr>,
    /// threshold of the consensus (how many oracles must submit data for consensus to be reached)
    pub threshold: u32,
    /// delta in percent per million (ppm), for which two values are considered equal
    pub data_delta_ppm: u64,
    /// length of a round in seconds
    pub round_length: u64,
}

/// Describes the round entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Copy)]
pub struct Round {
    /// a number of the round
    pub round: u64,
    /// when the round started (UNIX timestamp in seconds)
    pub start: u64,
}

impl Round {
    /// is round passed?
    pub fn is_passed(&self, env: &Env, round_length: u64) -> bool {
        self.start + round_length <= env.block.time.seconds()
    }

    /// how many rounds passed since the round started?
    pub fn rounds_passed(&self, env: &Env, round_length: u64) -> u64 {
        (env.block.time.seconds() - self.start) / round_length
    }

    /// Returns a new round after the current one based on two inputs:
    /// `round_length` - length of one round in seconds
    /// `rounds` - how many rounds to add to the current one
    /// The new round has the `start_time` equals to `self.start + rounds * round_length`
    /// and the `round` equals to `self.round + rounds`
    pub fn add_rounds(&self, round_length: u64, rounds: u64) -> Round {
        Round {
            round: self.round + rounds,
            start: self.start + rounds * round_length,
        }
    }

    /// Returns a new round after the current one based on one input:
    /// `round_length` - length of one round in seconds
    pub fn next_round(&self, round_length: u64) -> Round {
        Round {
            round: self.round + 1,
            start: self.start + round_length,
        }
    }
}

/// Describes a data you want to get a consensus for.
/// The structure must be Clonable, Serialized and Deserialized to store it in the CosmWasm storage.
pub trait ConsensusData: Serialize + DeserializeOwned + Clone {
    /// The method tries to form a single value from a slice of [ConsensusData] values based on `threshold` and `delta_ppm` from the Config.
    fn try_consensus(data: &[Self], threshold: usize, delta_ppm: u64) -> Option<Self>;
}

/// State of the consensus
pub struct State<T: ConsensusData> {
    /// the current pending round we are waiting data for
    pub pending_round: Item<Round>,
    /// the configuration of the consensus
    pub config: Item<Config>,
    /// the pending data for the current round
    pub pending_data: Map<Addr, OracleData<T>>,
    /// the last published data oracles agreed on
    pub last_published_data: Item<OracleData<T>>,
}

const PENDING_ROUND_KEY: &str = "consensus__pending_round";
const CONFIG_KEY: &str = "consensus__config";
const PENDING_DATA_KEY: &str = "consensus__pending_data";
const LAST_PUBLISHED_DATA_KEY: &str = "consensus__last_published_data";

impl<T: ConsensusData> State<T> {
    /// State constructor
    pub const fn default() -> Self {
        State {
            pending_round: Item::new(PENDING_ROUND_KEY),
            pending_data: Map::new(PENDING_DATA_KEY),
            last_published_data: Item::new(LAST_PUBLISHED_DATA_KEY),
            config: Item::new(CONFIG_KEY),
        }
    }

    /// Initialized storage values of the consensus State.
    /// Must be called only once during a contract instantiation
    pub fn initialize(
        &self,
        storage: &mut dyn Storage,
        env: &Env,
        config: Config,
    ) -> StdResult<()> {
        self.pending_round.save(
            storage,
            &Round {
                round: 0,
                start: env.block.time.seconds(),
            },
        )?;

        self.config.save(storage, &config)?;

        Ok(())
    }

    /// Returns all pending data
    fn get_all_pending_data(&self, storage: &dyn Storage) -> StdResult<Vec<OracleData<T>>> {
        let oracles = self.config.load(storage)?.oracles;

        let mut v = Vec::new();
        for addr in oracles {
            if let Some(d) = self.pending_data.may_load(storage, addr)? {
                v.push(d);
            }
        }
        Ok(v)
    }

    /// Returns the current pending round
    pub fn get_pending_round(&self, storage: &dyn Storage) -> StdResult<Round> {
        self.pending_round.load(storage)
    }

    /// Returns the last current data oracles agreed on
    /// If the pending round is passed, returns the consensus data for the current pending round
    /// Otherwise, returns the last published data from the storage
    pub fn get_last_published_data(
        &self,
        env: &Env,
        storage: &dyn Storage,
    ) -> StdResult<Option<OracleData<T>>> {
        let config = self.config.load(storage)?;

        let pending_round = self.pending_round.load(storage)?;

        // if pending round is passed, try to get consensus from the current pending data
        if pending_round.is_passed(env, config.round_length) {
            let pending = self.get_all_pending_data(storage)?;

            let data: Vec<T> = pending
                .iter()
                .map(|oracle_data| oracle_data.data.clone())
                .collect();

            if let Some(consensus) = ConsensusData::try_consensus(
                &data,
                config.threshold as usize,
                config.data_delta_ppm,
            ) {
                return Ok(Some(OracleData {
                    round: pending_round.round,
                    timestamp: env.block.time.seconds(),
                    data: consensus,
                }));
            }
        }

        // otherwise, we just return the last published data
        self.last_published_data.may_load(storage)
    }

    /// Publishes data for consensus.
    /// * If the pending round is passed, try to form a consensus for the current pending data and move to the next round;
    /// * If all oracles have submitted but the current round is not passed yet, try to form the consensus but not increase the round.
    ///
    /// An error is returned in the following cases:
    /// * an oracle tries to publish data for the same round more than ones;
    /// * an oracle tries to publish data for the past or future round;
    ///
    /// The method returns `ConsensusResult::ConsensusReached(OracleData<T>)` if the call
    /// and `ConsensusResult::ConsensusNotReached` in case it did not as the first argument
    /// and the current pending round as the second
    pub fn publish_data(
        &self,
        storage: &mut dyn Storage,
        env: &Env,
        oracle: Addr,
        new_data: OracleData<T>,
    ) -> StdResult<(ConsensusResult<T>, Round)> {
        let mut pending_round = self.pending_round.load(storage)?;

        let config = self.config.load(storage)?;

        let mut consensus_data = ConsensusResult::ConsensusNotReached;

        // if pending round is passed:
        // * process pending data for the passed round
        // * if consensus for the pending data is reached, publish it
        // * clear pending data
        if pending_round.is_passed(env, config.round_length) {
            let pending = self.get_all_pending_data(storage)?;

            let data: Vec<T> = pending
                .iter()
                .map(|oracle_data| oracle_data.data.clone())
                .collect();

            if let Some(consensus) = ConsensusData::try_consensus(
                &data,
                config.threshold as usize,
                config.data_delta_ppm,
            ) {
                let oracle_data = OracleData {
                    round: pending_round.round,
                    timestamp: env.block.time.seconds(),
                    data: consensus,
                };
                self.last_published_data.save(storage, &oracle_data)?;
                consensus_data = ConsensusReached(oracle_data);
            }

            pending_round = pending_round.add_rounds(
                config.round_length,
                pending_round.rounds_passed(env, config.round_length),
            );
            self.pending_round.save(storage, &pending_round)?;

            // Reset pending data
            self.pending_data.clear(storage);
        }

        // oracles can't publish data for already finalized rounds
        if let Some(last_data) = self.last_published_data.may_load(storage)? {
            if last_data.round >= new_data.round {
                return Err(StdError::generic_err("Invalid round"));
            }
        }

        // Only accept data for pending round
        if new_data.round != pending_round.round {
            return Err(StdError::generic_err("Invalid round"));
        }

        // Check if oracle has already submitted data for this round
        if self
            .pending_data
            .may_load(storage, oracle.clone())?
            .is_some()
        {
            return Err(StdError::generic_err(
                "Oracle has already submitted data for this round",
            ));
        }

        self.pending_data.save(
            storage,
            oracle.clone(),
            &OracleData {
                round: new_data.round,
                timestamp: env.block.time.seconds(),
                data: new_data.data,
            },
        )?;

        // Check if round is complete: either all oracles or round time expired
        let pending = self.get_all_pending_data(storage)?;

        // Try forming consensus, because we have all oracles published their data for a round
        if pending.len() == config.oracles.len() {
            let data: Vec<T> = pending
                .iter()
                .map(|oracle_data| oracle_data.data.clone())
                .collect();

            if let Some(consensus) = ConsensusData::try_consensus(
                &data,
                config.threshold as usize,
                config.data_delta_ppm,
            ) {
                let oracle_data = OracleData {
                    round: new_data.round,
                    timestamp: env.block.time.seconds(),
                    data: consensus,
                };
                self.last_published_data.save(storage, &oracle_data)?;
                consensus_data = ConsensusReached(oracle_data);
            }

            self.pending_data.clear(storage);
        }

        Ok((consensus_data, pending_round))
    }
}

/// Result of the `publish_data` method
pub enum ConsensusResult<T> {
    /// The consensus was reached, the first element is the data
    ConsensusReached(OracleData<T>),
    /// The consensus was not reached
    ConsensusNotReached,
}

/// Data submitted by an oracle
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct OracleData<T> {
    /// The round number an oracle tries to submit data for
    pub round: u64,
    /// The UNIX timestamp in seconds when the oracle submitted the data
    pub timestamp: u64,
    /// The data submitted by the oracle
    pub data: T,
}

/// A helper function that calculates consensus for a given array of SignedDecimals
// TODO: make it generic (not critical for now, but it would be nice to have)
pub fn consensus_on_items(
    items: &[SignedDecimal],
    threshold: usize,
    delta_ppm: u64,
) -> Option<SignedDecimal> {
    if items.len() < threshold {
        return None;
    }
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // Find largest sublice [i..j] such that sorted[j-1] - sorted[i] <= sorted[j-1] * data_delta_ppm / 1_000_000
    let ppm = Decimal::from_ratio(delta_ppm, 1_000_000u64);
    let mut max_len = 0;
    let mut best_slice = (0, 0);
    for i in 0..sorted.len() {
        for j in (i + threshold)..=sorted.len() {
            let low = sorted[i];
            let high = sorted[j - 1];

            // if |high - low| <= (max(|low|, |high|) * data_delta_ppm / 1_000_000) && j - i > max_len
            if high.abs_diff(low)
                <= low
                    .abs_diff(SignedDecimal::zero())
                    .max(high.abs_diff(SignedDecimal::zero()))
                    * ppm
                && j - i > max_len
            {
                max_len = j - i;
                best_slice = (i, j);
            }
        }
    }
    if max_len < threshold {
        return None;
    }
    let slice = &sorted[best_slice.0..best_slice.1];
    Some(median(slice))
}

pub fn consensus_on_items_u64(items: &[u64], threshold: usize, delta_ppm: u64) -> Option<u64> {
    if items.len() < threshold {
        return None;
    }
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // Find largest sublice [i..j] such that sorted[j-1] - sorted[i] <= sorted[j-1] * data_delta_ppm / 1_000_000
    // let ppm = Decimal::from_ratio(delta_ppm, 1_000_000u64);
    let mut max_len = 0;
    let mut best_slice = (0, 0);
    for i in 0..sorted.len() {
        for j in (i + threshold)..=sorted.len() {
            let low = sorted[i];
            let high = sorted[j - 1];

            // if |high - low| <= (max(|low|, |high|) * data_delta_ppm / 1_000_000) && j - i > max_len
            let low_high_abs_diff = low.abs_diff(0).max(high.abs_diff(0));
            if Uint128::new(high.abs_diff(low) as u128)
                <= (Decimal::from_ratio(low_high_abs_diff, delta_ppm)
                    / Decimal::from_atomics(Uint128::new(1_000_000), 0).ok()?)
                .to_uint_floor()
                && j - i > max_len
            {
                max_len = j - i;
                best_slice = (i, j);
            }
        }
    }
    if max_len < threshold {
        return None;
    }
    let slice = &sorted[best_slice.0..best_slice.1];
    Some(median_u64(slice))
}

pub fn consensus_on_items_uint128(
    items: &[Uint128],
    threshold: usize,
    delta_ppm: u64,
) -> Option<Uint128> {
    if items.len() < threshold {
        return None;
    }
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // Find largest sublice [i..j] such that sorted[j-1] - sorted[i] <= sorted[j-1] * data_delta_ppm / 1_000_000
    // let ppm = Decimal::from_ratio(delta_ppm, 1_000_000u64);
    let mut max_len = 0;
    let mut best_slice = (0, 0);
    for i in 0..sorted.len() {
        for j in (i + threshold)..=sorted.len() {
            let low = sorted[i];
            let high = sorted[j - 1];

            // if |high - low| <= (max(|low|, |high|) * data_delta_ppm / 1_000_000) && j - i > max_len
            let low_high_abs_diff = low
                .abs_diff(Uint128::zero())
                .max(high.abs_diff(Uint128::zero()));
            if high.abs_diff(low)
                <= (Decimal::from_ratio(low_high_abs_diff, delta_ppm)
                    / Decimal::from_atomics(Uint128::new(1_000_000), 0).ok()?)
                .to_uint_floor()
                && j - i > max_len
            {
                max_len = j - i;
                best_slice = (i, j);
            }
        }
    }
    if max_len < threshold {
        return None;
    }
    let slice = &sorted[best_slice.0..best_slice.1];
    Some(median_u128(slice))
}

// TODO: tests
// Utility function that returns item only if all items are the same
pub fn exact_consensus_on_items<T: Eq + Clone>(
    items: &[T],
    threshold: usize, // TODO: probably we dont need threshold here?
) -> Option<T> {
    if items.len() < threshold {
        return None;
    }
    let item = items.first()?;

    for a in items.iter() {
        if a != item {
            return None;
        }
    }

    Some(item.clone())
}

/// Utility function that calculates the median value of a slice of SignedDecimals
fn median(slice: &[SignedDecimal]) -> SignedDecimal {
    let n = slice.len();
    if n == 0 {
        return SignedDecimal::zero();
    }
    if n % 2 == 1 {
        slice[n / 2]
    } else {
        (slice[n / 2 - 1] + slice[n / 2]) / SignedDecimal::from_ratio(2, 1)
    }
}

fn median_u64(slice: &[u64]) -> u64 {
    let n = slice.len();
    if n == 0 {
        return 0;
    }
    if n % 2 == 1 {
        slice[n / 2]
    } else {
        (slice[n / 2 - 1] + slice[n / 2]) / 2
    }
}

fn median_u128(slice: &[Uint128]) -> Uint128 {
    let n = slice.len();
    if n == 0 {
        return Uint128::zero();
    }
    if n % 2 == 1 {
        slice[n / 2]
    } else {
        (slice[n / 2 - 1] + slice[n / 2]) / Uint128::new(2)
    }
}
