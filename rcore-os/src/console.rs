use crate::sbi::console_put_char;
use core::fmt::{self, Write};

pub struct KStdout;

impl Write for KStdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            console_put_char(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    KStdout.write_fmt(args).unwrap()
}

#[macro_export]
macro_rules! print {
    ($fmt:literal $(,$($arg:tt)+)?) => {
        $crate::console::print(format_args!($fmt $(,$($arg)+)?));
    };
}

#[macro_export]
macro_rules! println {
    ($fmt:literal $(,$($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(,$($arg)+)?));
    };
}

#[macro_export]
macro_rules! error {
    ($fmt:literal $(,$($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[31m[kernel - error] ", $fmt, "\x1b[0m\n") $(,$($arg)+)?))
    }
}

#[macro_export]
macro_rules! log {
    ($fmt:literal $(,$($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[34m[kernel] ", $fmt, "\x1b[0m\n") $(,$($arg)+)?))
    }
}

#[macro_export]
macro_rules! warn {
    ($fmt:literal $(,$($arg:tt)+)?) => {
        $crate::console::print(format_args!(concat!("\x1b[93m[kernel - warn] ", $fmt, "\x1b[0m\n") $(,$($arg)+)?))
    }
}
