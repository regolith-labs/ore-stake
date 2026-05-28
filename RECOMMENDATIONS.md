# Recommendations

Final actionable items from a full re-scan of the codebase on 2026-05-27. All previously identified issues (zero-amount distribute, dead CLI code, missing writable checks) have been resolved.

---

### 1. BUG - `sdk::compound()` conflates bot signer with stake authority

**File:** `api/src/sdk.rs:123-146`

The `compound(signer: Pubkey)` SDK function uses the single `signer` parameter both as the transaction signer (the bot) **and** to derive the stake PDA via `stake_pda(signer)`. But compound is designed to be permissionless -- a bot compounds *someone else's* stake. The on-chain handler doesn't enforce any authority check on the signer, but the SDK derives the wrong PDA when the caller isn't the stake owner.

A bot calling `compound(bot_pubkey)` would target `stake_pda(bot_pubkey)` instead of the intended user's stake account.

**Fix:** Accept both the bot signer and the stake authority:

```rust
pub fn compound(signer: Pubkey, authority: Pubkey) -> Instruction {
    let stake_address = stake_pda(authority).0;
    let stake_tokens_address = get_associated_token_address(&stake_address, &MINT_ADDRESS);
    // ...
    AccountMeta::new(signer, true),  // bot signs the tx
    // ...
    AccountMeta::new(stake_address, false),  // derived from authority
```

---

### 2. Missing `close` SDK builder

**File:** `api/src/sdk.rs`

The SDK provides instruction builders for all 8 instructions except `close`. Clients that want to close a stake account must build the instruction manually.

**Fix:** Add a `close(signer: Pubkey)` function to `sdk.rs`.

---

### 3. Error code renumbering (conditional)

**File:** `api/src/error.rs`

If the program is already deployed, the pending error renumbering is a breaking change:

| Error | Old code | New code |
|---|---|---|
| `NoDeposits` | `1` | `0` |
| `InsufficientReserves` (was `InsufficientBalance`) | `2` | `1` |

Any off-chain code matching on `custom error: 0x1` would misinterpret the error. If the program is not yet deployed, this is fine as-is.

**Fix (if deployed):** Preserve original discriminants: `NoDeposits = 1, InsufficientReserves = 2`.

---

No other actionable issues found. The on-chain program logic, account validation, arithmetic, and security posture are sound.
