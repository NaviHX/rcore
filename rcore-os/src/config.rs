pub const MAX_APP_NUM: usize = 10;
pub const APP_BASEADDR: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x200000;

pub const USER_STACK_SIZE: usize = 0x2000;
pub const KERNEL_STACK_SIZE: usize = 0x4000;

/// Clock frequency in qemu
pub const CLOCK_FREQ: usize = 12500000;

pub const KERNEL_HEAP_SIZE: usize = 0x4000;

pub const MEMORY_END: usize = 0x80800000;

pub const PAGE_SIZE: usize = 1 << 12;
pub const TRAP_CONTEXT: usize = usize::MAX - PAGE_SIZE * 2 + 1;
pub const TRAMPOLINE: usize = TRAP_CONTEXT + PAGE_SIZE;

