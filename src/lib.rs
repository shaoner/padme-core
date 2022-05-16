#![no_std]
//! # Padme
//!
//! `padme_core` is a gameboy emulator engine that can be used to create a gameboy emulator on any platform.
//! It is possible to use padme to build an emulator on any platform, so it supports no_std environments.
//! It's especially a great fit for web assembly or embedded devices where you cannot always rely on dynamic memory allocation or threads.
//! Although, for simplicity, most examples and tests use the std crate.
//!
//! For now, it's not planned to support gameboy color or gameboy advance, but maybe later.
//!
//! ## How to build an emulator
//!
//! ```
//! use padme_core::{AudioSpeaker, Button, Pixel, Rom, Screen, SerialOutput, System};
//!
//! struct MyScreen {
//!    // ... your framebuffer implementation
//! }
//!
//! impl Screen for MyScreen {
//!     fn set_pixel(&mut self, pixel: &Pixel, x: u8, y: u8) {
//!         // add pixel to your framebuffer
//!     }
//!     fn update(&mut self) {
//!         // a frame has been pushed
//!         // this is called during a VBlank
//!     }
//! }
//!
//! struct MySpeaker {
//!     // ... your audio buffer implementation
//! }
//!
//! impl AudioSpeaker for MySpeaker {
//!     fn set_samples(&mut self, left: f32, right: f32) {
//!         // add samples for left and right channels
//!     }
//! }
//!
//! struct MySerialConsole {
//! }
//!
//! impl SerialOutput for MySerialConsole {
//!     fn putchar(&mut self, ch: u8) {
//!         // a byte has been transmitted
//!     }
//! }
//!
//! # let mut bin = [0u8; 32 * 1024];
//! // Loads a game, most commonly, it is retrieved from a file
//! // let bin: Vec<u8> = std::fs::read("some_game.gb").expect("could not find game");
//! let mut rom = Rom::load(&mut bin[..]).unwrap();
//! let mut emulator = System::new(rom, MyScreen {}, MySerialConsole {}, MySpeaker {});
//! // Set the number of frame per seconds
//! // This also sets the number of cycles needed per frame given the fixed CPU clock frequency
//! emulator.set_frame_rate(60);
//! # let mut running = true;
//! # let a_pressed = false;
//! # let b_pressed = false;
//! # let start_pressed = false;
//! # let select_pressed = false;
//! # let up_pressed = false;
//! # let down_pressed = false;
//! # let left_pressed = false;
//! # let right_pressed = false;
//!
//! while running {
//!     let t0 = std::time::Instant::now();
//!     emulator.update_frame();
//!     emulator.set_button(Button::A, a_pressed);
//!     emulator.set_button(Button::B, b_pressed);
//!     emulator.set_button(Button::Start, start_pressed);
//!     emulator.set_button(Button::Select, select_pressed);
//!     emulator.set_button(Button::Up, up_pressed);
//!     emulator.set_button(Button::Down, down_pressed);
//!     emulator.set_button(Button::Left, left_pressed);
//!     emulator.set_button(Button::Right, right_pressed);
//!     // here we need to sleep for the remaining time
//!     let frame_time = t0.elapsed();
//!     let min_frame_time = emulator.min_frame_time();
//!     if frame_time < min_frame_time {
//!         std::thread::sleep(min_frame_time - frame_time);
//!     }
//! #   running = false;
//! }
//! ```

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

pub mod default;
