use crate::syscall::*;

pub fn yield_() -> isize {
    sys_yield()
}
pub fn fork() -> isize {
    sys_fork()
}

pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => yield_(),
            exit_pid => return exit_pid,
        };
    }
}

pub fn waitpid(pid: isize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid, exit_code as *mut _) {
            -2 => yield_(),
            exit_pid => return exit_pid,
        };
    }
}

pub fn exec(path: &str) -> isize {
    sys_exec(path)
}
