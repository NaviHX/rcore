#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysAddr(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysPageNum(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtAddr(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VirtPageNum(pub usize);

const PAGE_SIZE_BITS: usize = 12;
const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS;
const PAGE_OFFSET_MASK: usize = PAGE_SIZE - 1;
const PA_WIDTH_SV39: usize = 56;
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;
const VA_WIDTH_SV39: usize = 39;
const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - PAGE_SIZE_BITS;
const VPN_INDEX_WIDTH_SV39: usize = 9;

macro_rules! address_pagenum_impl {
    ($addr:ty, $addr_width:expr, $pn:ty, $pn_width:expr) => {
        impl From<usize> for $addr {
            fn from(v: usize) -> Self {
                Self(v & ((1 << $addr_width) - 1))
            }
        }
        impl From<usize> for $pn {
            fn from(v: usize) -> Self {
                Self(v & ((1 << $pn_width) - 1))
            }
        }

        impl From<$addr> for usize {
            fn from(v: $addr) -> usize {
                v.0
            }
        }
        impl From<$pn> for usize {
            fn from(v: $pn) -> usize {
                v.0
            }
        }

        impl $addr {
            pub fn page_offset(&self) -> usize {
                self.0 & PAGE_OFFSET_MASK
            }
            pub fn floor(&self) -> $pn {
                (self.0 >> PAGE_SIZE_BITS).into()
            }
            pub fn ceil(&self) -> $pn {
                ((self.0 + PAGE_SIZE - 1) >> PAGE_SIZE_BITS).into()
            }
        }

        impl From<$addr> for $pn {
            fn from(v: $addr) -> Self {
                assert_eq!(v.page_offset(), 0);
                v.floor()
            }
        }
        impl From<$pn> for $addr {
            fn from(v: $pn) -> Self {
                Self(v.0 << PAGE_SIZE_BITS)
            }
        }
    };
}

address_pagenum_impl!(PhysAddr, PA_WIDTH_SV39, PhysPageNum, PPN_WIDTH_SV39);
address_pagenum_impl!(VirtAddr, VA_WIDTH_SV39, VirtPageNum, VPN_WIDTH_SV39);

macro_rules! pagenum_impl {
    ($pagenum:ty) => {
        impl $pagenum {
            pub fn get_byte_array(&self) -> &'static mut [u8] {
                let addr: usize = usize::from(self.clone()) << $crate::mem::address::PAGE_SIZE_BITS;
                unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, 4096) }
            }

            pub fn get_mut<T>(&self) -> &'static mut T {
                let addr: usize = usize::from(self.clone()) << $crate::mem::address::PAGE_SIZE_BITS;
                unsafe { (addr as *mut T).as_mut().unwrap() }
            }

            pub fn get_pte_table(&self) -> &'static mut [$crate::mem::page_table::PageTableEntry] {
                let addr: usize = usize::from(self.clone()) << $crate::mem::address::PAGE_SIZE_BITS;
                unsafe {
                    core::slice::from_raw_parts_mut(
                        addr as *mut $crate::mem::page_table::PageTableEntry,
                        512,
                    )
                }
            }
        }
    };
}

pagenum_impl!(PhysPageNum);
pagenum_impl!(VirtPageNum);

impl VirtPageNum {
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        for it in idx.iter_mut().rev() {
            *it = vpn & 0x1ff;
            vpn >>= VPN_INDEX_WIDTH_SV39;
        }
        idx
    }
}
