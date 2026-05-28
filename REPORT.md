# Pre-Freeze Audit Report

**Program:** ore-stake v0.2.5
**Date:** 2026-05-28
**Scope:** Full source review of `api/` and `program/` crates. CLI reviewed for completeness.

---

## Critical / High Severity

### 1. Orphaned rewards when `total_staked == 0` during vesting

**File:** `api/src/state/vesting.rs:30-33`

When tokens vest while `total_staked == 0`, `vested_amount` advances but `rewards_factor` is not updated. The tokens remain in the treasury token account permanently unclaimable — they are not attributed to any staker and there is no recovery mechanism.

This is tested and marked "orphaned by design," but after a freeze there is no way to recover these funds. Any distribute that completes vesting while there are zero stakers will permanently lock those tokens.

**Impact:** Permanent loss of distributed tokens under a plausible scenario (all stakers withdraw between distribute calls).

### 2. `init` is permissionless — no access control

**File:** `program/src/init.rs:13`

`process_init` checks that `signer_info` is a signer but does not verify the signer is an expected authority. Anyone can call init. The function is idempotent (checks `data_is_empty`), so after first initialization this is harmless. But there is no way to prevent a race to initialize, and no admin authority is recorded anywhere.

**Impact:** Low in practice (idempotent), but there is zero admin control for any future operational needs post-freeze.

---

## Medium Severity

### 3. `compound_fee` silently overwritten on every deposit

**File:** `program/src/deposit.rs:92`

```rust
stake.compound_fee = compound_fee;
```

Every deposit unconditionally overwrites the compound fee. A user who deposits additional ORE and passes `compound_fee: 0` will silently disable auto-compounding. The compound instruction requires `compound_fee > 0` (`compound.rs:19`), so their stake can no longer be compounded until another deposit sets a nonzero fee.

**Impact:** User-facing footgun. Auto-compounding can be accidentally disabled with no warning.

### 4. Deposit silently caps amount to sender balance

**File:** `api/src/state/stake.rs:72`

```rust
let amount = sender.amount().min(amount);
```

If a user requests a deposit of 100 ORE but only has 50, the instruction silently deposits 50 instead of failing. The event emitted reflects the reduced amount, but clients expecting an exact deposit will be silently shorted.

**Impact:** Unexpected behavior for integrators. A deposit request of X may deposit less than X without error.

### 5. Single global vesting schedule — distributor interference

**File:** `program/src/distribute.rs:44-57`

There is one global `Vesting` account. When a second distribute arrives before the first finishes vesting, the schedule is adjusted via a `start_time` recalculation. Rapid successive distributes will continuously extend the effective vesting window, diluting the vesting rate for all stakers. This is by design but has surprising emergent behavior under high distribute frequency.

**Impact:** Under rapid distribute cadence, rewards reach stakers more slowly than the 1-hour vesting period suggests.

---

## Low Severity / Code Quality

### 8. Dead code: `OreStakeEvent` enum

**File:** `api/src/event.rs:4-10`

The `OreStakeEvent` enum is defined but never used anywhere. Event discriminators are hardcoded as magic numbers in the program handlers:

```rust
// deposit.rs:108
disc: 1,
// claim.rs:70
disc: 0,
// withdraw.rs:78
disc: 3,
```

The enum values (Claim=0, Deposit=1, Distribute=2, Withdraw=3, Compound=4) match the hardcoded values, but the enum itself is dead code.

### 9. Dead code: `Stake::pda()` and `Treasury::pda()` methods

**Files:** `api/src/state/stake.rs:43-45`, `api/src/state/treasury.rs:20-22`

Both `pda()` methods are defined but never called anywhere in the codebase. The free functions `stake_pda()` and `treasury_pda()` in `state/mod.rs` are used instead.

### 10. Dead code: `ONE_MINUTE` constant

**File:** `api/src/consts.rs:2`

`ONE_MINUTE` is only used to define `ONE_HOUR`. It is never referenced directly elsewhere.

### 11. Missing `#[allow(dead_code)]` or removal of unused items

Items 8-10 above would generate compiler warnings without suppression. These should either be removed or explicitly marked as public API.

### 12. Event discriminators should use the enum

The `OreStakeEvent` enum exists for exactly this purpose. Instead of `disc: 1`, the code should use `disc: OreStakeEvent::Deposit as u64`. This prevents accidental mismatch if event ordering changes.

### 13. `close` does not validate `recipient_info` when token balance is zero

**File:** `program/src/close.rs:28-53`

If `stake_tokens.amount() == 0`, the `recipient_info` account is never validated — it skips the ATA check entirely. No transfer occurs, so this is harmless, but it means an arbitrary writable account can be passed as `recipient_info` without error.

### 14. Inconsistent `data` parameter handling

**File:** `program/src/lib.rs:31-38`

- `process_close(accounts)` — no data parameter
- `process_compound(accounts, data)` — takes data but ignores it (`_data`)
- `process_init(accounts, data)` — takes data but ignores it (`_data`)
- `process_log(accounts, data)` — takes data but ignores it (`_data`)

Three handlers accept data they never use. Minor inconsistency.

### 15. No integration tests

All tests are unit tests in `api/src/state/`. There are no program-level integration tests that exercise the instruction handlers against a simulated runtime. The state logic is well-tested; the account validation, CPI, and token transfer paths are untested.

---

## Design Notes (Not Bugs)

### 16. Permissionless compound

Anyone can call `compound` on any stake account. The caller receives `compound_fee` SOL as payment. This is the intended design for a public auto-compound bot network, but it means bots will race to compound, and MEV extractors could front-run legitimate compounders.

### 17. No admin / pause / emergency mechanism

There is no admin key, no pause functionality, and no emergency withdrawal mechanism. After freeze, any issue discovered in the program is permanent. This is a deliberate design choice for trustlessness, but the trade-off is zero recoverability.

### 18. Vesting precision loss in `distribute` schedule adjustment

**File:** `program/src/distribute.rs:52-55`

The `new_elapsed` recalculation uses integer arithmetic with `div_ceil`, which introduces up to 1 second of rounding error per adjustment. Under repeated rapid distributes, these errors accumulate. The practical impact is negligible (a few seconds of vesting drift at most).

### 19. `close` returns `compound_fee_reserve` SOL implicitly

**File:** `program/src/close.rs:65`

When a stake account is closed, `stake_info.close(&signer_info)` returns all lamports (including `compound_fee_reserve`) to the signer. The close check only verifies `balance == 0 && rewards == 0` — it does not check or report on `compound_fee_reserve`. This is correct but implicit.

### 20. CLI hardcoded compute budget

**File:** `cli/src/main.rs:142-143`

```rust
ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
ComputeBudgetInstruction::set_compute_unit_price(1_000_000),
```

The compute unit limit (1.4M) and price (1M micro-lamports) are hardcoded. The price is very high for a CLI tool and could waste SOL. This is a CLI-only concern and does not affect the on-chain program.

---

## Summary

| Severity | Count | Key Items |
|----------|-------|-----------|
| Critical/High | 2 | Orphaned rewards, permissionless init |
| Medium | 3 | Fee overwrite, silent deposit cap, vesting interference |
| Low / Quality | 8 | Dead code, missing tests, inconsistencies |
| Design Notes | 5 | Permissionless compound, no admin, precision loss |

The core staking math (rewards factor accumulation, vesting) is sound and well-tested at the unit level. The primary concern for a freeze is **item #1** — tokens permanently locked when vesting completes with zero stakers. If this scenario is unlikely in practice, the program is in reasonable shape. The dead code (items 8-10) and magic number discriminators (item 12) are the most actionable cleanups before freeze.
