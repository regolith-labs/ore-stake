use serde::{Deserialize, Serialize};
use steel::*;

use super::OreAccount;

/// Treasury is a singleton account which tracks top level protocol balances and holds onto staking yield.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Treasury {
    /// The cumulative ORE distributed to stakers, divided by the total stake at the time of distribution.
    pub stake_rewards_factor: Numeric,

    /// The current total amount of ORE staking deposits.
    pub total_staked: u64,
}

account!(OreAccount, Treasury);
