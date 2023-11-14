#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

mod lang_items;
mod sbi;
mod console;
mod upsync;
mod utils;

// mod batch;
mod loader;
mod task;

mod trap;
mod syscall;
mod timer;
mod config;
mod mem;

#[macro_use]
extern crate alloc;


use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    log!("Hello, {}!", "World");
    mem::init();
    log!("Memory Inited");
    trap::init();
    trap::enable_timer_interrupt();
    log!("Trap Inited");
    timer::set_next_trigger();
    // batch::print_app_info();
    // batch::run_next_app();
    log!("{} apps loaded.", loader::get_num_app());
    loader::list_apps();
    task::add_init_proc();
    task::processor::run_tasks();
    sbi::shutdown();
}

fn clear_bss() {
    extern "C" {
        fn bss_start();
        fn bss_end();
    }
    (bss_start as usize..bss_end as usize).for_each(|p| {
        unsafe {
            (p as *mut u8).write_volatile(0);
        }
    })
}
