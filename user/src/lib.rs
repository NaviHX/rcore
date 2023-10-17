#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]

mod syscall;
pub mod console;
mod lang_items;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    clear_bss();
    exit(main());
    panic!("The application should have exited!");
}

fn clear_bss() {
    extern "C" {
        fn bss_start();
        fn bss_end();
    }

    (bss_start as usize..bss_end as usize).for_each(|p| {
        unsafe { (p as *mut u8).write_volatile(0) }
    })
}

#[no_mangle]
#[linkage = "weak"]
fn main() -> i32 {
    panic!("Cannot find main()");
}

use syscall::*;

pub fn write(fd: usize, buf: &[u8]) -> isize { sys_write(fd, buf)}
pub fn exit(xstate: i32) -> isize { sys_exit(xstate) }
pub fn yield_() -> isize { sys_yield() }
