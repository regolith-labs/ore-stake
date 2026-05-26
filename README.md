# ORE Stake

A staking protocol for ORE holders to receive a share of protocol revenues. 

## API
- [`Consts`](api/src/consts.rs) – Program constants.
- [`Error`](api/src/error.rs) – Custom program errors.
- [`Event`](api/src/event.rs) – Custom program events.
- [`Instruction`](api/src/instruction.rs) – Declared instructions and arguments.

## Instructions

#### Admin
- [`Claim`](program/src/claim.rs) – Claims accrued staking rewards.
- [`Close`](program/src/close.rs) – Closes a stake account and reclaims rent.
- [`Compound`](program/src/compound.rs) – Compounds accrued rewards back into the stake deposit.
- [`Deposit`](program/src/deposit.rs) – Deposits ORE into a stake account.
- [`Distribute`](program/src/distribute.rs) – Distributes ORE to the treasury for vesting to stakers.
- [`Init`](program/src/init.rs) – Initializes the program (treasury, treasury token account, and vesting account).
- [`Log`](program/src/log.rs) – Logs non-truncatable event data.
- [`Withdraw`](program/src/withdraw.rs) – Withdraws ORE from a stake account.

## State
- [`Stake`](api/src/state/stake.rs) – A user's staking position (balance, rewards, compound fee settings).
- [`Treasury`](api/src/state/treasury.rs) – Singleton tracking the global rewards factor and total staked amount.
- [`Vesting`](api/src/state/vesting.rs) – Singleton tracking the current reward vesting schedule.

## Tests

To run the test suite, use the Solana toolchain:

```
cargo test-sbf
```

For line coverage, use llvm-cov:

```
cargo llvm-cov
```
