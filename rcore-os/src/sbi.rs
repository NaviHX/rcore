#![allow(unused)]
use core::arch::asm;

pub fn sbi_call(which: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") arg0 => ret,
            in("x11") arg1,
            in("x12") arg2,
            in("x16") fid,
            in("x17") which,
        );
    }
    ret
}

#[repr(usize)]
pub enum Sbi {
    SetTimer = 0,
    ConsolePutChar,
    ConsoleGetChar,
    ClearIpi,
    SendIpi,
    RemoteFenceI,
    RemoteSFenceVma,
    RemoteSFenceVmaAsid,

    SRSTExtension = 0x53525354,
}

#[repr(usize)]
pub enum ExtensionFid {
    Shutdown = 0,
}

pub fn console_put_char(c: usize) {
    sbi_call(Sbi::ConsolePutChar as usize, 0, c, 0, 0);
}

pub fn console_get_char() -> usize {
    sbi_call(Sbi::ConsoleGetChar as usize, 0, 0, 0, 0)
}

pub fn shutdown() -> ! {
    sbi_call(Sbi::SRSTExtension as usize, ExtensionFid::Shutdown as usize, 0, 0, 0);
    panic!("WTF!!? It should shutdown!");
}

pub fn set_timer(timer: usize) {
    // sbi_call(Sbi::SetTimer as usize, 0, timer, 0, 0);
    sbi_rt::set_timer(timer as u64);
}
