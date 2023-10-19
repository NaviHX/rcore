#![allow(unused)]
use core::arch::global_asm;

use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
    utvec::TrapMode,
};

use crate::{
    error,
    syscall::syscall,
    task::{exit_and_run_next, suspend_and_run_next},
    timer::set_next_trigger,
};

use self::context::TrapContext;

pub mod context;

global_asm!(include_str!("trap.asm"));

pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe { stvec::write(__alltraps as usize, TrapMode::Direct) }
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault | Exception::StorePageFault) => {
            error!("PageFault in appication, killed.");
            exit_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            error!("Illegal instruction in application, killed.");
            exit_and_run_next();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_and_run_next();
        }
        _ => {
            error!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }

    cx
}

pub fn enable_timer_interrupt() {
    unsafe { sie::set_stimer() };
}
