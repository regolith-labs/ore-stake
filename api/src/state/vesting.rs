use serde::{Deserialize, Serialize};
use steel::*;

use super::{OreAccount, Treasury, ONE_HOUR};

/// Vesting account tracks reward vesting into the treasury.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Vesting {
    /// The initial amount of ORE to be vested into the treasury.
    pub initial_amount: u64,

    /// The total amount of ORE that has been vested into the treasury.
    pub vested_amount: u64,

    /// The timestamp of the first vesting.
    pub start_time: i64,
}

impl Vesting {
    pub fn vest(&mut self, clock: &Clock, treasury: &mut Treasury) -> u64 {
        let time_elapsed = (clock.unix_timestamp - self.start_time).max(0);
        let vested_amount = self
            .initial_amount
            .min(((self.initial_amount as u128 * time_elapsed as u128) / ONE_HOUR as u128) as u64);
        let amount = vested_amount - self.vested_amount;
        if treasury.total_staked > 0 {
            treasury.rewards_factor += Numeric::from_fraction(amount, treasury.total_staked);
        }
        self.vested_amount = vested_amount;
        amount
    }
}

account!(OreAccount, Vesting);
