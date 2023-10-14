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
