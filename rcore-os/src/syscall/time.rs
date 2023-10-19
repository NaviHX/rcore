use crate::timer::{get_time_us, MICRO_PER_SEC};

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let usec = get_time_us();
    let sec = usec / MICRO_PER_SEC;

    unsafe {
        let ts = &mut *ts;
        ts.sec = sec;
        ts.usec = usec;
    }

    0
}
