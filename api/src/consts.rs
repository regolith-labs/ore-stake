use solana_program::{pubkey, pubkey::Pubkey};

/// The duration of one minute, in seconds.
pub const ONE_MINUTE: i64 = 60;

/// The duration of one hour, in seconds.
pub const ONE_HOUR: i64 = 60 * ONE_MINUTE;

/// The duration of one day, in seconds.
pub const ONE_DAY: i64 = 24 * ONE_HOUR;

/// The seed of the stake account PDA.
pub const STAKE: &[u8] = b"stake";

/// The seed of the treasury account PDA.
pub const TREASURY: &[u8] = b"treasury";

/// The seed of the vesting account PDA.
pub const VESTING: &[u8] = b"vesting";
