use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::{treasury_pda, Vesting, ONE_HOUR};

use super::OreAccount;

/// Treasury is a singleton account which tracks top level protocol balances and holds onto staking yield.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Treasury {
    /// The cumulative ORE distributed to stakers, divided by the total stake at the time of distribution.
    pub rewards_factor: Numeric,

    /// The current total amount of ORE staking deposits.
    pub total_staked: u64,
}

impl Treasury {
    pub fn pda() -> (Pubkey, u8) {
        treasury_pda()
    }

    pub fn vest(&mut self, clock: &Clock, vesting: &mut Vesting) -> u64 {
        let time_elapsed = clock.unix_timestamp - vesting.start_time;
        let vested_amount = vesting.initial_amount.min(
            ((vesting.initial_amount as u128 * time_elapsed as u128) / ONE_HOUR as u128) as u64,
        );
        let amount = vested_amount - vesting.vested_amount;
        self.rewards_factor += Numeric::from_fraction(amount, self.total_staked);
        vesting.vested_amount = vested_amount;
        amount
    }
}

account!(OreAccount, Treasury);
