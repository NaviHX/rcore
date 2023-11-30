use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::{self, Vec},
};
use spin::{Mutex, MutexGuard};

use crate::{
    block_cache::get_block_cache,
    block_dev::BlockDevice,
    efs::EasyFileSystem,
    layout::{DirEntry, DiskInode, DiskInodeType, DIRENTRY_SIZE},
};

pub struct Inode {
    block_id: u32,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id,
            block_offset,
            fs,
            block_device,
        }
    }

    pub fn read_disk_inode_and<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id as usize, self.block_device.clone())
            .lock()
            .read_and(self.block_offset, f)
    }

    pub fn read_disk_inode_mut_and<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id as usize, self.block_device.clone())
            .lock()
            .read_mut_and(self.block_offset, f)
    }

    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs_lock = self.fs.lock();
        self.read_disk_inode_and(|disk_inode| {
            if !disk_inode.is_dir() {
                return None;
            }

            let dir_size = disk_inode.size as usize;
            let count = dir_size / DIRENTRY_SIZE;
            for i in 0..count {
                let mut dir_entry = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(
                        DIRENTRY_SIZE * i,
                        dir_entry.as_bytes_mut(),
                        &self.block_device,
                    ),
                    DIRENTRY_SIZE
                );

                if dir_entry.name() == name {
                    let inode_id = dir_entry.inode_id();
                    let (block_id, block_offset) = fs_lock.get_disk_inode_pos(inode_id);
                    return Some(Arc::new(Inode {
                        block_id,
                        block_offset,
                        fs: self.fs.clone(),
                        block_device: self.block_device.clone(),
                    }));
                }
            }

            None
        })
    }

    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs_lock: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }

        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs_lock.alloc_data_block());
        }

        disk_inode.increase_size(new_size, v, &self.block_device);
    }
}

// util funcs
impl Inode {
    pub fn ls(&self) -> Vec<(String, Arc<Inode>)> {
        let fs_lock = self.fs.lock();
        self.read_disk_inode_and(|disk_inode| {
            let count = disk_inode.size as usize / DIRENTRY_SIZE;
            let mut v = Vec::new();

            for i in 0..count {
                let mut dir_entry = DirEntry::empty();

                assert_eq!(
                    disk_inode.read_at(
                        DIRENTRY_SIZE * i,
                        dir_entry.as_bytes_mut(),
                        &self.block_device,
                    ),
                    DIRENTRY_SIZE
                );

                let (block_id, block_offset) = fs_lock.get_disk_inode_pos(dir_entry.inode_id());
                let name = dir_entry.name().to_string();

                v.push((
                    name,
                    Arc::new(Inode {
                        block_id,
                        block_offset,
                        fs: self.fs.clone(),
                        block_device: self.block_device.clone(),
                    }),
                ))
            }

            v
        })
    }

    pub fn create(&mut self, name: &str) -> Option<Arc<Inode>> {
        let mut fs_lock = self.fs.lock();

        match self
            .read_disk_inode_and(|root_inode| {
                assert!(root_inode.is_dir());
                self.find(name)
            })
            .is_some()
        {
            true => return None,
            false => (),
        }

        let new_inode_id = fs_lock.alloc_inode();
        let (new_inode_block_id, new_inode_block_offset) = fs_lock.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, self.block_device.clone())
            .lock()
            .read_mut_and(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });

        self.read_disk_inode_mut_and(|root_inode| {
            let file_count = (root_inode.size as usize) / DIRENTRY_SIZE;
            let new_size = (file_count + 1) * DIRENTRY_SIZE;

            self.increase_size(new_size as u32, root_inode, &mut fs_lock);
            let new_direntry = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENTRY_SIZE,
                new_direntry.as_bytes(),
                &self.block_device,
            );
        });

        Some(Arc::new(Self::new(
            new_inode_block_id,
            new_inode_block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.read_disk_inode_mut_and(|disk_inode| {
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);

            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data_block(data_block);
            }
        })
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode_and(|disk_inode| {
            disk_inode.read_at(offset, buf, &self.block_device)
        })
    }
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        self.read_disk_inode_mut_and(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        })
    }
}
