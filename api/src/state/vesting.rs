use serde::{Deserialize, Serialize};
use steel::*;

use super::OreAccount;

/// Vesting account tracks reward vesting into the treasury.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Vesting {
    /// The initial amount of ORE to be vested into the treasury.
    pub initial_amount: u64,

    /// The total amount of ORE that has been vested into the treasury.
    pub total_vested: u64,

    /// The timestamp of the first vesting.
    pub start_time: i64,
}

account!(OreAccount, Vesting);
