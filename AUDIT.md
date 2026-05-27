# ORE Stake Program - Pre-Freeze Security Audit

**Program ID:** `STkEAu2cEyQp5ktgUauRVq8es6mEP2w6ixw4NEd5tDJ`
**Commit:** `205eb6b` + pending changes
**Date:** 2026-05-26
**Context:** Final review before revoking upgrade authority
**Auditor:** Claude Opus 4.6 (automated static analysis)

---

## Summary

The ore-stake program is a Solana staking contract for the ORE token built on the Steel framework. Users deposit ORE, earn proportional rewards from distributions via a 1-hour linear vesting schedule, and can claim or auto-compound rewards. The contract uses a rewards-factor accumulator pattern common in DeFi staking designs.

| Severity | Count |
|----------|-------|
| Critical | 0 |
| High | 0 |
| Medium | 0 |
| Low | 1 |
| Info | 14 |

**Overall assessment:** The contract is well-structured with proper PDA validation, authority checks, overflow protection, and token balance safety checks. No critical, high, or medium issues were found. The codebase is suitable for freezing.

---

## Findings

### LOW-1: No way to update compound settings without depositing tokens

**File:** `program/src/deposit.rs:89-92`

**Description:** The `compound_fee` and `compound_fee_reserve` are only settable via the `deposit` instruction. A staker who wants to change their compound fee (e.g., disable auto-compounding by setting it to 0) must make a new deposit. The `AmountZero` check has been removed, so users can now call deposit with `amount=0` to update settings without moving tokens. However, `claim` and `withdraw` still reject `amount=0`, so those cannot be used as settings-update paths.

**Impact:** Minimal. Deposit with `amount=0` serves as the settings update path. Users may need to be informed of this workflow.

---

### INFO-1: Overflow protection enabled

**File:** `Cargo.toml:39,49`

Both `[profile.release]` and `[profile.dev]` have `overflow-checks = true`. All arithmetic panics and reverts the transaction on overflow rather than silently wrapping. This is essential for financial programs.

---

### INFO-2: Init is permissionless and idempotent

**File:** `program/src/init.rs`

Anyone can call `init`. This is safe because all accounts are created at deterministic PDA addresses, creation is guarded by `data_is_empty()` checks, and initial state values are hardcoded constants (zero). Calling init on already-initialized accounts simply validates them.

---

### INFO-3: Distribute is permissionless

**File:** `program/src/distribute.rs`

Anyone can distribute rewards to stakers by transferring ORE from their own token account. This is by design. The `total_staked > 0` check prevents distributions when there are no stakers.

---

### INFO-4: Compound is permissionless (by design)

**File:** `program/src/compound.rs`

Compound is callable by anyone for any staker. Stakers opt in by setting `compound_fee > 0` during deposit. A `ONE_DAY` cooldown since last claim prevents excessive compounding. The fee is paid from a staker-funded lamport reserve on the stake PDA. Compound always compounds all available rewards (`u64::MAX`).

---

### INFO-5: Vesting rewards intentionally lost when total_staked is zero

**File:** `api/src/state/vesting.rs:30-32`

When `treasury.total_staked == 0`, vesting advances (`vested_amount` is updated) but `rewards_factor` is not incremented. Tokens that vest during a zero-staked window are not distributed to anyone. This is accepted by design -- rewards should not retroactively accrue to future stakers who were not present during vesting.

---

### INFO-6: Vesting schedule merges on rapid distributions

**File:** `program/src/distribute.rs:45-53`

When `distribute` is called during an active vesting period (< 1 hour since last distribution), the new tokens are merged into the existing schedule. The math correctly preserves the `vested_amount` invariant. The merged schedule results in remaining tokens (old + new) vesting over the remaining time, effectively compressing the vesting window for newly added tokens. Distributions spaced >= 1 hour apart each get a full independent vesting period.

---

### INFO-7: Two hardcoded time constants

**File:** `api/src/consts.rs:14,20`

| Constant | Value | Usage |
|----------|-------|-------|
| `ONE_HOUR` | 3600s | Vesting duration for distributed rewards |
| `ONE_DAY` | 86400s | Compound cooldown (minimum time between auto-compounds) |

These cannot be changed post-freeze.

---

### INFO-8: PDA address validation

Treasury and Vesting singleton accounts are validated via `has_address(&treasury_pda().0)` and `has_address(&vesting_pda().0)` in all mutating instructions (claim, compound, deposit, distribute, withdraw). Stake accounts are validated via discriminator + program ownership + authority check. Token accounts are validated as proper ATAs. Mint is validated against the hardcoded `MINT_ADDRESS`.

---

### INFO-9: Token balance safety checks

**Files:** `program/src/deposit.rs:94-99`, `program/src/withdraw.rs:66-71`

Both deposit and withdraw include post-operation safety checks verifying `stake_tokens.amount() >= stake.balance`. This guards against any accounting drift between on-chain token state and program state.

---

### INFO-10: No reentrancy risk

All CPIs are to trusted system programs (SPL Token, Associated Token Account, System Program). None call back to the ore-stake program. The program's own `Log` instruction is called via CPI but is a no-op that only validates the Treasury PDA signer.

---

### INFO-11: Close requires zero balance and zero rewards

**File:** `program/src/close.rs:18`

The `close` instruction requires `balance == 0 && rewards == 0`. It does not call `update_rewards`, but this is correct: with `balance == 0`, any pending rewards from a `rewards_factor` delta would compute to `delta * 0 = 0`, so no rewards can be missed. Any dust tokens remaining in the stake token account are swept to the recipient before closing.

---

### INFO-12: Security.txt configured

**File:** `program/src/lib.rs:46-53`

The program includes a properly configured `security_txt!` macro with contact info (`hardhatchad@gmail.com`), project URL, source code link, and security policy.

---

### INFO-13: Event discriminators are hardcoded

**Files:** `program/src/claim.rs:72`, `deposit.rs:105`, `distribute.rs:68`, `withdraw.rs:77`, `compound.rs:72`

Event `disc` values (0, 1, 2, 3, 4) are hardcoded literals in instruction handlers rather than derived from the `OreStakeEvent` enum. Since the contract is being frozen, these values are permanently fixed and cannot drift.

---

### INFO-14: Compound rent exemption check

**File:** `program/src/compound.rs:58-62`

The compound instruction verifies `stake_info.lamports() - compound_fee >= minimum_rent` before deducting the compound fee. This ensures the stake PDA always maintains rent-exempt status after paying the bot. The assertion `compound_fee_reserve >= compound_fee` at the top ensures the fee is covered by deposited SOL.

---

## MEV and Timing Attack Analysis

### Sandwich attack on distribute

An attacker could attempt to deposit before a `distribute` and withdraw after to capture a disproportionate share of rewards.

**Mitigated by two defenses:**

1. **vest-before-balance ordering.** In every instruction, `stake.deposit()` / `stake.withdraw()` / `stake.claim()` calls `update_rewards()` which calls `vest()` **before** modifying `balance` or `total_staked`. When the attacker's front-run deposit executes, pending vesting settles using the pre-attack `total_staked`. The attacker's balance is 0 during this settlement, so they earn nothing from prior vesting.

2. **1-hour linear vesting.** After `distribute` sets `start_time = now`, the attacker's back-run withdraw calls `vest()` in the same block. With `time_elapsed = 0`, `vested_amount = 0` -- zero rewards are available. The attacker must remain staked for the full hour to capture their proportional share, at which point they are simply a regular staker.

### Front-running vesting completion

An attacker deposits at T+3599 (1 second before vesting completes) to capture the final vesting increment.

**Mitigated:** The deposit triggers `vest()` at the pre-attack `total_staked`, settling 3599/3600 of the distribution to existing stakers. After the attacker's balance is added, only 1/3600 of the distribution remains, split across the now-diluted pool. The attacker's return is `(attacker_balance / new_total_staked) * (initial_amount / 3600)` -- negligible relative to the capital deployed.

### Malicious leader clock manipulation

Solana validators have bounded discretion over slot timestamps. A malicious leader could push the clock forward by a few seconds to vest slightly more tokens in their slot.

**Bounded impact:** The Solana runtime constrains timestamps to be roughly monotonic and close to wall-clock time. The maximum gain is a few seconds of accelerated vesting (~0.1% of one distribution). This shifts timing but does not create tokens or steal from other stakers.

### Compound fee sniping

A malicious leader could censor competing bots' compound transactions and submit their own to collect the fee.

**No protocol impact:** The staker outcome is identical regardless of which bot compounds -- same rewards re-deposited, same fee deducted. This is standard validator MEV, not a protocol vulnerability.

### total_staked manipulation

An attacker with a large share of the pool could withdraw to deflate `total_staked`, hoping to later re-deposit and benefit from the higher per-unit rewards distributed during their absence.

**Self-defeating:** Withdraw calls `update_rewards` and syncs `rewards_factor` before reducing balance. The attacker forfeits all rewards that vest during their withdrawal. Re-depositing syncs to the new baseline. The remaining stakers benefit at the attacker's expense.

### Treasury solvency under rounding

The Numeric type (I80F48 fixed-point) truncates toward zero on division, multiplication, and `to_u64()` conversion. The full arithmetic chain is:

1. `from_fraction(amount, total_staked)` -- truncates down
2. `accumulated_rewards - self.rewards_factor` -- exact (same-precision subtraction)
3. `accumulated * from_u64(balance)` -- truncates down
4. `to_u64()` -- truncates fractional part

All rounding is in the protocol's favor. The sum of all user rewards across the pool is always **less than or equal to** total distributed tokens. The treasury accumulates a small dust surplus over time and can never become insolvent from rounding.

---

## Changes Made During Audit

The following changes were applied during this audit session and are included in the reviewed code:

| Change | File(s) | Description |
|--------|---------|-------------|
| Clock guard in vest() | `api/src/state/vesting.rs:22-24` | Early return if `clock.unix_timestamp < self.start_time`, preventing underflow on `clock - start_time` if the Solana clock ever drifts backward. Replaces previous `.max(0)` + `.saturating_sub()` approach. |
| Remove AmountZero check from deposit | `program/src/deposit.rs` | Allows `amount=0` deposits so users can update compound fee settings without transferring tokens. |
| Guard timestamp updates on amount > 0 | `api/src/state/stake.rs:56-59,73-77,90-94` | `last_claim_at`, `last_deposit_at`, and `last_withdraw_at` only update when the corresponding operation moves a non-zero amount. Prevents zero-amount operations from resetting timestamps (e.g., compound no longer resets `last_claim_at` when there are no rewards to compound). |

---

## Instruction-by-Instruction Validation

### Init (`program/src/init.rs`)

| Check | Status |
|-------|--------|
| Signer validation | Signer required |
| Mint validation | Hardcoded MINT_ADDRESS |
| PDA creation | Deterministic seeds (TREASURY, VESTING) |
| Idempotent | data_is_empty() guards |
| Program validation | system_program, token_program, associated_token_program |
| State initialization | Zeroed defaults |

### Deposit (`program/src/deposit.rs`)

| Check | Status |
|-------|--------|
| Signer validation | signer + payer both required |
| Mint validation | Hardcoded MINT_ADDRESS |
| Sender validation | ATA of signer for MINT_ADDRESS |
| Stake PDA | Created with [STAKE, signer_key] seeds; existing validated by ownership + authority |
| Treasury PDA | has_address(&treasury_pda().0) |
| Vesting PDA | has_address(&vesting_pda().0) |
| Amount clamping | min(sender.amount(), requested) |
| Token transfer | SPL transfer from sender to stake_tokens |
| Balance safety check | stake_tokens.amount() >= stake.balance |
| Program validation | system, token, ata, ore_stake |

### Withdraw (`program/src/withdraw.rs`)

| Check | Status |
|-------|--------|
| Signer validation | Required, must be stake authority |
| Amount validation | AmountZero check |
| Amount clamping | min(balance, requested) |
| Stake PDA | Ownership + authority check |
| Treasury/Vesting PDA | has_address checks |
| Token transfer | Signed transfer with PDA seeds |
| Balance safety check | stake_tokens.amount() >= stake.balance |
| Recipient validation | ATA created or validated |

### Claim (`program/src/claim.rs`)

| Check | Status |
|-------|--------|
| Signer validation | Required, must be stake authority |
| Amount validation | AmountZero check |
| Amount clamping | min(rewards, requested) |
| Stake PDA | Ownership + authority check |
| Treasury/Vesting PDA | has_address checks |
| Treasury tokens | Validated as ATA of treasury |
| Token transfer | Signed transfer from treasury with PDA seeds |
| Recipient validation | ATA created or validated |

### Compound (`program/src/compound.rs`)

| Check | Status |
|-------|--------|
| Signer validation | Any signer (bot) |
| Opt-in check | compound_fee > 0 |
| Fee reserve check | compound_fee_reserve >= compound_fee |
| Cooldown check | last_claim_at + ONE_DAY < now |
| Zero-reward guard | Early return if claim returns 0 (no fee paid) |
| Rent exemption | lamports - fee >= minimum_rent |
| Treasury/Vesting PDA | has_address checks |
| Token transfers | Signed from treasury to stake_tokens |
| Fee payment | stake_info.send(compound_fee, signer) |

### Distribute (`program/src/distribute.rs`)

| Check | Status |
|-------|--------|
| Signer validation | Required |
| Amount validation | AmountZero check |
| Sender balance | assert(amount() >= amount) |
| Stakers exist | total_staked > 0 required |
| Treasury/Vesting PDA | has_address checks |
| Vesting settlement | vest() called before schedule update |
| Schedule math | Preserves vested_amount invariant on merge |
| Token transfer | SPL transfer from sender to treasury_tokens |
| Overflow protection | u128 intermediate arithmetic, overflow-checks=true |

### Close (`program/src/close.rs`)

| Check | Status |
|-------|--------|
| Signer validation | Required, must be stake authority |
| Zero balance | balance == 0 required |
| Zero rewards | rewards == 0 required |
| Dust sweep | Remaining stake_tokens transferred to recipient |
| Token account close | Signed with PDA seeds |
| Stake account close | Rent returned to signer |
| Recipient validation | ATA created or validated (only if tokens exist) |

### Log (`program/src/log.rs`)

| Check | Status |
|-------|--------|
| Signer validation | Must be Treasury PDA (only callable via internal CPI) |
| Side effects | None (no-op, data stored in tx log) |

---

## Scope

This audit covers all on-chain source files in `api/src/` and `program/src/`. The CLI (`cli/src/main.rs`) was excluded as an off-chain client. This is a static code review -- no dynamic testing, fuzzing, or formal verification was performed.

### Files reviewed

**API (state, instructions, SDK):**
- `api/src/lib.rs` - Program ID declaration
- `api/src/consts.rs` - Constants (decimals, time durations, seeds, mint address)
- `api/src/error.rs` - Custom error types
- `api/src/event.rs` - Event structs for logging
- `api/src/instruction.rs` - Instruction data structs and discriminators
- `api/src/sdk.rs` - Client-side instruction builders
- `api/src/state/mod.rs` - Account types, PDA derivation functions
- `api/src/state/treasury.rs` - Treasury state (rewards_factor, total_staked)
- `api/src/state/stake.rs` - Stake state and core logic (claim, deposit, withdraw, update_rewards)
- `api/src/state/vesting.rs` - Vesting state and vest() logic

**Program (instruction handlers):**
- `program/src/lib.rs` - Entrypoint and instruction dispatch
- `program/src/init.rs` - Initialize treasury, vesting, and token accounts
- `program/src/deposit.rs` - Deposit ORE into staking
- `program/src/withdraw.rs` - Withdraw ORE from staking
- `program/src/claim.rs` - Claim accrued rewards
- `program/src/compound.rs` - Auto-compound rewards (bot-callable)
- `program/src/distribute.rs` - Distribute new rewards to stakers
- `program/src/close.rs` - Close empty stake accounts
- `program/src/log.rs` - Internal logging via CPI

**Configuration:**
- `Cargo.toml` - Workspace config, overflow-checks, dependencies
- `program/Cargo.toml` - Program dependencies
- `api/Cargo.toml` - API dependencies

---

*This audit was performed by an AI system and should be supplemented with human expert review, dynamic testing, and formal verification before finalizing the freeze decision.*
