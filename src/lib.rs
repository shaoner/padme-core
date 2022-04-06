#![no_std]

// Private mods
#[macro_use]
mod bitops;
mod bus;
mod collections;
mod cpu;
mod error;
mod interrupt;
mod ram;
mod region;
mod rom;
mod serial;
mod timer;

// Public exports
pub use error::Error;
pub use rom::{CartridgeType, CgbMode, Licensee, Rom};
pub use serial::SerialOutput;
