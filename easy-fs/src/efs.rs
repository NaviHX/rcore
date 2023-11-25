use alloc::sync::Arc;
use spin::Mutex;

use crate::{
    bitmap::DiskBitmap,
    block_cache::get_block_cache,
    block_dev::BlockDevice,
    layout::{DataBlock, DiskInode, DiskInodeType, SuperBlock, INODE_SIZE},
    BLOCK_SIZE,
};

pub struct EasyFileSystem {
    pub block_device: Arc<dyn BlockDevice>,
    pub inode_bitmap: DiskBitmap,
    pub data_bitmap: DiskBitmap,

    inode_area_start_block: u32,
    data_area_start_block: u32,
}

impl EasyFileSystem {
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        let inode_bitmap = DiskBitmap::new(1_usize, inode_bitmap_blocks as usize);
        let inode_num = inode_bitmap.maxium();
        let inode_area_blocks = (inode_num * INODE_SIZE + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let inode_total_blocks = inode_bitmap_blocks as usize + inode_area_blocks;

        let data_total_blocks = total_blocks as usize - inode_total_blocks - 1;
        let data_bitmap_blocks = (data_total_blocks + 4096) / 4097;
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;
        let data_bitmap = DiskBitmap::new(1usize + inode_total_blocks, data_bitmap_blocks);

        let mut efs = Self {
            block_device: block_device.clone(),
            inode_bitmap,
            data_bitmap,

            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks as u32 + data_bitmap_blocks as u32,
        };

        // clear all blocks
        for i in 0..total_blocks as usize {
            get_block_cache(i, block_device.clone())
                .lock()
                .read_mut_and(0, |block: &mut DataBlock| {
                    for b in block.iter_mut() {
                        *b = 0;
                    }
                })
        }

        // initialize super block
        get_block_cache(0, block_device.clone())
            .lock()
            .read_mut_and(0, |super_block: &mut SuperBlock| {
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks as u32,
                    data_bitmap_blocks as u32,
                    data_area_blocks as u32,
                );
            });

        // alloc an inode for root
        assert_eq!(efs.alloc_inode(), 0);
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        get_block_cache(root_inode_block_id as usize, block_device.clone())
            .lock()
            .read_mut_and(root_inode_offset, |inode: &mut DiskInode| {
                inode.initialize(DiskInodeType::Directory);
            });

        Arc::new(Mutex::new(efs))
    }

    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        get_block_cache(0, block_device.clone())
            .lock()
            .read_and(0, |super_block: &SuperBlock| {
                assert!(super_block.is_valid());
                let inode_total_blocks =
                    super_block.inode_area_blocks + super_block.inode_bitmap_blocks;
                let efs = Self {
                    block_device,
                    inode_bitmap: DiskBitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: DiskBitmap::new(
                        inode_total_blocks as usize + 1,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                };

                Arc::new(Mutex::new(efs))
            })
    }

    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }

    pub fn alloc_data_block(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }

    pub fn dealloc_data_block(&mut self, block_id: u32) {
        get_block_cache(block_id as usize, self.block_device.clone())
            .lock()
            .read_mut_and(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|b| *b = 0);
            });

        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_area_start_block) as usize,
        );
    }

    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {
        let inodes_per_block = (BLOCK_SIZE / INODE_SIZE) as u32;
        let block_id = inode_id / inodes_per_block + self.inode_area_start_block;
        (
            block_id,
            (inode_id % inodes_per_block) as usize * INODE_SIZE,
        )
    }

    pub fn get_data_block_id(&self, data_block_id: u32) -> u32 {
        self.data_area_start_block + data_block_id
    }
}
