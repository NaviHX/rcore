#![allow(unused)]

use core::arch::asm;

use riscv::register::sstatus::{Sstatus, self, SPP};
use bitfield::BitMut;

#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        // set sstatus to U
        let mut bits: u64 = unsafe { core::mem::transmute(sstatus::read()) };
        bits.set_bit(8, false);
        let sstatus = unsafe { core::mem::transmute(bits) };
        let mut ctx = TrapContext {
            x: [0; 32],
            sstatus,
            sepc: entry,
        };
        ctx.set_sp(sp);
        ctx
    }
}
