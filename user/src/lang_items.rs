use core::panic::PanicInfo;

use crate::{exit, println};

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    if let Some(loc) = info.location() {
        println!(
            "Panicked at {}:{} {}",
            loc.file(),
            loc.line(),
            info.message().unwrap()
        );
    } else {
        println!("Panicked: {}", info.message().unwrap());
    }
    exit(1);
    panic!("The application should have exited after panicked!")
}
