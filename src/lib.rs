#![no_std]

// Private mods
#[macro_use]
mod bitops;
mod error;
mod interrupt;
mod region;
mod rom;

// Public exports
pub use error::Error;
pub use rom::{CartridgeType, CgbMode, Licensee, Rom};
