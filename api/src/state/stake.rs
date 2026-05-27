use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::{stake_pda, Treasury, Vesting};

use super::OreAccount;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Stake {
    /// The authority of this staker account.
    pub authority: Pubkey,

    /// The balance of this stake account.
    pub balance: u64,

    /// The lamport fee to pay for auto-compounding bots.
    pub compound_fee: u64,

    /// The lamport reserve to pay auto-compounding fees.
    pub compound_fee_reserve: u64,

    /// The timestamp of last claim.
    pub last_claim_at: i64,

    /// The timestamp the last time this staker deposited.
    pub last_deposit_at: i64,

    /// The timestamp the last time this staker withdrew.
    pub last_withdraw_at: i64,

    /// The rewards factor last time rewards were updated on this stake account.
    pub rewards_factor: Numeric,

    /// The amount of ORE this staker can claim.
    pub rewards: u64,

    /// The total amount of ORE this staker has earned over its lifetime.
    pub lifetime_rewards: u64,
}

impl Stake {
    pub fn pda(&self) -> (Pubkey, u8) {
        stake_pda(self.authority)
    }

    pub fn claim(
        &mut self,
        amount: u64,
        clock: &Clock,
        treasury: &mut Treasury,
        vesting: &mut Vesting,
    ) -> u64 {
        self.update_rewards(clock, treasury, vesting);
        let amount = self.rewards.min(amount);
        if amount > 0 {
            self.rewards -= amount;
            self.last_claim_at = clock.unix_timestamp;
        }
        amount
    }

    pub fn deposit(
        &mut self,
        amount: u64,
        clock: &Clock,
        treasury: &mut Treasury,
        sender: &TokenAccount,
        vesting: &mut Vesting,
    ) -> u64 {
        self.update_rewards(clock, treasury, vesting);
        let amount = sender.amount().min(amount);
        if amount > 0 {
            self.balance += amount;
            self.last_deposit_at = clock.unix_timestamp;
            treasury.total_staked += amount;
        }
        amount
    }

    pub fn withdraw(
        &mut self,
        amount: u64,
        clock: &Clock,
        treasury: &mut Treasury,
        vesting: &mut Vesting,
    ) -> u64 {
        self.update_rewards(clock, treasury, vesting);
        let amount = self.balance.min(amount);
        if amount > 0 {
            self.balance -= amount;
            self.last_withdraw_at = clock.unix_timestamp;
            treasury.total_staked -= amount;
        }
        amount
    }

    pub fn update_rewards(
        &mut self,
        clock: &Clock,
        treasury: &mut Treasury,
        vesting: &mut Vesting,
    ) {
        // Vest any unvested ORE into the treasury.
        vesting.vest(&clock, treasury);

        // Accumulate rewards, weighted by stake balance.
        if treasury.rewards_factor > self.rewards_factor {
            let accumulated_rewards = treasury.rewards_factor - self.rewards_factor;
            let personal_rewards = accumulated_rewards * Numeric::from_u64(self.balance);
            self.rewards += personal_rewards.to_u64();
            self.lifetime_rewards += personal_rewards.to_u64();
        }

        // Update this stake account's last seen rewards factor.
        self.rewards_factor = treasury.rewards_factor;
    }
}

account!(OreAccount, Stake);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::ONE_HOUR;

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

    fn vesting_done() -> Vesting {
        Vesting {
            initial_amount: 0,
            vested_amount: 0,
            start_time: 0,
        }
    }

    fn new_stake(balance: u64) -> Stake {
        Stake {
            authority: Pubkey::default(),
            balance,
            compound_fee: 0,
            compound_fee_reserve: 0,
            last_claim_at: 0,
            last_deposit_at: 0,
            last_withdraw_at: 0,
            rewards_factor: Numeric::ZERO,
            rewards: 0,
            lifetime_rewards: 0,
        }
    }

    // -------------------------------------------------------
    // update_rewards
    // -------------------------------------------------------

    #[test]
    fn test_update_rewards_basic_accrual() {
        let mut t = treasury(100);
        let mut v = vesting_done();
        let mut s = new_stake(100);

        // Simulate a distribution: rewards_factor = 1000/100 = 10 per unit
        t.rewards_factor = Numeric::from_fraction(1000, 100);

        s.update_rewards(&clock(100), &mut t, &mut v);

        // 100 balance * 10 per unit = 1000 rewards
        assert_eq!(s.rewards, 1000);
        assert_eq!(s.lifetime_rewards, 1000);
        assert_eq!(s.rewards_factor, t.rewards_factor);
    }

    #[test]
    fn test_update_rewards_zero_balance_earns_nothing() {
        let mut t = treasury(100);
        let mut v = vesting_done();
        let mut s = new_stake(0);

        t.rewards_factor = Numeric::from_fraction(1000, 100);

        s.update_rewards(&clock(100), &mut t, &mut v);

        assert_eq!(s.rewards, 0);
        assert_eq!(s.lifetime_rewards, 0);
    }

    #[test]
    fn test_update_rewards_no_double_counting() {
        let mut t = treasury(100);
        let mut v = vesting_done();
        let mut s = new_stake(100);

        t.rewards_factor = Numeric::from_fraction(1000, 100);
        s.update_rewards(&clock(100), &mut t, &mut v);
        assert_eq!(s.rewards, 1000);

        // Call again with no new distribution
        s.update_rewards(&clock(200), &mut t, &mut v);
        assert_eq!(s.rewards, 1000); // unchanged
        assert_eq!(s.lifetime_rewards, 1000);
    }

    #[test]
    fn test_update_rewards_proportional_to_balance() {
        let mut t = treasury(300); // total staked
        let mut v = vesting_done();

        let mut s1 = new_stake(100); // 1/3 of pool
        let mut s2 = new_stake(200); // 2/3 of pool

        // Distribute 3000 tokens
        t.rewards_factor = Numeric::from_fraction(3000, 300);

        s1.update_rewards(&clock(100), &mut t, &mut v);
        s2.update_rewards(&clock(100), &mut t, &mut v);

        assert_eq!(s1.rewards, 1000);
        assert_eq!(s2.rewards, 2000);
    }

    #[test]
    fn test_update_rewards_accumulates_across_distributions() {
        let mut t = treasury(100);
        let mut v = vesting_done();
        let mut s = new_stake(100);

        // First distribution: 500
        t.rewards_factor = Numeric::from_fraction(500, 100);
        s.update_rewards(&clock(100), &mut t, &mut v);
        assert_eq!(s.rewards, 500);

        // Second distribution: 300 more
        t.rewards_factor += Numeric::from_fraction(300, 100);
        s.update_rewards(&clock(200), &mut t, &mut v);
        assert_eq!(s.rewards, 800);
        assert_eq!(s.lifetime_rewards, 800);
    }

    #[test]
    fn test_update_rewards_with_live_vesting() {
        let mut t = treasury(100);
        let mut v = Vesting {
            initial_amount: 3600,
            vested_amount: 0,
            start_time: 0,
        };
        let mut s = new_stake(100);

        // At half vesting period, 1800 should have vested
        s.update_rewards(&clock(ONE_HOUR / 2), &mut t, &mut v);

        assert_eq!(v.vested_amount, 1800);
        assert_eq!(s.rewards, 1800); // 100% of pool
    }

    // -------------------------------------------------------
    // claim
    // -------------------------------------------------------

    #[test]
    fn test_claim_full_amount() {
        let mut t = treasury(100);
        let mut v = vesting_done();
        let mut s = new_stake(100);
        s.rewards = 500;
        s.rewards_factor = t.rewards_factor;

        let claimed = s.claim(500, &clock(1000), &mut t, &mut v);
        assert_eq!(claimed, 500);
        assert_eq!(s.rewards, 0);
        assert_eq!(s.last_claim_at, 1000);
    }

    #[test]
    fn test_claim_partial_amount() {
        let mut t = treasury(100);
        let mut v = vesting_done();
        let mut s = new_stake(100);
        s.rewards = 500;
        s.rewards_factor = t.rewards_factor;

        let claimed = s.claim(200, &clock(1000), &mut t, &mut v);
        assert_eq!(claimed, 200);
        assert_eq!(s.rewards, 300);
        assert_eq!(s.last_claim_at, 1000);
    }

    #[test]
    fn test_claim_clamped_to_rewards() {
        let mut t = treasury(100);
        let mut v = vesting_done();
        let mut s = new_stake(100);
        s.rewards = 500;
        s.rewards_factor = t.rewards_factor;

        let claimed = s.claim(9999, &clock(1000), &mut t, &mut v);
        assert_eq!(claimed, 500);
        assert_eq!(s.rewards, 0);
    }

    #[test]
    fn test_claim_zero_no_timestamp_update() {
        let mut t = treasury(100);
        let mut v = vesting_done();
        let mut s = new_stake(100);
        s.rewards = 0;
        s.rewards_factor = t.rewards_factor;
        s.last_claim_at = 42;

        let claimed = s.claim(u64::MAX, &clock(1000), &mut t, &mut v);
        assert_eq!(claimed, 0);
        assert_eq!(s.last_claim_at, 42); // unchanged
    }

    // -------------------------------------------------------
    // withdraw
    // -------------------------------------------------------

    #[test]
    fn test_withdraw_full_balance() {
        let mut t = treasury(100);
        let mut v = vesting_done();
        let mut s = new_stake(100);
        s.rewards_factor = t.rewards_factor;

        let withdrawn = s.withdraw(100, &clock(1000), &mut t, &mut v);
        assert_eq!(withdrawn, 100);
        assert_eq!(s.balance, 0);
        assert_eq!(t.total_staked, 0);
        assert_eq!(s.last_withdraw_at, 1000);
    }

    #[test]
    fn test_withdraw_partial() {
        let mut t = treasury(100);
        let mut v = vesting_done();
        let mut s = new_stake(100);
        s.rewards_factor = t.rewards_factor;

        let withdrawn = s.withdraw(40, &clock(1000), &mut t, &mut v);
        assert_eq!(withdrawn, 40);
        assert_eq!(s.balance, 60);
        assert_eq!(t.total_staked, 60);
    }

    #[test]
    fn test_withdraw_clamped_to_balance() {
        let mut t = treasury(100);
        let mut v = vesting_done();
        let mut s = new_stake(100);
        s.rewards_factor = t.rewards_factor;

        let withdrawn = s.withdraw(9999, &clock(1000), &mut t, &mut v);
        assert_eq!(withdrawn, 100);
        assert_eq!(s.balance, 0);
    }

    #[test]
    fn test_withdraw_zero_no_timestamp_update() {
        let mut t = treasury(0);
        let mut v = vesting_done();
        let mut s = new_stake(0);
        s.rewards_factor = t.rewards_factor;
        s.last_withdraw_at = 42;

        let withdrawn = s.withdraw(100, &clock(1000), &mut t, &mut v);
        assert_eq!(withdrawn, 0);
        assert_eq!(s.last_withdraw_at, 42); // unchanged
    }

    #[test]
    fn test_withdraw_updates_rewards_first() {
        let mut t = treasury(100);
        let mut v = Vesting {
            initial_amount: 3600,
            vested_amount: 0,
            start_time: 0,
        };
        let mut s = new_stake(100);

        // Withdraw at half vesting -- should accrue rewards before withdrawing
        let withdrawn = s.withdraw(100, &clock(ONE_HOUR / 2), &mut t, &mut v);
        assert_eq!(withdrawn, 100);
        assert_eq!(s.rewards, 1800); // rewards from vesting were captured
        assert_eq!(s.balance, 0);
    }

    // -------------------------------------------------------
    // claim + withdraw integration
    // -------------------------------------------------------

    #[test]
    fn test_full_lifecycle() {
        let mut t = treasury(0);
        let mut v = vesting_done();
        let mut s = new_stake(0);

        // Simulate deposit: balance=1000, total_staked=1000
        s.balance = 1000;
        t.total_staked = 1000;

        // Distribute 500 (simulate rewards_factor bump)
        t.rewards_factor = Numeric::from_fraction(500, 1000);

        // Claim all
        let claimed = s.claim(u64::MAX, &clock(100), &mut t, &mut v);
        assert_eq!(claimed, 500);
        assert_eq!(s.rewards, 0);

        // Withdraw all
        let withdrawn = s.withdraw(u64::MAX, &clock(200), &mut t, &mut v);
        assert_eq!(withdrawn, 1000);
        assert_eq!(s.balance, 0);
        assert_eq!(t.total_staked, 0);

        // Ready to close
        assert_eq!(s.balance, 0);
        assert_eq!(s.rewards, 0);
    }

    #[test]
    fn test_late_joiner_no_retroactive_rewards() {
        let mut t = treasury(100);
        let mut v = vesting_done();

        // First distribution happened before staker joined
        t.rewards_factor = Numeric::from_fraction(1000, 100);

        // New staker joins with rewards_factor synced to current
        let mut s = new_stake(100);
        s.rewards_factor = t.rewards_factor;
        t.total_staked = 200;

        // Second distribution
        t.rewards_factor += Numeric::from_fraction(1000, 200);

        s.update_rewards(&clock(100), &mut t, &mut v);

        // Should only get share of second distribution: 100/200 * 1000 = 500
        assert_eq!(s.rewards, 500);
    }
}
