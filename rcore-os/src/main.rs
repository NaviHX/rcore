#![no_std]
#![no_main]
#![feature(panic_info_message)]

mod lang_items;
mod sbi;
mod console;
mod upsync;

// mod batch;
mod loader;
mod task;

mod trap;
mod syscall;
mod timer;
mod config;


use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    log!("Hello, {}!", "World");
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    // batch::print_app_info();
    // batch::run_next_app();
    log!("{} apps loaded.", loader::get_num_apps());
    task::run_first_task();
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
