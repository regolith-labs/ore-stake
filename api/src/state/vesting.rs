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
        if clock.unix_timestamp < self.start_time {
            return 0;
        }
        let time_elapsed = clock.unix_timestamp - self.start_time;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn clock(ts: i64) -> Clock {
        Clock {
            slot: 0,
            epoch_start_timestamp: 0,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: ts,
        }
    }

    fn treasury(total_staked: u64) -> Treasury {
        Treasury {
            rewards_factor: Numeric::ZERO,
            total_staked,
        }
    }

    fn vesting(initial_amount: u64, start_time: i64) -> Vesting {
        Vesting {
            initial_amount,
            vested_amount: 0,
            start_time,
        }
    }

    #[test]
    fn test_vest_linear_proportional() {
        let mut v = vesting(3600, 0);
        let mut t = treasury(100);

        // 25% elapsed
        let amount = v.vest(&clock(ONE_HOUR / 4), &mut t);
        assert_eq!(amount, 900);
        assert_eq!(v.vested_amount, 900);

        // 50% elapsed (incremental)
        let amount = v.vest(&clock(ONE_HOUR / 2), &mut t);
        assert_eq!(amount, 900);
        assert_eq!(v.vested_amount, 1800);

        // 75% elapsed
        let amount = v.vest(&clock(3 * ONE_HOUR / 4), &mut t);
        assert_eq!(amount, 900);
        assert_eq!(v.vested_amount, 2700);

        // 100% elapsed
        let amount = v.vest(&clock(ONE_HOUR), &mut t);
        assert_eq!(amount, 900);
        assert_eq!(v.vested_amount, 3600);
        assert_eq!(v.vested_amount, v.initial_amount);
    }

    #[test]
    fn test_vest_caps_at_initial_amount() {
        let mut v = vesting(3600, 0);
        let mut t = treasury(100);

        // 2x ONE_HOUR -- should still cap at initial_amount
        let amount = v.vest(&clock(2 * ONE_HOUR), &mut t);
        assert_eq!(amount, 3600);
        assert_eq!(v.vested_amount, 3600);

        // Further calls return 0
        let amount = v.vest(&clock(3 * ONE_HOUR), &mut t);
        assert_eq!(amount, 0);
    }

    #[test]
    fn test_vest_clock_before_start_returns_zero() {
        let mut v = vesting(3600, 1000);
        let mut t = treasury(100);

        let amount = v.vest(&clock(500), &mut t);
        assert_eq!(amount, 0);
        assert_eq!(v.vested_amount, 0);
        assert_eq!(t.rewards_factor, Numeric::ZERO);
    }

    #[test]
    fn test_vest_clock_exactly_at_start() {
        let mut v = vesting(3600, 1000);
        let mut t = treasury(100);

        let amount = v.vest(&clock(1000), &mut t);
        assert_eq!(amount, 0);
        assert_eq!(v.vested_amount, 0);
    }

    #[test]
    fn test_vest_zero_total_staked_orphans_rewards() {
        let mut v = vesting(3600, 0);
        let mut t = treasury(0); // no stakers

        let amount = v.vest(&clock(ONE_HOUR), &mut t);
        assert_eq!(amount, 3600);
        assert_eq!(v.vested_amount, 3600);
        // rewards_factor unchanged -- tokens orphaned by design
        assert_eq!(t.rewards_factor, Numeric::ZERO);
    }

    #[test]
    fn test_vest_updates_rewards_factor() {
        let mut v = vesting(3600, 0);
        let mut t = treasury(100);

        v.vest(&clock(ONE_HOUR), &mut t);

        // rewards_factor should be 3600/100 = 36
        let expected = Numeric::from_fraction(3600, 100);
        assert_eq!(t.rewards_factor, expected);
    }

    #[test]
    fn test_vest_incremental_rewards_factor() {
        let mut v = vesting(3600, 0);
        let mut t = treasury(100);

        v.vest(&clock(ONE_HOUR / 2), &mut t);
        let rf_half = t.rewards_factor;

        v.vest(&clock(ONE_HOUR), &mut t);
        let rf_full = t.rewards_factor;

        // Full vesting rewards_factor should be 2x half
        let expected_half = Numeric::from_fraction(1800, 100);
        let expected_full = Numeric::from_fraction(3600, 100);
        assert_eq!(rf_half, expected_half);
        assert_eq!(rf_full, expected_full);
    }

    #[test]
    fn test_vest_zero_initial_amount() {
        let mut v = vesting(0, 0);
        let mut t = treasury(100);

        let amount = v.vest(&clock(ONE_HOUR), &mut t);
        assert_eq!(amount, 0);
        assert_eq!(v.vested_amount, 0);
    }

    #[test]
    fn test_vest_idempotent_same_timestamp() {
        let mut v = vesting(3600, 0);
        let mut t = treasury(100);

        let a1 = v.vest(&clock(ONE_HOUR / 2), &mut t);
        let rf1 = t.rewards_factor;

        let a2 = v.vest(&clock(ONE_HOUR / 2), &mut t);
        let rf2 = t.rewards_factor;

        assert_eq!(a1, 1800);
        assert_eq!(a2, 0);
        assert_eq!(rf1, rf2);
    }

    #[test]
    fn test_vest_stakers_leave_mid_vest() {
        let mut v = vesting(3600, 0);
        let mut t = treasury(100);

        // First half vests normally
        v.vest(&clock(ONE_HOUR / 2), &mut t);
        let rf_after_first = t.rewards_factor;
        assert_eq!(v.vested_amount, 1800);

        // All stakers leave
        t.total_staked = 0;

        // Second half vests but rewards_factor doesn't change
        v.vest(&clock(ONE_HOUR), &mut t);
        assert_eq!(v.vested_amount, 3600);
        assert_eq!(t.rewards_factor, rf_after_first);
    }
}
