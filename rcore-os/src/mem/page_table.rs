use crate::{mem::address::*, debug};
use alloc::{vec::Vec, string::String};
use bitfield::size_of;
use bitflags::bitflags;

use super::frame_allocator::{frame_alloc, FrameTracker};

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits() as usize,
        }
    }

    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }

    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V).0 != PTEFlags::empty().0
    }
}

pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

// TODO impl recursive mapping
impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().expect("Cannot alloc a frame for root page table!");
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
}

// TODO impl recursive mapping
impl PageTable {
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result = None;

        #[allow(clippy::needless_range_loop)]
        for i in 0..3 {
            let idx = idxs[i];
            let pte = &mut ppn.get_pte_table()[idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().expect("No un-allocated frame for page table entry");
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }

        result
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result = None;

        #[allow(clippy::needless_range_loop)]
        for i in 0..3 {
            let idx = idxs[i];
            let pte = &mut ppn.get_pte_table()[idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }

        result
    }
}

impl PageTable {
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping!", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping!", vpn);
        debug!("Unmap vpn {:?}", vpn);
        *pte = PageTableEntry::empty();
    }
}

impl PageTable {
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }

    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        let vpn: VirtPageNum = va.floor();
        self.translate(vpn).map(|pte| PhysAddr::from(PhysAddr::from(pte.ppn()).0 + va.page_offset()))
    }
}

impl PageTable {
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

pub fn translate_byte_buffer(
    token: usize,
    ptr: *const u8,
    len: usize,
) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = vec![];

    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va = VirtAddr::from(end).min(vpn.into());
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_byte_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_byte_array()[start_va.page_offset()..end_va.page_offset()])
        }

        start = end_va.into();
    }

    v
}

/// T's memory may crosses different pages
pub fn translate<T: Sized>(
    token: usize,
    ptr: *const T,
) -> Vec<&'static mut [u8]> {
    let len = size_of::<T>();
    translate_byte_buffer(token, ptr as *const u8, len)
}

/// T's memory may crosses different pages. Please only use this function with primitive types.
pub unsafe fn translate_raw<T: Sized>(
    token: usize,
    ptr: *const T,
) -> &'static mut T {
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    page_table.translate_va(va.into()).unwrap().get_mut()
}

pub fn translate_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;

    loop {
        let ch: u8 = *(page_table.translate_va(va.into()).unwrap().get_mut());
        if ch == 0 {
            break;
        } else {
            string.push(ch as char);
            va += 1;
        }
    }

    string
}
