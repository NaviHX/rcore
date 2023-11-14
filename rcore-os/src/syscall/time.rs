use crate::{timer::{get_time_us, MICRO_PER_SEC}, utils::{any_as_u8_slice, copy_to_dsts}, mem::page_table::translate, task::processor::current_user_token};

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let usec = get_time_us();
    let sec = usec / MICRO_PER_SEC;
    let src = TimeVal { sec, usec };

    unsafe {
        let src = any_as_u8_slice(&src);
        let mut dsts = translate(current_user_token(), ts);

        match copy_to_dsts(src, &mut dsts[..]) {
            Ok(_) => 0,
            Err(_) => 1,
        }
    }
}
