# ORE Stake - Final Pre-Freeze Audit Report

**Program ID:** `STkEAu2cEyQp5ktgUauRVq8es6mEP2w6ixw4NEd5tDJ`
**Date:** 2026-05-28
**Codebase version:** commit `fd1de8d` (master, clean)
**Purpose:** Comprehensive final review before code freeze

---

## Executive Summary

The ore-stake program is a well-structured Solana staking contract using the Steel framework and a rewards-factor accumulator pattern. The codebase is clean, concise, and correctly handles the core staking lifecycle (deposit, withdraw, claim, compound, distribute, close). No critical or high-severity vulnerabilities were found. All items from the previous RECOMMENDATIONS.md have been addressed (compound SDK fixed, close SDK added). The remaining findings are low/informational and represent everything that could be regretted post-freeze.

---

## Critical / High Findings

**None.**

---

## Medium Findings

**None.**

---

## Low Findings

### L-1. Error code renumbering is a breaking change if program is deployed

**File:** `api/src/error.rs`

```rust
pub enum OreStakeError {
    NoDeposits = 0,
    InsufficientReserves = 1,
}
```

If the program was previously deployed with `NoDeposits = 1` and `InsufficientBalance = 2` (old names/codes), any off-chain code matching on `custom error: 0x1` will misinterpret errors. If the program is not yet deployed on mainnet, this is a non-issue.

**Recommendation:** If deployed, preserve original discriminants: `NoDeposits = 1, InsufficientReserves = 2`.

---

### L-2. IDL version (`0.2.2`) is stale vs crate version (`0.2.4`)

**File:** `api/idl.json:2`

The IDL declares `"version": "0.2.2"` but `Cargo.toml` workspace version is `0.2.4`. Clients consuming the IDL may see a version mismatch.

**Recommendation:** Bump IDL version to `0.2.4` or match whatever the frozen version will be.

---

### L-3. Unused dependencies

| Crate | Location | Used? |
|---|---|---|
| `base64` | `api/Cargo.toml` | Not imported anywhere in `api/src/` |
| `solana-account-decoder` | `cli/Cargo.toml` | Not imported in `cli/src/main.rs` |
| `spl-associated-token-account` | `cli/Cargo.toml` | Not imported in `cli/src/main.rs` |
| `rand` (dev-dep) | `program/Cargo.toml` | Not used in any test file |

These inflate compile times and lockfile size. Not a correctness issue.

**Recommendation:** Remove unused dependencies before freeze.

---

## Informational Findings

### I-1. Vesting rewards permanently lost when `total_staked == 0`

**File:** `api/src/state/vesting.rs:30-32`

When `total_staked` is zero, `vest()` advances `vested_amount` but does not increment `rewards_factor`. Tokens that vest during a zero-staked window are permanently unrecoverable -- they remain in the treasury token account but are never assigned to anyone. This is by design (prevents retroactive reward accrual), but if significant rewards are distributed while no one is staked, those tokens are permanently stranded.

**Post-freeze implication:** No mechanism exists to sweep orphaned tokens from the treasury.

---

### I-2. No emergency pause mechanism

There is no mechanism to pause the program in case of a discovered vulnerability. Once frozen, the only mitigation is social -- telling users to stop interacting with the program.

---

### I-3. Hardcoded constants cannot be tuned post-freeze

**File:** `api/src/consts.rs`

- `ONE_HOUR` (vesting duration) = 3600 seconds
- `ONE_DAY` (compound cooldown) = 86400 seconds

These are baked into the program and can never be adjusted.

---

### I-4. Hardcoded event discriminators

**Files:** `program/src/claim.rs:71`, `deposit.rs:105`, `distribute.rs:69`, `withdraw.rs:77`, `compound.rs:74`

Event `disc` values are hardcoded literals (0, 1, 2, 3, 4) rather than derived from the `OreStakeEvent` enum. The enum in `event.rs` serves only as documentation. Post-freeze this is permanently locked, but if they had ever drifted, events would be mislabeled. Verified: they match.

---

### I-5. No integration or fuzz tests

The codebase has good unit test coverage for `Stake` and `Vesting` logic (~20 tests), but:

- No program-level integration tests (via `solana-program-test` or `bankrun`)
- No fuzz tests for arithmetic edge cases
- No test coverage for instruction handlers (account validation, CPI behavior)
- No test for the compound flow end-to-end
- No test for the close instruction

Account validation logic (PDA checks, signer checks, ATA checks) is verified only by code review.

---

### I-6. `distribute` vesting merge rounding with `div_ceil`

**File:** `program/src/distribute.rs:52-55`

```rust
let new_elapsed = (vesting.vested_amount as u128)
    .saturating_mul(ONE_HOUR as u128)
    .div_ceil(new_initial as u128) as i64;
vesting.start_time = clock.unix_timestamp - new_elapsed;
```

`div_ceil` rounds the elapsed-time calculation up, pushing `start_time` slightly further back. The next `vest()` call will compute a `vested_amount` that's 1-2 token base-units higher than the actual amount at merge time. Over many rapid distributions this could compound, but it's always bounded by `initial_amount` and favors stakers by at most dust amounts.

---

### I-7. Permissionless `init`

**File:** `program/src/init.rs`

Anyone can call `init` and become the rent payer for the treasury, treasury token account, and vesting accounts. After creation, the program owns these PDAs and the payer cannot reclaim rent (~0.003 SOL). The instruction is idempotent (checks `data_is_empty()`), so a second call is a no-op. Not exploitable, but the deployer should call `init` first.

---

### I-8. Compound fee can be set without funding the reserve

**File:** `program/src/deposit.rs:91-93`

A user can set `compound_fee = 1_000_000` with `compound_fee_deposit = 0`. The compound instruction checks `compound_fee_reserve >= compound_fee` (compound.rs:20), so auto-compounding would simply never trigger. Not harmful, but could confuse users.

---

### I-9. `compound_fee` is overwritten on every deposit

**File:** `program/src/deposit.rs:91`

```rust
stake.compound_fee = compound_fee;
```

Each deposit overwrites the compound fee setting. A user depositing additional ORE must re-specify their desired compound fee, or it resets. If they pass `compound_fee = 0` on a second deposit, auto-compounding is silently disabled.

---

### I-10. CLI hardcodes extreme compute unit price

**File:** `cli/src/main.rs:144`

```rust
ComputeBudgetInstruction::set_compute_unit_price(1_000_000)
```

1M micro-lamports/CU is extremely high for admin operations. Not a program issue, but could be costly if used carelessly. The CLI is `publish = false` so this only affects internal use.

---

### I-11. Single global vesting schedule compresses windows

**File:** `api/src/state/vesting.rs`, `program/src/distribute.rs:44-57`

There is only one global `Vesting` account. Multiple rapid `distribute` calls merge into a single schedule, which compresses the vesting window for newly added tokens. This is a design choice -- it means rapid distributions effectively give later tokens a shorter vesting period than 1 hour, reducing the MEV protection for those specific tokens.

---

### I-12. Variable naming inconsistency in SDK

**File:** `api/src/sdk.rs`

Several variables named `vesting_info`, `treasury_tokens_info`, `ore_mint_info` hold `Pubkey` addresses, not `AccountInfo`. This is a readability issue only (e.g., `vesting_info` at line 39, 50, etc. should be `vesting_address`).

---

## Previously Identified Items - Status

| Item | Source | Status |
|---|---|---|
| `sdk::compound()` conflated bot signer with authority | RECOMMENDATIONS.md #1 | **RESOLVED** - Now takes `(signer, authority)` |
| Missing `close` SDK builder | RECOMMENDATIONS.md #2 | **RESOLVED** - Added at `sdk.rs:148-167` |
| Dead CLI code (`validate`, `get_stakes`, etc.) | Previous audit | **RESOLVED** - Removed |
| Missing `.is_writable()` on writable token accounts | Previous audit | **RESOLVED** - All writable accounts checked |
| Zero-amount `distribute` touching vesting state | Previous audit | **RESOLVED** - Early `return Ok(())` added |

---

## Security Checklist

- [x] All PDA accounts validated by address or ownership
- [x] All signers validated with `.is_signer()?`
- [x] All token accounts validated as proper ATAs
- [x] ORE mint validated against hardcoded `MINT_ADDRESS`
- [x] Overflow checks enabled in both release and dev profiles
- [x] No reentrancy risk (CPIs only to system/token programs and self-log)
- [x] Rounding favors protocol via Numeric truncation (treasury stays solvent)
- [x] MEV/sandwich attacks mitigated by 1-hour vesting window
- [x] `close` requires zero balance and zero rewards
- [x] Compound has opt-in fee, cooldown (`ONE_DAY`), and rent-exemption guards
- [x] `security_txt` configured with contact info and policy link
- [x] `distribute` short-circuits on `amount == 0`
- [x] Stake authority checked on deposit, withdraw, claim, and close
- [x] Compound is permissionless by design (anyone can trigger for any stake)
- [x] Post-transfer safety checks: `stake_tokens.amount() >= stake.balance` in deposit and withdraw
- [x] `u128` intermediates used for large multiplications in vesting and distribute
- [x] `i64` time values guarded for non-negativity before cast to `u128`
- [ ] Integration tests for instruction handlers
- [ ] Fuzz tests for arithmetic edge cases

---

## Architecture Notes

### What works well

- **Rewards-factor accumulator pattern** is correctly implemented. Each staker's rewards are `(treasury.rewards_factor - stake.rewards_factor) * stake.balance`, updated lazily on every interaction. Late joiners cannot retroactively earn.

- **Vesting** provides MEV protection. The 1-hour linear vesting prevents sandwich attacks on `distribute` -- an attacker cannot deposit, wait for distribution, and immediately withdraw to capture rewards.

- **Safety invariant checks** post-deposit and post-withdraw verify `stake_tokens.amount() >= stake.balance`, catching any accounting drift before the transaction commits.

- **Rounding always favors the protocol.** The `Numeric` type truncates toward zero. Treasury can never become insolvent from rounding errors.

- **PDA validation** is thorough. Treasury and vesting are validated by exact address. Stake accounts are validated by program ownership + authority field match. All token accounts are validated as proper ATAs for the correct owner and mint.

- **Clean separation** between API (types, SDK, state logic) and program (instruction handlers). State mutation logic lives in `Stake` and `Vesting` impl blocks with unit tests.

### Permanent design tradeoffs (post-freeze)

1. **No governance or parameter updates.** Vesting duration and compound cooldown are hardcoded forever.
2. **No emergency pause.** No way to halt the program if a vulnerability is discovered.
3. **No orphan token recovery.** Tokens vested during `total_staked == 0` are permanently locked.
4. **Single vesting schedule.** Rapid distributions compress vesting for later tokens.
5. **Compound is all-or-nothing.** `stake.claim(u64::MAX)` always claims all rewards then deposits all. No partial compound.

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
- `api/src/lib.rs` - Program ID declaration
- `api/src/consts.rs` - Time and seed constants
- `api/src/error.rs` - Custom error types
- `api/src/event.rs` - Event structs with discriminators
- `api/src/instruction.rs` - Instruction data types and enum
- `api/src/sdk.rs` - Client-side instruction builders
- `api/src/state/mod.rs` - Account types, PDA derivation helpers
- `api/src/state/stake.rs` - Stake state + logic + 17 unit tests
- `api/src/state/treasury.rs` - Treasury state
- `api/src/state/vesting.rs` - Vesting state + logic + 10 unit tests
- `api/idl.json` - IDL for client codegen

**CLI (off-chain, unpublished):**
- `cli/src/main.rs` - Admin utility (init, inspect treasury/stake)
- `cli/Cargo.toml` - CLI dependencies

**Configuration:**
- `Cargo.toml` - Workspace config (overflow-checks, LTO, codegen-units)
- `program/Cargo.toml` - Program dependencies
- `api/Cargo.toml` - API dependencies
- `localnet.sh` - Local validator script
- `SECURITY.md` - Vulnerability disclosure policy

---

*This report covers static analysis only. No dynamic testing, fuzzing, or formal verification was performed. It should supplement -- not replace -- a professional security audit.*
