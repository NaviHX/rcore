use core::arch::asm;

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use bitflags::bitflags;
use lazy_static::lazy_static;
use riscv::register::satp;

extern "C" {
    fn text_start();
    fn text_end();
    fn rodata_start();
    fn rodata_end();
    fn data_start();
    fn data_end();
    fn bss_with_stack_start();
    fn bss_end();
    fn kernel_end();
    fn trampoline_start();
}

lazy_static! {
    static ref TEXT_START: usize = text_start as usize;
    static ref TEXT_END: usize = text_end as usize;
    static ref RODATA_START: usize = rodata_start as usize;
    static ref RODATA_END: usize = rodata_end as usize;
    static ref DATA_START: usize = data_start as usize;
    static ref DATA_END: usize = data_end as usize;
    static ref BSS_WITH_STACK_START: usize = bss_with_stack_start as usize;
    static ref BSS_END: usize = bss_end as usize;
    static ref KERNEL_END: usize = kernel_end as usize;
    static ref TRAMPOLINE_START: usize = trampoline_start as usize;
}


use crate::{
    config::{MEMORY_END, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT},
    debug,
    loader::USER_STACK_SIZE,
    mem::address::StepByOne,
    upsync::UPSyncCell,
};

use super::{
    address::{PhysAddr, PhysPageNum, VPNRange, VirtAddr, VirtPageNum},
    frame_allocator::{frame_alloc, FrameTracker},
    page_table::{PTEFlags, PageTable, PageTableEntry},
};

pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    #[derive(Clone, Copy)]
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

impl MapArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn = start_va.floor();
        let end_vpn = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_range: VPNRange::new(
                another.vpn_range.get_start(),
                another.vpn_range.get_end(),
            ),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
        }
    }

    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            debug!("Copy data for virtual page 0x{:x}", current_vpn.0);
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_byte_array()[..src.len()];
            debug!("Copy from 0x{:x} to 0x{:x}", src.as_ptr() as usize, dst.as_ptr() as usize);
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
}

impl MapArea {
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum = match self.map_type {
            MapType::Identical => PhysPageNum(vpn.0),
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                let ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
                ppn
            }
        };
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }

    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if let MapType::Framed = self.map_type {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }
}

pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: vec![],
        }
    }

    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
    }

    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    // Without kernel stacks
    pub fn new_kernel() -> Self {
        debug!("Creating kernel memory set!");

        let mut memory_set = Self::new_bare();
        memory_set.map_trampoline();
        debug!(".text {:#x}..{:#x}", text_start as usize, text_end as usize);
        debug!(
            ".rodata {:#x}..{:#x}",
            rodata_start as usize, rodata_end as usize
        );
        debug!(".data {:#x}..{:#x}", data_start as usize, data_end as usize);
        debug!(
            ".bss {:#x}..{:#x}",
            bss_with_stack_start as usize, bss_end as usize
        );

        memory_set.push(
            MapArea::new(
                (*TEXT_START).into(),
                (*TEXT_END).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (*RODATA_START).into(),
                (*RODATA_END).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (*DATA_START).into(),
                (*DATA_END).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (*BSS_WITH_STACK_START).into(),
                (*BSS_END).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        // mapping remaining physical memory
        memory_set.push(
            MapArea::new(
                (*KERNEL_END).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        debug!("Created a new kernel memory set");
        memory_set
    }

    /// return (MemorySet, user_sp, entry_point)
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        debug!("Creating app memory set!");
        let mut memory_set = Self::new_bare();
        memory_set.map_trampoline();

        debug!("Creating elf context");
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        debug!("Get Header");
        let elf_header = elf.header;
        debug!("Get magic");
        let magic = elf_header.pt1.magic;
        debug!("Chekc magic");
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "Invalid ELF header!");

        debug!("Loading elf sections");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = (ph.virtual_addr() + ph.mem_size()).into();

                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }

                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();
                debug!("Loading ELF: Mapping 0x{:x}..0x{:x}", start_va.0, end_va.0);
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }

        // user stack
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();

        // guard page
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;

        // map user stack and guard page
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        // map trap context and trampoline
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        debug!("Created a new app memory set");
        (
            memory_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
        )
    }

    pub fn from_existed_user_space(user_space: &MemorySet) -> Self {
        let mut memory_set = Self::new_bare();
        memory_set.map_trampoline();

        for area in user_space.areas.iter() {
            let new_area = MapArea::from_another(area);
            memory_set.push(new_area, None);

            for vpn in area.vpn_range {
                let src_ppn = user_space.translate(vpn).unwrap().ppn();
                let dst_ppn = memory_set.translate(vpn).unwrap().ppn();
                dst_ppn.get_byte_array().copy_from_slice(src_ppn.get_byte_array());
            }
        }

        memory_set
    }

    pub fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(*TRAMPOLINE_START).into(),
            PTEFlags::R | PTEFlags::X,
        )
    }

    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        if let Some((idx, area)) = self.areas.iter_mut().enumerate().find(|(_, area)| area.vpn_range.get_start() == start_vpn) {
            debug!("Removing area {}: {:?}..{:?}", idx, area.vpn_range.get_start(), area.vpn_range.get_end());
            area.unmap(&mut self.page_table);
            self.areas.remove(idx);

            return;
        }

        panic!("Cannot find area starting with {:?}. Cannot remove!", start_vpn);
    }

    pub fn recycle_data_pages(&mut self) {
        self.areas.clear();
    }
}

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSyncCell<MemorySet>> =
        Arc::new(unsafe { UPSyncCell::new(MemorySet::new_kernel()) });
}
