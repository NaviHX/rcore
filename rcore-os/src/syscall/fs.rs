#![allow(unused)]

use crate::{print, mem::page_table::translate_byte_buffer, task::{processor::current_user_token, suspend_and_run_next}, sbi::console_get_char};

const FD_STDOUT: usize = 1;
const FD_STDIN: usize = 0;

pub fn sys_write(fd: usize, buf: *mut u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = translate_byte_buffer(current_user_token(), buf as *const u8, len);
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

pub fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read");
            let mut c: usize;
            loop {
                c = console_get_char();
                if c == 0 {
                    suspend_and_run_next();
                    continue;
                } else {
                    break;
                }
            };

            let ch = c as u8;
            let mut buffers = translate_byte_buffer(current_user_token(), buf, len);
            unsafe { buffers[0].as_mut_ptr().write_volatile(ch); }
            1
        }
        fd => {
            panic!("Unsupported fd {fd}")
        }
    }
}
