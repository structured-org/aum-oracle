use crate::consensus::PublishResult::ConsensusReached;
use crate::error::{ConsensusError, ConsensusResult};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, Decimal, Decimal256, Env, Order, SignedDecimal256, StdResult, Storage, Uint128,
};
use cw_storage_plus::{Item, Map};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
use std::ops::{Add, Div, Sub};

/// Describes the configuration of consensus
#[cw_serde]
pub struct Config {
    /// a list of messengers that can submit data for consensus
    pub messengers: Vec<Addr>,
    /// threshold of the consensus (how many messengers must submit data for consensus to be reached)
    pub threshold: u32,
    /// delta in percent per million (ppm), for which two values are considered equal
    pub data_delta_ppm: u64,
    /// length of a round in seconds
    pub round_length: u64,
}

impl Config {
    /// Validates the configuration parameters.
    pub fn validate(&self) -> Result<(), ConsensusError> {
        if self.messengers.is_empty() {
            return Err(ConsensusError::InvalidMessengers {});
        }

        if self.threshold == 0 {
            return Err(ConsensusError::ZeroThreshold {});
        }

        if (self.threshold as usize) > self.messengers.len() {
            return Err(ConsensusError::LargeThreshold {});
        }

        Ok(())
    }
}

/// Describes the round entity
#[cw_serde]
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

    /// Returns a new round after the current one based on one input:
    /// `round_length` - length of one round in seconds
    pub fn next_round(&self, round_length: u64) -> Round {
        Round {
            round: self.round + 1,
            start: self.start + round_length,
        }
    }
}

/// Describes the data you want to get a consensus for.
/// The structure must be Clonable, Serialized and Deserialized to store it in the CosmWasm storage.
pub trait ConsensusData<O>: Serialize + DeserializeOwned + Clone {
    /// The method sorts and cleans incoming data, and checks for all required fields present.
    fn prepublish_cleanup(&mut self, options: O) -> Result<(), ConsensusError>;
    /// The method tries to form a single value from a slice of [ConsensusData] values based on `threshold` and `delta_ppm` from the Config.
    fn try_consensus(data: &[Self], threshold: usize, delta_ppm: u64) -> Option<Self>;
}

/// State of the consensus
pub struct State<T: ConsensusData<O>, O> {
    /// the current pending round we are waiting data for
    pub pending_round: Item<Round>,
    /// the configuration of the consensus
    pub config: Item<Config>,
    /// the pending configuration for the next round that will be applied on round switch
    pub pending_config: Item<Config>,
    /// the last published data messengers agreed on
    pub last_published_data: Item<ConsensusOutcome<T>>,
    /// the pending data for the current round
    pending_data: Map<Addr, T>,
    /// necessary to allow trait constraint
    #[allow(dead_code)]
    phantom_data: Option<PhantomData<O>>,
}

const PENDING_ROUND_KEY: &str = "consensus__pending_round";
const CONFIG_KEY: &str = "consensus__config";
const PENDING_DATA_KEY: &str = "consensus__pending_data";
const LAST_PUBLISHED_DATA_KEY: &str = "consensus__last_published_data";
const PENDING_CONFIG_KEY: &str = "consensus__pending_config";

impl<T: ConsensusData<O>, O> State<T, O> {
    /// State constructor
    pub const fn default() -> Self {
        State {
            pending_round: Item::new(PENDING_ROUND_KEY),
            pending_data: Map::new(PENDING_DATA_KEY),
            last_published_data: Item::new(LAST_PUBLISHED_DATA_KEY),
            config: Item::new(CONFIG_KEY),
            pending_config: Item::new(PENDING_CONFIG_KEY),
            phantom_data: None,
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

    /// Returns all pending data as a Vec<T>
    fn get_all_pending_data(&self, storage: &dyn Storage) -> StdResult<Vec<T>> {
        let mut v: Vec<T> = Vec::new();
        for data in self
            .pending_data
            .range(storage, None, None, Order::Ascending)
        {
            v.push(data?.1);
        }

        Ok(v)
    }

    /// Returns the current pending round
    pub fn get_pending_round(&self, storage: &dyn Storage) -> StdResult<Round> {
        self.pending_round.load(storage)
    }

    /// Saves the new consensus configuration
    /// New configuration will be applied only on **the next round**
    /// This mechanism guarantees all messengers use the same config and no consensus changes
    /// can happen in the middle of a round unexpectedly
    pub fn save_config(&self, storage: &mut dyn Storage, new_config: Config) -> StdResult<()> {
        self.pending_config.save(storage, &new_config)
    }

    /// Returns the last consensus data messengers agreed on
    /// If the pending round is passed, returns the consensus data for the current pending round
    /// Otherwise, returns the last published data from the storage
    pub fn get_last_published_data(
        &self,
        env: &Env,
        storage: &dyn Storage,
    ) -> StdResult<Option<ConsensusOutcome<T>>> {
        let config = self.config.load(storage)?;

        let pending_round = self.pending_round.load(storage)?;

        // if pending round is passed, try to get consensus from the current pending data
        if pending_round.is_passed(env, config.round_length) {
            let pending = self.get_all_pending_data(storage)?;

            if let Some(consensus) = ConsensusData::try_consensus(
                &pending,
                config.threshold as usize,
                config.data_delta_ppm,
            ) {
                return Ok(Some(ConsensusOutcome {
                    round: pending_round.round,
                    timestamp: pending_round.start + config.round_length,
                    data: consensus,
                }));
            }
        }

        // otherwise, we just return the last published data
        self.last_published_data.may_load(storage)
    }

    /// Publishes data for consensus.
    /// * If the pending round is passed, try to form a consensus for the current pending data and move to the next round;
    /// * If all messengers have submitted but the current round is not passed yet, try to form the consensus but not increase the round.
    ///
    /// An error is returned in the following cases:
    /// * a messenger tries to publish data for the same round more than once;
    /// * a messenger tries to publish data for the past or future round;
    ///
    /// The method returns `PublishResult::ConsensusReached(ConsensusOutcome<T>)` if the call
    /// and `PublishResult::ConsensusNotReached` in case it did not as the first argument
    /// and the current pending round as the second
    pub fn publish_data(
        &self,
        storage: &mut dyn Storage,
        env: &Env,
        messenger: Addr,
        mut new_data: T,
        options: O,
    ) -> ConsensusResult<(PublishResult<T>, Round)> {
        let mut pending_round = self.pending_round.load(storage)?;

        let config = self.config.load(storage)?;

        ConsensusData::prepublish_cleanup(&mut new_data, options)?;

        let mut pub_res = PublishResult::ConsensusNotReached;

        // if the pending round is passed:
        // * process pending data for the passed round
        // * move to the next round
        // * update config (optional)
        if pending_round.is_passed(env, config.round_length) {
            let pending_data = self.get_all_pending_data(storage)?;
            pub_res = self.finalize_round_data(
                pending_data,
                pending_round.clone(),
                config.clone(),
                env,
                storage,
            )?;

            pending_round = Round {
                round: pending_round.round + 1,
                start: env.block.time.seconds(),
            };
            self.pending_round.save(storage, &pending_round)?;

            // if there is some pending config, we need to write to the main config storage on round switch
            if let Some(pending_config) = self.pending_config.may_load(storage)? {
                self.config.save(storage, &pending_config)?;
                // clear pending config
                self.pending_config.remove(storage);
            }
        }

        // Check if messenger has already submitted data for this round
        if self
            .pending_data
            .may_load(storage, messenger.clone())?
            .is_some()
        {
            return Err(ConsensusError::DoubleSubmission {});
        }

        self.pending_data
            .save(storage, messenger.clone(), &new_data)?;

        let pending_data = self.get_all_pending_data(storage)?;

        // Try forming consensus if we have all messengers published their data for the round
        if pending_data.len() == config.messengers.len() {
            pub_res = self.finalize_round_data(
                pending_data,
                pending_round.clone(),
                config,
                env,
                storage,
            )?;
        }

        Ok((pub_res, pending_round))
    }

    /// Tries to form a consensus for the pending data and publishes it if consensus is reached.
    /// Regardless of whether consensus is reached or not, the pending data is cleared.
    fn finalize_round_data(
        &self,
        pending_data: Vec<T>,
        round: Round,
        config: Config,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> ConsensusResult<PublishResult<T>> {
        let mut res = PublishResult::ConsensusNotReached;

        if let Some(consensus) = ConsensusData::try_consensus(
            &pending_data,
            config.threshold as usize,
            config.data_delta_ppm,
        ) {
            let oracle_data = ConsensusOutcome {
                round: round.round,
                timestamp: env.block.time.seconds(),
                data: consensus,
            };
            self.last_published_data.save(storage, &oracle_data)?;
            res = ConsensusReached(oracle_data);
        }

        self.pending_data.clear(storage);

        Ok(res)
    }
}

/// Result of the `publish_data` method
pub enum PublishResult<T> {
    /// The consensus was reached for the ConsensusOutcome<T> data
    ConsensusReached(ConsensusOutcome<T>),
    /// The consensus was not reached
    ConsensusNotReached,
}

/// The outcome of the consensus algorithm for data with type T
#[cw_serde]
pub struct ConsensusOutcome<T> {
    /// The round number when the consensus was reached
    pub round: u64,
    /// The UNIX timestamp in seconds when the consensus was reached
    pub timestamp: u64,
    /// The data submitted by the messengers
    pub data: T,
}

// Single field consensus
pub fn consensus_on_field<F, T, O>(
    data: &[T],
    extract: F,
    threshold: usize,
    delta_ppm: u64,
) -> Option<SignedDecimal256>
where
    F: Fn(&T) -> SignedDecimal256,
    T: ConsensusData<O>,
{
    let items: Vec<SignedDecimal256> = data.iter().map(&extract).collect();
    consensus_on_items_dec256(&items, threshold, delta_ppm)
}

/// A helper function that calculates consensus for a given array of SignedDecimal256s
pub fn consensus_on_items_dec256(
    items: &[SignedDecimal256],
    threshold: usize,
    delta_ppm: u64,
) -> Option<SignedDecimal256> {
    let ppm = Decimal256::from_ratio(delta_ppm, 1_000_000u64);
    consensus_on_items(
        items,
        threshold,
        |high, low| {
            // |high - low| <= (max(|low|, |high|) * data_delta_ppm / 1_000_000)
            let diff = high.abs_diff(low);
            let max_dispersion = low
                .abs_diff(SignedDecimal256::zero())
                .max(high.abs_diff(SignedDecimal256::zero()))
                * ppm;
            Some(diff <= max_dispersion)
        },
        SignedDecimal256::from_atomics(2, 0).ok()?,
    )
}

pub fn consensus_on_items_u64(items: &[u64], threshold: usize, delta_ppm: u64) -> Option<u64> {
    let ppm = Decimal::from_ratio(delta_ppm, 1_000_000u64);
    consensus_on_items(
        items,
        threshold,
        |high, low| {
            let diff = Uint128::new(high.abs_diff(low) as u128);
            let decimal_high = Decimal::from_atomics(high, 0).ok()?;
            let max_dispersion = (decimal_high * ppm).to_uint_floor();
            Some(diff <= max_dispersion)
        },
        2u64,
    )
}

pub fn consensus_on_items_uint128(
    items: &[Uint128],
    threshold: usize,
    delta_ppm: u64,
) -> Option<Uint128> {
    let ppm = Decimal::from_ratio(delta_ppm, 1_000_000u64);
    consensus_on_items(
        items,
        threshold,
        |high, low| {
            let diff = high.abs_diff(low);
            let decimal_high = Decimal::from_atomics(high, 0).ok()?;
            let max_dispersion = (decimal_high * ppm).to_uint_floor();
            Some(diff <= max_dispersion)
        },
        Uint128::new(2),
    )
}

// Utility function that returns item only if all items are the same
pub fn all_items_equal<T: Eq + Clone>(items: &[T]) -> Option<T> {
    let item = items.first()?;

    for a in items.iter() {
        if a != item {
            return None;
        }
    }

    Some(item.clone())
}

pub fn consensus_on_items<T, F>(
    items: &[T],
    threshold: usize,
    inside_ppm_bounds: F,
    two: T, // 2 in T type
) -> Option<T>
where
    T: Eq + PartialOrd + Copy + Clone + Add<Output = T> + Sub<Output = T> + Div<Output = T>,
    F: Fn(T, T) -> Option<bool>,
{
    if items.len() < threshold {
        return None;
    }
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // Find largest subslice [i..j] such that sorted[j-1] - sorted[i] <= sorted[j-1] * data_delta_ppm / 1_000_000
    let mut max_len = 0;
    let mut best_slice = (0, 0);
    'outer: for i in 0..sorted.len() {
        // iterate in reverse, so we could find the largest faster
        for j in ((i + threshold)..=sorted.len()).rev() {
            let low = sorted[i];
            let high = sorted[j - 1];

            if inside_ppm_bounds(high, low)? && j - i > max_len {
                max_len = j - i;
                best_slice = (i, j);
                // we found the largest slice, we can exit
                break 'outer;
            }
        }
    }
    if max_len < threshold {
        return None;
    }
    let slice = &sorted[best_slice.0..best_slice.1];
    median(slice, two)
}

/// Utility function that calculates the generic median value of a SignedDecimals slice
pub fn median<T>(slice: &[T], two: T) -> Option<T>
where
    T: Copy + Clone + Add<Output = T> + Sub<Output = T> + Div<Output = T>,
{
    let n = slice.len();
    if n == 0 {
        return None;
    }
    let res = if n % 2 == 1 {
        slice[n / 2]
    } else {
        (slice[n / 2 - 1] + slice[n / 2]) / two
    };

    Some(res)
}
