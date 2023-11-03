#![allow(unused)]

use crate::{print, mem::page_table::translate_byte_buffer, task::get_current_token};

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buf: *mut u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = translate_byte_buffer(get_current_token(), buf as *const u8, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        fd => {
            panic!("Unsupported fd {fd}")
        }
    }
}
