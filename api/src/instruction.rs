use steel::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum OreStakeInstruction {
    // Misc
    Log = 0,
    Init = 1,
    Distribute = 2,

    // Staker
    Deposit = 10,
    Withdraw = 11,
    ClaimYield = 12,
    CompoundYield = 13,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Log {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Deposit {
    pub amount: [u8; 8],
    pub compound_fee: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Withdraw {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ClaimYield {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CompoundYield {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Distribute {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Init {}

instruction!(OreStakeInstruction, Log);
instruction!(OreStakeInstruction, Init);
instruction!(OreStakeInstruction, Distribute);
instruction!(OreStakeInstruction, Deposit);
instruction!(OreStakeInstruction, Withdraw);
instruction!(OreStakeInstruction, ClaimYield);
instruction!(OreStakeInstruction, CompoundYield);
