use steel::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum OreInstruction {
    // Misc
    Log = 8,

    // Staker
    Deposit = 10,
    Withdraw = 11,
    ClaimYield = 12,
    CompoundYield = 22,
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

instruction!(OreInstruction, Log);
instruction!(OreInstruction, Deposit);
instruction!(OreInstruction, Withdraw);
instruction!(OreInstruction, ClaimYield);
instruction!(OreInstruction, CompoundYield);
