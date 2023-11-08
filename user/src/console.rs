use crate::{write, syscall::sys_read, read};
use core::fmt::{self, Write};

pub struct Stdout;
pub const STDOUT: usize = 1;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write(STDOUT, s.as_bytes());
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap()
}

pub struct Stdin;
pub const STDIN: usize = 0;

impl Stdin {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        let ret = read(STDIN, buf);
        if ret == -1 {
            Err(())
        } else {
            Ok(ret as usize)
        }
    }
}

pub fn getchar() -> u8 {
    let mut c = [0u8; 1];
    let _ = Stdin.read(&mut c);
    c[0]
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
        $crate
    }
}
