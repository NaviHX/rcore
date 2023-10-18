#![no_std]
#![no_main]

use user_lib::yield_;

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> isize {
    for _ in 0..10 {
        println!("BBBBB");
        yield_();
    }

    0
}
