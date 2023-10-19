#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{yield_, time::get_time};

#[no_mangle]
fn main() -> isize {
    let start_time = get_time();
    let sleep_time = 10;

    println!("Start to sleep! I will wake up after {} seconds.", sleep_time);

    loop {
        let now = get_time();

        if now.sec - start_time.sec > sleep_time {
            break;
        }

        // now we have timer
        // yield_();
    }

    println!("Wake up!");
    0
}
