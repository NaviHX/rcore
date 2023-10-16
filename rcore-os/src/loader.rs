use core::arch::asm;

use crate::trap::context::TrapContext;

pub const MAX_APP_NUM: usize = 10;
pub const APP_BASEADDR: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x20000;

/// Load all apps into instruction memory
pub fn load_apps() {
    // Find the addresses of each app.
    extern "C" {
        fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = unsafe { num_app_ptr.read_volatile() };
    let app_start_ptr = unsafe { num_app_ptr.add(1) };
    let app_start = unsafe { core::slice::from_raw_parts(app_start_ptr, num_app + 1) };

    // clear i-cache first
    unsafe {
        asm!("fence.i");
    }
    // Load every app into memory
    for i in 0..num_app {
        unsafe {
            let base_address = get_app_base_address(i);
            (base_address..base_address + APP_SIZE_LIMIT).for_each(|p| unsafe {
                (p as *mut u8).write_volatile(0);
            });

            let src = core::slice::from_raw_parts(
                app_start[i] as *mut u8,
                app_start[i + 1] - app_start[i],
            );
            let dst = core::slice::from_raw_parts_mut(base_address as *mut u8, src.len());
            dst.copy_from_slice(src);
        }
    }
}

fn get_app_base_address(id: usize) -> usize {
    APP_BASEADDR + id * APP_SIZE_LIMIT
}

const USER_STACK_SIZE: usize = 0x2000;
const KERNEL_STACK_SIZE: usize = 0x2000;

struct KernelStack {
    mem: [u8; KERNEL_STACK_SIZE],
}

struct UserStack {
    mem: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    mem: [0; KERNEL_STACK_SIZE],
};

static USER_STACK: UserStack = UserStack {
    mem: [0; USER_STACK_SIZE],
};

macro_rules! stack_impl {
    ($stack:ty, $size:expr) => {
        impl $stack {
            fn get_sp(&self) -> usize {
                self.mem.as_ptr() as usize + $size
            }
        }
    };
}

stack_impl!(KernelStack, KERNEL_STACK_SIZE);
stack_impl!(UserStack, USER_STACK_SIZE);

impl KernelStack {
    fn push_context(&self, ctx: TrapContext) -> &TrapContext {
        let new_sp = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe { *new_sp = ctx };
        unsafe { new_sp.as_mut().unwrap() }
    }
}
