use steel::*;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, IntoPrimitive)]
#[repr(u32)]
pub enum OreStakeError {
    #[error("Amount is zero")]
    AmountZero = 0,

    #[error("No deposits")]
    NoDeposits = 1,

    #[error("Insufficient balance")]
    InsufficientBalance = 2,
}

error!(OreStakeError);
