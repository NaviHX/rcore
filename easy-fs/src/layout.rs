use alloc::{sync::Arc, vec::Vec};

use crate::{block_cache::get_block_cache, block_dev::BlockDevice, BLOCK_SIZE};

#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    pub total_blocks: u32,
    pub inode_bitmap_blocks: u32,
    pub inode_area_blocks: u32,
    pub data_bitmap_blocks: u32,
    pub data_area_blocks: u32,
}

pub const EFS_MAGIC: u32 = 0xdead_beef;

impl SuperBlock {
    pub fn initialize(
        &mut self,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
        inode_area_blocks: u32,
        data_bitmap_blocks: u32,
        data_area_blocks: u32,
    ) {
        *self = Self {
            magic: EFS_MAGIC,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

pub const INODE_DIRECT_COUNT: usize = 28;
pub const INODE_INDIRECT1_COUNT: usize = BLOCK_SIZE / core::mem::size_of::<u32>();
pub const INDIRECT1_BOUND: usize = INODE_DIRECT_COUNT + INODE_INDIRECT1_COUNT;
pub const INODE_SIZE: usize = core::mem::size_of::<DiskInode>();
pub const DIRENTRY_SIZE: usize = core::mem::size_of::<DirEntry>();
pub const DIRENTRY_COUNT: usize = BLOCK_SIZE / DIRENTRY_SIZE;
pub const NAME_LENGTH_LIMIT: usize = 28;

pub type IndirectBlock = [u32; INODE_INDIRECT1_COUNT];
pub type DataBlock = [u8; BLOCK_SIZE];
pub type DirBlock = [DirEntry; DIRENTRY_COUNT];

#[repr(C)]
pub struct DirEntry {
    name: [u8; NAME_LENGTH_LIMIT + 1],
    inode_id: u32,
}

impl DirEntry {
    pub fn empty() -> Self {
        Self {
            name: [0u8; NAME_LENGTH_LIMIT + 1],
            inode_id: 0,
        }
    }

    pub fn new(name: &str, inode_id: u32) -> Self {
        let mut bytes = [0u8; NAME_LENGTH_LIMIT + 1];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Self {
            name: bytes,
            inode_id,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, DIRENTRY_SIZE) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, DIRENTRY_SIZE) }
    }

    pub fn name(&self) -> &str {
        let len = (0usize..).find(|i| self.name[*i] == 0).unwrap();
        core::str::from_utf8(&self.name[..len]).unwrap()
    }

    pub fn inode_id(&self) -> u32 {
        self.inode_id
    }
}

#[repr(C)]
pub struct DiskInode {
    pub size: u32,
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
    type_: DiskInodeType,
}

#[derive(PartialEq)]
pub enum DiskInodeType {
    File,
    Directory,
}

impl DiskInode {
    pub fn initialize(&mut self, type_: DiskInodeType) {
        self.size = 0;
        self.direct.iter_mut().for_each(|v| *v = 0);
        self.indirect1 = 0;
        self.indirect2 = 0;
        self.type_ = type_;
    }

    pub fn is_dir(&self) -> bool {
        self.type_ == DiskInodeType::Directory
    }

    pub fn is_file(&self) -> bool {
        self.type_ == DiskInodeType::File
    }

    pub fn get_block_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let inner_id = inner_id as usize;

        if inner_id < INODE_DIRECT_COUNT {
            self.direct[inner_id]
        } else if inner_id < INDIRECT1_BOUND {
            get_block_cache(self.indirect1 as usize, block_device.clone())
                .lock()
                .read_and(0, |indirect_block: &IndirectBlock| {
                    indirect_block[inner_id - INODE_DIRECT_COUNT]
                })
        } else {
            let last = inner_id - INDIRECT1_BOUND;
            let indirect1_id: u32 = get_block_cache(self.indirect2 as usize, block_device.clone())
                .lock()
                .read_and(0, |indirect2_block: &IndirectBlock| {
                    indirect2_block[last / INODE_INDIRECT1_COUNT]
                });
            get_block_cache(indirect1_id as usize, block_device.clone())
                .lock()
                .read_and(0, |indirect1_block: &IndirectBlock| {
                    indirect1_block[last % INODE_INDIRECT1_COUNT]
                })
        }
    }

    pub fn data_blocks(&self) -> u32 {
        Self::_data_blocks(self.size)
    }

    fn _data_blocks(size: u32) -> u32 {
        (size + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32
    }

    pub fn total_blocks(size: u32) -> u32 {
        let data_blocks = Self::_data_blocks(size) as usize;
        let mut total = data_blocks;

        if data_blocks > INODE_DIRECT_COUNT {
            total += 1;
        }

        if data_blocks > INDIRECT1_BOUND {
            total += 1;
            total +=
                (data_blocks - INDIRECT1_BOUND + INODE_INDIRECT1_COUNT - 1) / INODE_INDIRECT1_COUNT;
        }

        total as u32
    }

    pub fn blocks_num_needed(&self, new_size: u32) -> u32 {
        assert!(new_size >= self.size);
        Self::total_blocks(new_size) - Self::total_blocks(self.size)
    }

    pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        let mut current_blocks = self.data_blocks();
        self.size = new_size;
        let mut total_blocks = self.data_blocks();
        let mut new_blocks = new_blocks.into_iter();

        // fill direct
        while current_blocks < total_blocks.min(INODE_DIRECT_COUNT as u32) {
            self.direct[current_blocks as usize] = new_blocks.next().unwrap();
            current_blocks += 1;
        }

        // alloc indirect1
        if total_blocks > INODE_DIRECT_COUNT as u32 {
            if current_blocks == INODE_DIRECT_COUNT as u32 {
                self.indirect1 = new_blocks.next().unwrap();
            }
            current_blocks -= INODE_DIRECT_COUNT as u32;
            total_blocks -= INODE_DIRECT_COUNT as u32;
        } else {
            return;
        }

        // fill indirect1
        get_block_cache(self.indirect1 as usize, block_device.clone()).lock().read_mut_and(0, |indirect1: &mut IndirectBlock| {
            while current_blocks < total_blocks.min(INODE_INDIRECT1_COUNT as u32) {
                indirect1[current_blocks as usize] = new_blocks.next().unwrap();
                current_blocks += 1;
            }
        });

        // alloc indirect2
        if total_blocks > INODE_INDIRECT1_COUNT as u32 {
            if current_blocks == INODE_INDIRECT1_COUNT as u32 {
                self.indirect2 = new_blocks.next().unwrap();
            }

            current_blocks -= INODE_INDIRECT1_COUNT as u32;
            total_blocks -= INODE_INDIRECT1_COUNT as u32;
        } else {
            return;
        }

        // fill indirect2 from (a0, b0) to (a1, b1)
        let mut a0 = current_blocks as usize / INODE_INDIRECT1_COUNT;
        let mut b0 = current_blocks as usize % INODE_INDIRECT1_COUNT;
        let a1 = total_blocks as usize / INODE_INDIRECT1_COUNT;
        let b1 = total_blocks as usize % INODE_INDIRECT1_COUNT;
        get_block_cache(self.indirect2 as usize, block_device.clone()).lock().read_mut_and(0, |indirect2: &mut IndirectBlock| {
            while (a0 < a1) || (a0 == a1 && b0 < b1) {
                if b0 == 0 {
                    indirect2[a0] = new_blocks.next().unwrap();
                }

                get_block_cache(indirect2[a0] as usize, block_device.clone()).lock().read_mut_and(0, |indirect1: &mut IndirectBlock| {
                    indirect1[b0] = new_blocks.next().unwrap();
                });

                b0 += 1;
                if b0 == INODE_INDIRECT1_COUNT {
                    b0 = 0;
                    a0 += 1;
                }
            }
        });
    }

    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        let mut freed_blocks: Vec<u32> = Vec::new();
        let mut data_blocks = self.data_blocks();
        self.size = 0;

        let mut current_block = 0usize;

        // clear direct block
        while current_block < (data_blocks as usize).min(INODE_DIRECT_COUNT) {
            freed_blocks.push(self.direct[current_block]);
            self.direct[current_block] = 0;
            current_block += 1;
        }

        // clear indirect1 block
        if data_blocks > INODE_DIRECT_COUNT as u32 {
            freed_blocks.push(self.indirect1);
            data_blocks -= INODE_DIRECT_COUNT as u32;
            current_block = 0;
        } else {
            return freed_blocks;
        }

        get_block_cache(self.indirect1 as usize, block_device.clone()).lock().read_mut_and(0, |indirect1: &mut IndirectBlock| {
            while current_block < (data_blocks as usize).min(INODE_INDIRECT1_COUNT) {
                freed_blocks.push(indirect1[current_block]);
                current_block += 1;
            }
        });
        self.indirect1 = 0;

        // clear indirect2 block
        if data_blocks > INODE_INDIRECT1_COUNT as u32 {
            freed_blocks.push(self.indirect2);
            data_blocks -= INODE_INDIRECT1_COUNT as u32;
        } else {
            return freed_blocks;
        }

        let a1 = data_blocks / INODE_INDIRECT1_COUNT as u32;
        let b1 = data_blocks % INODE_INDIRECT1_COUNT as u32;
        get_block_cache(self.indirect2 as usize, block_device.clone()).lock().read_mut_and(0, |indirect2: &mut IndirectBlock| {
            for entry in indirect2.iter_mut().take(a1 as usize) {
                freed_blocks.push(*entry);
                get_block_cache((*entry) as usize, block_device.clone()).lock().read_mut_and(0, |indirect1: &mut IndirectBlock| {
                    for entry in indirect1.iter() {
                        freed_blocks.push(*entry);
                    }
                });
            }

            if b1 > 0 {
                freed_blocks.push(indirect2[a1 as usize]);
                get_block_cache(indirect2[a1 as usize] as usize, block_device.clone()).lock().read_mut_and(0, |indirect1: &mut IndirectBlock| {
                    for entry in indirect1.iter().take(b1 as usize) {
                        freed_blocks.push(*entry);
                    }
                });
            }
        });
        self.indirect2 = 0;

        freed_blocks
    }

    pub fn read_at(
        &self,
        offset: usize,
        buf: &mut [u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        let len = buf.len();
        let end = (offset + len).min(self.size as usize);

        if start >= end {
            return 0;
        }

        let mut start_block = start / BLOCK_SIZE;
        let mut read_size = 0usize;
        loop {
            let mut end_current_block = (start / BLOCK_SIZE + 1) * BLOCK_SIZE;
            end_current_block = end_current_block.min(end);

            let block_read_size = end_current_block - start;
            let dst = &mut buf[read_size..read_size + block_read_size];
            get_block_cache(
                self.get_block_id(start_block as u32, block_device) as usize,
                block_device.clone(),
            )
            .lock()
            .read_and(0, |data_block: &DataBlock| {
                let src = &data_block[start % BLOCK_SIZE..start % BLOCK_SIZE + block_read_size];
                dst.copy_from_slice(src);
            });

            read_size += block_read_size;
            if end_current_block == end {
                break read_size;
            }
            start_block += 1;
            start = end_current_block;
        }
    }

    pub fn write_at(
        &mut self,
        offset: usize,
        buf: &[u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        let len = buf.len();
        let end = (start + len).min(self.size as usize);

        if start >= end {
            return 0;
        }

        let mut start_block = start / BLOCK_SIZE;
        let mut write_size = 0usize;

        loop {
            let mut end_current_block = (start / BLOCK_SIZE + 1) * BLOCK_SIZE;
            end_current_block = end_current_block.min(end);

            let block_write_size = end_current_block - start;
            let src = & buf[write_size..write_size + block_write_size];
            get_block_cache(
                self.get_block_id(start_block as u32, block_device) as usize,
                block_device.clone(),
            )
            .lock()
            .read_mut_and(0, |data_block: &mut DataBlock| {
                let dst = &mut data_block[start % BLOCK_SIZE..start % BLOCK_SIZE + block_write_size];
                dst.copy_from_slice(src);
            });

            write_size += block_write_size;
            if end_current_block == end {
                break write_size;
            }
            start_block += 1;
            start = end_current_block;
        }
    }
}
