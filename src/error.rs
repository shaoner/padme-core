use log::warn;

#[cfg_attr(debug_assertions, derive(Debug))]
pub enum Error {
    InvalidRomSize(usize),
}

macro_rules! io_error {
    ($addr: expr, $read: expr) => {
        warn!("Cannot {} @ 0x{:04X}", if $read { "read" } else { "write" }, $addr)
    }
}

pub fn io_error_read(address: u16) {
    io_error!(address, true)
}

pub fn io_error_write(address: u16) {
    io_error!(address, false)
}
