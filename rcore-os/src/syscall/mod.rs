use self::time::TimeVal;

mod fs;
mod process;
mod time;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => {
            fs::sys_write(args[0], args[1] as *mut u8, args[2])
        }
        SYSCALL_EXIT => {
            process::sys_exit(args[0] as i32)
        }
        SYSCALL_YIELD => {
            process::sys_yield()
        }
        SYSCALL_GET_TIME => {
            time::sys_get_time(args[0] as *mut TimeVal, args[1])
        }
        id => {
            panic!("Unsupported syscall id: {id}")
        }
    }
}
