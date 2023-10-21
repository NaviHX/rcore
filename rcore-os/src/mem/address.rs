#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhyAddr(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhyPageNum(pub usize);

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
const VPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;

macro_rules! address_pagenum_impl {
    ($addr:ty, $addr_width:expr, $pn:ty, $pn_width:expr) => {
        impl From<usize> for $addr {
            fn from(v: usize) -> Self { Self (v & ((1 << $addr_width) - 1)) }
        }
        impl From<usize> for $pn {
            fn from(v: usize) -> Self { Self (v & ((1 << $pn_width) - 1)) }
        }

        impl From<$addr> for usize {
            fn from(v: $addr) -> usize { v.0 }
        }
        impl From<$pn> for usize {
            fn from(v: $pn) -> usize { v.0 }
        }

        impl $addr {
            pub fn page_offset(&self) -> usize { self.0 & PAGE_OFFSET_MASK }
            pub fn floor(&self) -> $pn { (self.0 >> PAGE_SIZE_BITS).into() }
            pub fn ceil(&self) -> $pn { ((self.0 + PAGE_SIZE - 1) >> PAGE_SIZE_BITS).into() }
        }

        impl From<$addr> for $pn {
            fn from(v: $addr) -> Self {
                assert_eq!(v.page_offset(), 0);
                v.floor()
            }
        }
        impl From<$pn> for $addr {
            fn from(v: $pn) -> Self { Self(v.0 << PAGE_SIZE_BITS) }
        }
    };
}

address_pagenum_impl!(PhyAddr, PA_WIDTH_SV39, PhyPageNum, PPN_WIDTH_SV39);
address_pagenum_impl!(VirtAddr, VA_WIDTH_SV39, VirtPageNum, VPN_WIDTH_SV39);

