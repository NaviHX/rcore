use core::panic::PanicInfo;

use crate::{sbi::shutdown, error};

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    if let Some(loc) = info.location() {
        error!(
            "Kernel Panicked at {}:{} {}",
            loc.file(),
            loc.line(),
            info.message().unwrap()
        );
    } else {
        error!("Kernel Panicked: {}", info.message().unwrap());
    }
    shutdown();
}
