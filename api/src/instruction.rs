use steel::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum OreStakeInstruction {
    // Misc
    Log = 0,
    Initialize = 1,
    Distribute = 2,

    // Staker
    Deposit = 10,
    Withdraw = 11,
    ClaimYield = 12,
    CompoundYield = 13,

    // Migration
    MigrateStake = 20,
    MigrateTreasury = 21,
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
pub struct Initialize {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MigrateStake {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MigrateTreasury {}

instruction!(OreStakeInstruction, Log);
instruction!(OreStakeInstruction, Initialize);
instruction!(OreStakeInstruction, Distribute);
instruction!(OreStakeInstruction, Deposit);
instruction!(OreStakeInstruction, Withdraw);
instruction!(OreStakeInstruction, ClaimYield);
instruction!(OreStakeInstruction, CompoundYield);
instruction!(OreStakeInstruction, MigrateStake);
instruction!(OreStakeInstruction, MigrateTreasury);
