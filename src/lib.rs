#![no_std]

// Private mods
#[macro_use]
mod bitops;

mod apu;
mod bus;
mod collections;
mod cpu;
mod error;
mod interrupt;
mod joypad;
mod ppu;
mod ram;
mod region;
mod rom;
mod serial;
mod system;
mod timer;

// Public exports
pub use apu::{AUDIO_SAMPLE_RATE, AudioSpeaker};
pub use cpu::CLOCK_SPEED;
pub use error::Error;
pub use joypad::Button;
pub use ppu::{FRAME_HEIGHT, FRAME_WIDTH, Pixel, Screen};
pub use rom::{CartridgeType, CgbMode, Licensee, Rom};
pub use serial::SerialOutput;
pub use system::System;
