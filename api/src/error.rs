use steel::*;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, IntoPrimitive)]
#[repr(u32)]
pub enum OreStakeError {
    #[error("No deposits")]
    NoDeposits = 0,
}

error!(OreStakeError);
