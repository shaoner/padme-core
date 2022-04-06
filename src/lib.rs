#![no_std]

// Private mods
#[macro_use]
mod bitops;
mod error;
mod region;
mod rom;

pub use error::Error;
pub use rom::{CartridgeType, CgbMode, Licensee, Rom};
