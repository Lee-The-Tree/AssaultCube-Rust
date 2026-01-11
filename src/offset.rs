// AssaultCube v1.3.0.2 (32-bit) Offsets - Verified
pub const GameMode: u32 = 0x18AC00;
pub const PLAYER_COUNT: u32 = 0x18AC0C;
pub const VIEW_MATRIX: u32 = 0x17DFD0;
pub const ENTITY_LIST: u32 = 0x18AC04;
pub const LOCAL_PLAYER: u32 = 0x17E0A8;

pub const ENTITY_HEAD_POSITION: u32 = 0x4;
pub const ENTITY_FEET_POSITION: u32 = 0x28;
pub const ENTITY_HEALTH: u32 = 0xEC;
pub const ENTITY_NAME: u32 = 0x205;
pub const ENTITY_TEAM: u32 = 0x30C;
pub const ENTITY_LAST_VIS_FRAME: u32 = 0xE4;
pub const ENTITY_AMMO: u32 = 0x140;

pub const CURRENT_FRAME: u32 = 0x17F10C; // Based on dword_57F10C (0x57F10C - 0x400000)

pub const LOCAL_YAW: u32 = 0x34;
pub const LOCAL_PITCH: u32 = 0x38;
pub const LOCAL_RECOIL: u32 = 0x40; // Recoil offset

pub const POINTER_SIZE: usize = 4; // 32-bit pointers
