#![allow(unused)]
use core::arch::{asm, global_asm};

use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
    utvec::TrapMode, sepc, sstatus,
};

use crate::{
    config::{TRAMPOLINE, TRAP_CONTEXT},
    error,
    syscall::syscall,
    task::{processor::{current_trap_context, current_user_token}, exit_and_run_next, suspend_and_run_next},
    timer::set_next_trigger, debug,
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
pub fn trap_handler() -> ! {
    let sepc = sepc::read();
    let sstatus = sstatus::read();
    let scause = scause::read().cause();
    let stval = stval::read();
    // debug!("Sepc: 0x{:x}", sepc);
    // debug!("Sstatus: {:?}", sstatus);
    // debug!("Scause: {:?}", scause);
    // debug!("Stval: 0x{:x}", stval);

    // Ignore traps from kernel
    set_kernel_trap_entry();

    let cx = current_trap_context();
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            let result = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;

            // Syscall may change the memory mapping (e.g exec)
            let cx = current_trap_context();
            cx.x[10] = result as usize;
        }
        Trap::Exception(Exception::StoreFault | Exception::StorePageFault) => {
            error!("PageFault in appication, killed.");
            exit_and_run_next(-2);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            error!("Illegal instruction in application, killed.");
            exit_and_run_next(-3);
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

    trap_return();
}

pub fn enable_timer_interrupt() {
    unsafe { sie::set_stimer() };
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}

pub fn trap_from_kernel() -> ! {
    panic!("A trap from kernel!");
}

pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_context_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();

    extern "C" {
        fn __alltraps();
        fn __restore();
    }

    let restore_addr = __restore as usize - __alltraps as usize + TRAMPOLINE;

    unsafe {
        asm!(
            "fence.i",
            "jr {restore_addr}",
            restore_addr = in(reg) restore_addr,
            in("a0") trap_context_ptr,
            in("a1") user_satp,
            options(noreturn)
        );
    }

    panic!("Cannot return to user code!");
}
