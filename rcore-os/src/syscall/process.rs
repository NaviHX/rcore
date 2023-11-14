#![allow(unused)]

use alloc::sync::Arc;

use crate::debug;
use crate::loader::get_app_data_by_name;
use crate::log;
use crate::mem::page_table::translate_raw;
use crate::mem::page_table::translate_str;
use crate::task::manager::add_task;
use crate::task::processor::current_task;
use crate::task::processor::current_user_token;
use crate::task::suspend_and_run_next;
use crate::task::exit_and_run_next;

pub fn sys_exit(xstate: i32) -> ! {
    log!("Application exited with code {}", xstate);
    exit_and_run_next(xstate);
}

pub fn sys_yield() -> isize {
    // debug!("Task yields CPU");
    suspend_and_run_next();
    0
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    let trap_context = new_task.inner_exclusive_access().get_trap_context();

    trap_context.x[10] = 0;
    add_task(new_task);
    new_pid as isize
}

pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    if inner.children.iter().find(|p| pid == -1 || pid as usize == p.get_pid()).is_none() {
        return -1;
    }

    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        p.inner_exclusive_access().is_zombie() && (pid == -1 || p.get_pid() == pid as usize)
    });

    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.get_pid();

        let exit_code_result = child.inner_exclusive_access().exit_code;
        unsafe { *translate_raw(inner.memory_set.token(), exit_code) = exit_code_result; }
        found_pid as isize
    } else {
        -2
    }
}

pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translate_str(token, path);

    if let Some(data) = get_app_data_by_name(&path) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}
