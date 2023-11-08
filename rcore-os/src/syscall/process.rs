#![allow(unused)]

use crate::debug;
use crate::log;
use crate::task::suspend_and_run_next;
use crate::task::exit_and_run_next;

pub fn sys_exit(xstate: i32) -> ! {
    log!("Application exited with code {}", xstate);
    exit_and_run_next();
}

pub fn sys_yield() -> isize {
    debug!("Task yields CPU");
    suspend_and_run_next();
    0
}
