use serde::{Deserialize, Serialize};
use steel::*;

pub enum OreStakeEvent {
    Claim = 0,
    Deposit = 1,
    Distribute = 2,
    Withdraw = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct ClaimEvent {
    /// The event discriminator.
    pub disc: u64,

    /// The authority of the staker claiming.
    pub authority: Pubkey,

    /// The amount of ORE claimed.
    pub amount: u64,

    /// The timestamp of the event.
    pub ts: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct DepositEvent {
    /// The event discriminator.
    pub disc: u64,

    /// The authority of the staker depositing.
    pub authority: Pubkey,

    /// The amount of ORE deposited.
    pub amount: u64,

    /// The timestamp of the event.
    pub ts: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct DistributeEvent {
    /// The event discriminator.
    pub disc: u64,

    /// The amount of ORE distributed to stakers.
    pub amount: u64,

    /// The total amount of ORE staked at the time of distribution.
    pub total_staked: u64,

    /// The timestamp of the event.
    pub ts: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct WithdrawEvent {
    /// The event discriminator.
    pub disc: u64,

    /// The authority of the staker withdrawing.
    pub authority: Pubkey,

    /// The amount of ORE withdrawn.
    pub amount: u64,

    /// The timestamp of the event.
    pub ts: i64,
}

event!(ClaimEvent);
event!(DepositEvent);
event!(DistributeEvent);
event!(WithdrawEvent);
