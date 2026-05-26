# Code Review: ore-stake changes since 863ace0

**Commits reviewed** (oldest to newest):
1. `000c05b` — remove cooldown period
2. `c5b9465` — vesting
3. `12100da` — cleanup
4. `ccd2881` — update version
5. `0cd933b` — fix griefing issue
6. `2210ff8` — refactor vest() from Treasury to Vesting, fix init idempotency, cleanup

**Version**: 0.1.8 -> 0.2.1

---

## Summary of Changes

### 1. New Vesting System (major feature)

Previously, `distribute()` credited rewards to stakers **instantly** by bumping `treasury.rewards_factor` in a single step. Now, rewards vest **linearly over 1 hour** via a new singleton `Vesting` PDA account.

**New account -- `Vesting`** (`api/src/state/vesting.rs`):
- `initial_amount: u64` -- total ORE to vest
- `vested_amount: u64` -- ORE vested so far
- `start_time: i64` -- timestamp when vesting began

**Core vesting logic -- `Vesting::vest()`** (`api/src/state/vesting.rs:21-32`):
```rust
let time_elapsed = clock.unix_timestamp - self.start_time;
let vested_amount = self.initial_amount.min(
    ((self.initial_amount as u128 * time_elapsed as u128) / ONE_HOUR as u128) as u64,
);
let amount = vested_amount - self.vested_amount;
if treasury.total_staked > 0 {
    treasury.rewards_factor += Numeric::from_fraction(amount, treasury.total_staked);
}
self.vested_amount = vested_amount;
```

Every user-facing operation (deposit, withdraw, claim, compound) now calls `update_rewards()` which calls `vesting.vest()` first, ensuring the rewards_factor is up-to-date before computing personal rewards.

**Distribute mid-vesting merging** (`program/src/distribute.rs:37-50`):
When a new distribution arrives before the prior one finishes vesting, the code merges them:
- Flushes any pending vesting via `vesting.vest()`
- Adds new amount to `initial_amount`
- Recalculates `start_time` so that `vested_amount` remains consistent under the new total

### 2. Cooldown Period Removal

- **Claim**: Removed `last_deposit_at + ONE_DAY < clock.unix_timestamp` assertion
- **Withdraw**: Removed same assertion

Users can now claim and withdraw immediately after depositing.

### 3. Idempotent Init

`process_init()` wraps each account creation in `if data_is_empty()` checks, with else branches that validate existing accounts via `as_account_mut()` (owner + discriminator check). This allows re-running init to create the new Vesting account on an already-initialized program.

### 4. Refactor: vest() moved to Vesting

`vest()` was moved from `Treasury` to `Vesting`, where it naturally belongs. The method primarily operates on Vesting state and only touches Treasury to bump `rewards_factor`.

### 5. Plumbing Changes

- Every instruction now accepts and loads the `vesting` account
- SDK functions updated to include `vesting_pda()` in account lists
- IDL updated with Vesting account type and CompoundEvent
- CLI updated to fetch clock and vesting for accurate reward calculations

---

## Security Audit

### Account Validation -- PASS

All accounts are properly validated across all instructions:

- **Owner checks**: All custom accounts validated via `.as_account_mut::<Type>(&ore_stake_api::ID)?` which checks both program ownership and discriminator
- **Signer checks**: `signer_info.is_signer()?` on all privileged operations; authority verified via `.assert_mut(|s| s.authority == *signer_info.key)?`
- **Token accounts**: Associated token accounts validated with `.as_associated_token_account(owner, mint)?`; mint address checked with `.has_address(&MINT_ADDRESS)?`
- **Program IDs**: System program, token program, and associated token program all verified

### Access Control -- PASS

- **Deposit**: Only signer can deposit to their own stake account
- **Withdraw**: Only stake authority can withdraw
- **Claim**: Only stake authority can claim rewards
- **Compound**: Rate-limited to once per day (`last_claim_at + ONE_DAY`); requires `compound_fee > 0` opt-in
- **Close**: Only stake authority can close; requires zero balance
- **Distribute**: Permissionless but signer must provide the tokens from their own ATA
- **Init**: Permissionless (correct for singleton PDA initialization; signer pays rent)

### Token Safety -- PASS

- All transfers use `transfer()` or `transfer_signed()` with correct PDA seeds
- Zero-amount checks on all user-provided amounts
- Safety checks in deposit and withdraw ensure `stake_tokens.amount() >= stake.balance`
- Compound checks rent exemption after fee deduction

### Arithmetic Safety -- PASS

- u128 intermediates prevent overflow in vesting math
- `saturating_mul()` used in distribute mid-vesting merge
- `.min()` used to cap amounts (vested to initial, withdraw to balance, claim to rewards, deposit to sender balance)
- Division only by constants (`ONE_HOUR = 3600`) or guarded values (`total_staked > 0`)
- `Numeric` fixed-point type handles rewards_factor precision

### Reentrancy / CPI -- PASS

- No CPI back to self or untrusted programs
- Only external calls are to SPL Token and Associated Token programs
- `program_log()` uses `invoke_signed()` but only for event logging via treasury PDA

### Compound Fee Logic -- PASS (No Griefing Vectors)

- Requires explicit opt-in (`compound_fee > 0`)
- Reserve must cover fee (`compound_fee_reserve >= compound_fee`)
- Rate-limited to once per day
- Rent exemption verified after fee deduction
- Bot is paid from user's pre-funded reserve, not from rewards

---

## Remaining Issues

### ISSUE 1 (Medium): Cooldown removal enables just-in-time staking

Removing the ONE_DAY cooldown means a user can deposit, trigger a vest, claim rewards, and withdraw in rapid succession.

The 1-hour vesting mitigates the **instant** form of this (you can't capture a full distribution instantly), but a sophisticated attacker could:
1. Watch for `distribute` transactions in the mempool
2. Deposit a large amount right before or in the same block
3. Wait ~1 hour for full vesting
4. Claim and withdraw

Since vesting is linear, the attacker captures rewards proportional to their `(deposit / total_staked)` for the time they're staked. If `total_staked` is small and the attacker deposits a large amount, they capture most of the distribution.

**Impact**: Moderate. The 1-hour vest period makes this less profitable but doesn't eliminate it.

**Recommendation**: Consider a minimum staking duration (even a short one like 1 hour matching the vest period), or a "warm-up" period where newly deposited stake doesn't earn rewards for some window.

### ISSUE 2 (Low): No explicit PDA address validation on Vesting account

All instructions validate the Vesting account via:
```rust
vesting_info.as_account_mut::<Vesting>(&ore_stake_api::ID)?
```

This checks owner and discriminator but **not** that the address matches `vesting_pda().0`. Since the program only ever creates one Vesting account (PDA with seeds `[b"vesting"]`), and only program-owned accounts pass the owner check, this is safe in practice. But explicit PDA verification would be defense-in-depth.

### ISSUE 3 (Low): Negative time_elapsed edge case in vest()

In `Vesting::vest()`:
```rust
let time_elapsed = clock.unix_timestamp - self.start_time;
```

If `time_elapsed` were negative, the `i64` cast to `u128` wraps to a very large number, and `vested_amount` gets capped to `initial_amount` by `.min()`. This would instantly vest everything.

In current code, `start_time` is always set to `clock.unix_timestamp` or `clock.unix_timestamp - new_elapsed` (where `new_elapsed >= 0`), and Solana clocks only move forward, so `time_elapsed >= 0` always holds. But a `max(0, time_elapsed)` guard would make the invariant explicit and protect against any future code paths that might violate it.

---

## Math Verification

### Linear vesting formula

```
vested = min(initial, initial * elapsed / ONE_HOUR)
```

- At t=0: `vested = min(initial, 0) = 0` -- correct
- At t=ONE_HOUR: `vested = min(initial, initial) = initial` -- correct (fully vested)
- At t=ONE_HOUR/2: `vested = min(initial, initial/2) = initial/2` -- correct (50% vested)
- Uses u128 intermediate to avoid overflow -- correct for u64 amounts * i64 timestamps

### Mid-vesting merge formula

When adding `amount` to an in-progress vesting:
```rust
new_initial = initial + amount
new_elapsed = ceil(vested_amount * ONE_HOUR / new_initial)
start_time = now - new_elapsed
```

This solves for: "what start_time would produce the current vested_amount under the new initial_amount?" Using the vesting formula:
```
vested_amount = new_initial * new_elapsed / ONE_HOUR
=> new_elapsed = vested_amount * ONE_HOUR / new_initial
```

`div_ceil` rounds up `new_elapsed`, pushing `start_time` slightly further into the past. On the next vest call, the linear formula may produce a `vested_amount` slightly higher than stored, resulting in a tiny extra vest. The error is bounded by 1 unit of the smallest denomination and is negligible.

**The math is correct.** The vesting approach is sound.

### Rewards factor accumulation

The standard "rewards per share" pattern is preserved:
```
rewards_factor += amount / total_staked   (per vest increment)
personal_rewards = (current_factor - last_seen_factor) * balance
```

This is the canonical approach (used by Synthetix, Sushiswap MasterChef, etc.) and works correctly with the vesting additions.

### Overflow analysis

- `initial_amount as u128 * time_elapsed as u128`: max ~1.8e19 * 1.7e18 = 3e37, fits in u128
- `vested_amount as u128 * ONE_HOUR as u128`: max ~1.8e19 * 3600 = 6.5e22, fits in u128
- `new_initial = vesting.initial_amount + amount`: u64, could overflow with extreme values but bounded by ORE total supply
- `Numeric::from_fraction(amount, total_staked)`: guarded by `total_staked > 0`

No overflow risk under realistic token supply constraints.

---

## Summary Table

| # | Severity | Issue | Status |
|---|----------|-------|--------|
| 1 | Medium | No cooldown enables just-in-time staking | Design tradeoff -- consider mitigation |
| 2 | Low | No explicit PDA check on Vesting account | Defense-in-depth suggestion |
| 3 | Low | Negative time_elapsed edge case | Defensive guard suggestion |

### Previously identified issues now resolved:
- ~~Init idempotency broken~~ -- Fixed in `2210ff8` (removed `is_empty()` guards)
- ~~Commented-out code in withdraw.rs~~ -- Cleaned up in `2210ff8`

**Overall assessment**: The program is well-structured and secure. Account validation, access control, token safety, and arithmetic are all sound. The vesting math is correct. The one meaningful concern is the lack of a cooldown or warm-up period, which combined with the 1-hour vesting window, leaves a narrow but real opportunity for just-in-time staking arbitrage.
