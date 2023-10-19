use crate::syscall::sys_get_time;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn get_time() -> TimeVal {
    let mut time_val = TimeVal { sec: 0, usec: 0 };
    sys_get_time(&mut time_val, 0);

    time_val
}
