#![no_std]

// Private mods
#[macro_use]
mod bitops;
mod error;
mod region;
mod rom;

use error::Error;
use rom::Rom;
