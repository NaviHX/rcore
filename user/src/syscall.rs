use core::arch::asm;

pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id,
        );
    }

    ret
}

#[repr(usize)]
pub enum Syscalls {
    Write = 64,
    Exit = 93,
    Yield = 124,
    GetTime = 169,
    Fork = 220,
    Exec = 221,
    WaitPID = 260,
}

pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    syscall(
        Syscalls::Write as usize,
        [fd, buf.as_ptr() as usize, buf.len()],
    )
}

pub fn sys_exit(xstate: i32) -> isize {
    syscall(Syscalls::Exit as usize, [xstate as usize, 0, 0])
}

pub fn sys_yield() -> isize {
    syscall(Syscalls::Yield as usize, [0, 0, 0])
}

use crate::time::TimeVal;

pub fn sys_get_time(ts: &mut TimeVal, tz: usize) -> isize {
    syscall(Syscalls::GetTime as usize, [ts as *mut _ as usize, tz, 0])
}

pub fn sys_fork() -> isize {
    syscall(Syscalls::Fork as usize, [0, 0, 0])
}

pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(Syscalls::WaitPID as usize, [pid as usize, exit_code as usize, 0])
}

pub fn sys_exec(path: &str) -> isize {
    syscall(Syscalls::Exec as usize, [path.as_ptr() as usize, 0, 0])
}
