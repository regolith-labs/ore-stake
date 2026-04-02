use serde::{Deserialize, Serialize};
use steel::*;

use super::OreAccount;

// TODO Rename. It doesn't hold any tokens so it shouldn't be named Treasury.

/// Treasury is a singleton account which is the mint authority for the ORE token and the authority of
/// the program's global token account.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Treasury {
    /// The cumulative ORE distributed to stakers, divided by the total stake at the time of distribution.
    pub stake_rewards_factor: Numeric,

    /// The current total amount of ORE staking deposits.
    pub total_staked: u64,
}

account!(OreAccount, Treasury);
