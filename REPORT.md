# ORE Stake - Final Pre-Freeze Audit Report

**Program ID:** `STkEAu2cEyQp5ktgUauRVq8es6mEP2w6ixw4NEd5tDJ`
**Date:** 2026-05-27
**Codebase version:** commit `0bc32c7` + uncommitted changes
**Purpose:** Comprehensive final review before revoking upgrade authority

---

## Executive Summary

The ore-stake program is a well-structured Solana staking contract using the Steel framework and a rewards-factor accumulator pattern. The codebase is clean, concise, and correctly handles the core staking lifecycle. No critical or high-severity vulnerabilities were found. The items below are ordered by importance and represent everything that could be regretted post-freeze.

---

## Findings

### 1. ~~MEDIUM~~ RESOLVED - Zero-amount distribute short-circuited

**Files:** `program/src/claim.rs`, `program/src/withdraw.rs`, `program/src/distribute.rs`

The `amount == 0` early-return checks were removed from `claim`, `withdraw`, and `distribute`. Zero-amount `claim`/`withdraw` calls are harmless (transfer 0, no state corruption, timestamps not updated). For `distribute`, a zero-amount guard was added back as an early `return Ok(())` to prevent touching global vesting state or emitting misleading events.

---

### 2. ~~LOW~~ RESOLVED - Dead code removed from CLI

**File:** `cli/src/main.rs`

The `validate` command, `get_stakes`, `get_program_accounts`, and their associated imports were removed.

---

### 3. ~~LOW~~ RESOLVED - Writability checks added to all writable token accounts

`.is_writable()?` was added to `withdraw:stake_tokens_info`, `distribute:sender_info`, and `distribute:treasury_tokens_info`. All writable token accounts are now consistently validated.

---

### 4. LOW - Error code renumbering is a breaking change

**File:** `api/src/error.rs`

The pending diff removes `AmountZero = 0` and renumbers the remaining errors:
- `NoDeposits`: was `1`, now `0`
- `InsufficientReserves` (renamed from `InsufficientBalance`): was `2`, now `1`

Any off-chain code matching on error codes (e.g., `custom error: 0x1`) will break. If this program is already deployed and clients are parsing errors, the renumbering could cause confusion.

**Recommendation:** If the program is already live, preserve the original discriminants by keeping gaps: `NoDeposits = 1, InsufficientReserves = 2`. If not yet deployed, current approach is fine.

---

### 5. INFO - Vesting rewards permanently lost when `total_staked == 0`

**File:** `api/src/state/vesting.rs:30-32`

When `total_staked` is zero, `vest()` advances `vested_amount` but does not increment `rewards_factor`. Tokens that vest during a zero-staked window are unrecoverable -- they remain in the treasury token account but are never assigned to anyone. This is by design (prevents retroactive reward accrual), but if significant rewards are distributed while no one is staked, those tokens are permanently stranded.

**Implication:** If the program is frozen with tokens stuck in the treasury and no way to recover them, they are lost forever.

---

### 6. INFO - Compound rent exemption check uses pre-transfer lamports

**File:** `program/src/compound.rs:60-67`

The rent check `stake_info.lamports() - stake.compound_fee < minimum_rent` is performed *before* the lamport transfer. This is correct -- it's checking the post-transfer balance. However, note that between the transfer at line 50-57 (treasury tokens to stake tokens) and the lamport send at line 67, the stake account's lamports haven't changed, so the check is accurate.

---

### 7. INFO - Hardcoded event discriminators

**Files:** `program/src/claim.rs:71`, `deposit.rs:105`, `distribute.rs:65`, `withdraw.rs:74`, `compound.rs:72`

Event `disc` values are hardcoded literals (0, 1, 2, 3, 4) rather than derived from `OreStakeEvent` enum variants. Post-freeze this is permanently fixed, but it means the enum in `event.rs` serves only as documentation. If they ever drifted (they can't now), events would be mislabeled.

---

### 8. INFO - No integration or fuzz tests

The codebase has good unit test coverage for `Stake` and `Vesting` logic (~20 tests), but:

- No program-level integration tests (via `solana-program-test` or `bankrun`)
- No fuzz tests for arithmetic edge cases
- No test coverage for instruction handlers (account validation, CPI behavior)
- No test for the compound flow (claim + deposit in sequence)
- No test for the close instruction

This means the account validation logic (PDA checks, signer checks, ATA checks) is only verified by code review, not by automated tests.

---

### 9. INFO - `distribute` vesting merge uses `div_ceil`, may vest 1-2 extra units

**File:** `program/src/distribute.rs:48`

When merging a new distribution into an active vesting schedule, `div_ceil` rounds the elapsed time calculation up. This means the reconstructed `start_time` is pushed slightly further back, causing the next `vest()` call to compute a `vested_amount` that's 1-2 token base-units higher than the actual `vested_amount` at merge time. Over many rapid distributions, this could compound, but it's always bounded by `initial_amount` and favors stakers by at most dust amounts.

---

### 10. INFO - Permissionless `init` allows anyone to pay for account creation

**File:** `program/src/init.rs`

Anyone can call `init` and become the rent payer for the treasury, treasury token account, and vesting accounts. After creation, the program owns these PDAs and the payer cannot reclaim rent. This is by design but means the deployer should call `init` first. If a third party calls it, they're effectively donating ~0.003 SOL in rent.

---

### 11. INFO - CLI hardcodes high compute unit price

**File:** `cli/src/main.rs:157`

`ComputeBudgetInstruction::set_compute_unit_price(1_000_000)` is hardcoded at 1M micro-lamports per CU. This is extremely high for admin operations (init). Not a program issue, but could be costly if used carelessly.

---

### 12. INFO - `compound_fee` can be set without sufficient `compound_fee_deposit`

**File:** `program/src/deposit.rs:91-93`

A user can set `compound_fee = 1_000_000` with `compound_fee_deposit = 0`. The compound instruction checks `compound_fee_reserve >= compound_fee` (compound.rs:20), so auto-compounding would simply never execute. Not harmful, but could confuse users who set a fee but forget to fund the reserve.

---

### 13. INFO - Overflow protection correctly enabled

**File:** `Cargo.toml:40,50`

Both `[profile.release]` and `[profile.dev]` have `overflow-checks = true`. All arithmetic operations will panic (reverting the transaction) on overflow rather than silently wrapping. This is essential for a financial program. The `distribute.rs` merge math uses `u128` intermediates for extra safety.

---

### 14. INFO - `vesting.rs` cast chain from `i64` to `u128`

**File:** `api/src/state/vesting.rs:27-28`

```rust
let time_elapsed = clock.unix_timestamp - self.start_time;
let vested_amount = self.initial_amount
    .min(((self.initial_amount as u128 * time_elapsed as u128) / ONE_HOUR as u128) as u64);
```

`time_elapsed` is `i64`. The guard at line 22 ensures it's non-negative before the cast to `u128`. The `i64 as u128` cast for non-negative values is well-defined in Rust. The multiplication `u64::MAX * i64::MAX` fits comfortably in `u128`. The final `as u64` truncation is safe because `.min(self.initial_amount)` caps the value at `u64` range.

---

## Architecture Review

### What works well

- **Rewards-factor accumulator pattern** is correctly implemented. Each staker's rewards are `(treasury.rewards_factor - stake.rewards_factor) * stake.balance`, updated lazily on every interaction.

- **Vesting** provides MEV protection. The 1-hour linear vesting prevents sandwich attacks on `distribute` -- an attacker cannot deposit, wait for distribution, and immediately withdraw.

- **Safety checks** post-deposit and post-withdraw verify `stake_tokens.amount() >= stake.balance`, catching any accounting drift.

- **Rounding always favors the protocol.** The Numeric type truncates toward zero on division, multiplication, and `to_u64()`. Treasury can never become insolvent from rounding.

- **PDA validation** is thorough. Treasury and vesting are validated by address. Stake accounts are validated by program ownership + authority field. All token accounts are validated as proper ATAs.

### Potential regrets post-freeze

1. **No governance or parameter updates.** `ONE_HOUR` (vesting duration) and `ONE_DAY` (compound cooldown) are hardcoded constants. Post-freeze, these can never be tuned.

2. **No emergency pause.** There is no mechanism to pause the program in case of a discovered vulnerability. Once frozen, the only mitigation is to tell users to stop interacting.

3. **No way to recover orphaned tokens.** If tokens vest while `total_staked == 0`, they're permanently locked in the treasury token account with no mechanism to reclaim them.

4. **Single vesting schedule.** There is only one global `Vesting` account. Multiple rapid `distribute` calls merge into a single schedule, which compresses the vesting window for newly added tokens. This is a design choice, not a bug, but means rapid distributions effectively give later tokens a shorter vesting period than 1 hour.

5. **Compound is all-or-nothing.** `stake.claim(u64::MAX, ...)` always claims all rewards, then deposits all of them. There's no partial compound.

---

## Uncommitted Changes Assessment

The pending diff makes four changes:

| Change | Risk |
|---|---|
| Delete `DEPLOY.md` | None - operational runbook, not program code |
| Remove `AmountZero` checks from claim/withdraw/distribute | Resolved. Zero guard kept on distribute via early `return Ok(())` |
| Rename `InsufficientBalance` to `InsufficientReserves` | See Finding #4. Breaking if clients parse error codes |
| Remove `validate` CLI command | Resolved. Dead code cleaned up |

---

## Files Reviewed

**Program (on-chain):**
- `program/src/lib.rs` - Entrypoint, dispatch, security_txt
- `program/src/init.rs` - Initialize singleton accounts
- `program/src/deposit.rs` - Deposit ORE into staking
- `program/src/withdraw.rs` - Withdraw ORE from staking
- `program/src/claim.rs` - Claim accrued rewards
- `program/src/compound.rs` - Auto-compound rewards
- `program/src/distribute.rs` - Distribute rewards to stakers
- `program/src/close.rs` - Close empty stake accounts
- `program/src/log.rs` - Internal CPI logging

**API (state, types, SDK):**
- `api/src/lib.rs` - Program ID
- `api/src/consts.rs` - Constants
- `api/src/error.rs` - Error types
- `api/src/event.rs` - Event structs
- `api/src/instruction.rs` - Instruction data types
- `api/src/sdk.rs` - Client-side instruction builders
- `api/src/state/mod.rs` - Account types, PDA derivation
- `api/src/state/stake.rs` - Stake logic + tests
- `api/src/state/treasury.rs` - Treasury state
- `api/src/state/vesting.rs` - Vesting logic + tests

**CLI (off-chain):**
- `cli/src/main.rs` - Admin CLI

**Configuration:**
- `Cargo.toml` - Workspace config
- `program/Cargo.toml` - Program deps
- `api/Cargo.toml` - API deps
- `cli/Cargo.toml` - CLI deps

---

## Checklist

- [x] All PDA accounts validated by address or ownership
- [x] All signers validated
- [x] All token accounts validated as proper ATAs
- [x] Mint validated against hardcoded address
- [x] Overflow checks enabled in release and dev profiles
- [x] No reentrancy risk (CPIs only to system programs)
- [x] Rounding favors protocol (treasury always solvent)
- [x] Sandwich attacks mitigated by vesting
- [x] Close requires zero balance and zero rewards
- [x] Compound has opt-in, cooldown, and rent-exemption guards
- [x] security_txt configured
- [ ] Integration tests for instruction handlers
- [ ] Fuzz tests for arithmetic edge cases
- [x] `.is_writable()` on all writable token accounts
- [x] Dead code removed from CLI

---

*This report covers static analysis only. No dynamic testing, fuzzing, or formal verification was performed. It should supplement -- not replace -- human expert review.*
