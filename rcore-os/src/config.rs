pub const MAX_APP_NUM: usize = 10;
pub const APP_BASEADDR: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x200000;

pub const USER_STACK_SIZE: usize = 0x2000;
pub const KERNEL_STACK_SIZE: usize = 0x4000;

/// Clock frequency in qemu
pub const CLOCK_FREQ: usize = 12500000;
