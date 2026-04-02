# ORE

ORE Stake is a staking protocol for ORE.

## API
- [`Consts`](api/src/consts.rs) – Program constants.
- [`Error`](api/src/error.rs) – Custom program errors.
- [`Event`](api/src/error.rs) – Custom program events.
- [`Instruction`](api/src/instruction.rs) – Declared instructions and arguments.

## Instructions


#### Staking
- [`Deposit`](program/src/deposit.rs) - Deposits ORE into a stake account.
- [`Withdraw`](program/src/withdraw.rs) - Withdraws ORE from a stake account.
- [`ClaimSeeker`](program/src/claim_seeker.rs) - Claims a Seeker genesis token. 
- [`ClaimYield`](program/src/claim_yield.rs) - Claims staking yield.

#### Misc
- [`Log`](program/src/log.rs) – Logs non-truncatable event data.


## State
- [`Stake`](api/src/state/stake.rs) - Manages a user's staking activity.
- [`Treasury`](api/src/state/treasury.rs) - Mints, burns, and escrows ORE tokens. 


## Tests

To run the test suite, use the Solana toolchain: 

```
cargo test-sbf
```

For line coverage, use llvm-cov:

```
cargo llvm-cov
```
