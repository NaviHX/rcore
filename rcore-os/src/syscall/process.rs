#![allow(unused)]

use crate::{log, batch::run_next_app};

pub fn sys_exit(xstate: i32) -> ! {
    log!("Application exited with code {}", xstate);
    run_next_app()
}
