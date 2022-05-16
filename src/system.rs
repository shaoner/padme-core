use core::ops::Deref;
use core::time::Duration;

use crate::{Button, Error, Rom, Screen, AudioSpeaker, SerialOutput};
use crate::bus::Bus;
use crate::cpu::{Cpu, CLOCK_SPEED};

pub const DEFAULT_FRAME_RATE: u32 = 60;

pub struct System<T: Deref<Target=[u8]>,
                  S: Screen,
                  SO: SerialOutput,
                  AS: AudioSpeaker> {
    /// Address bus
    bus: Bus<T>,
    /// To execute instructions
    cpu: Cpu,
    /// A screen to give to the PPU
    screen: S,
    /// A serial output to give to the serial controller
    serial_output: SO,
    /// An audio speaker interface
    speaker: AS,
    /// Keep the number of cycles before a frame is refreshed
    cycles_per_frame: u32,
}

impl<T: Deref<Target=[u8]>,
     S: Screen,
     SO: SerialOutput,
     AS: AudioSpeaker> System<T, S, SO, AS> {
    pub fn new(rom: Rom<T>, screen: S, serial_output: SO, speaker: AS) -> Self {
        let bus = Bus::new(rom);
        let cpu = Cpu::new();

        System {
            bus,
            cpu,
            screen,
            serial_output,
            speaker,
            cycles_per_frame: CLOCK_SPEED / DEFAULT_FRAME_RATE,
        }
    }

    pub fn reset(&mut self) {
        self.bus.ppu.reset();
        self.bus.timer.reset();
        self.bus.serial.reset();
        self.bus.joypad.reset();
        self.bus.it.reset();
        self.cpu.reset();
    }

    /// Replace cartridge with a new buffer
    pub fn load_bin(&mut self, bytes: T) -> Result<(), Error> {
        let rom = Rom::load(bytes)?;

        self.reset();
        self.bus.set_rom(rom);
        Ok(())
    }

    /// Reload a new rom
    pub fn load_rom(&mut self, rom: Rom<T>) {
        self.bus.set_rom(rom);
        self.reset();
    }

    /// Single step to execute cpu, ppu, timer, serial & dma
    pub fn step(&mut self) -> u8 {
        let ticks = self.cpu.step(&mut self.bus);

        for _ in 0..ticks {
            self.bus.apu.step(&mut self.speaker);
            self.bus.ppu.step(&mut self.screen, &mut self.bus.it);
            self.bus.timer.step(&mut self.bus.it);
        }

        self.bus.serial.step(&mut self.serial_output, &mut self.bus.it);

        self.bus.dma_tick();

        ticks
    }

    /// Retrieve the rom in readonly
    pub fn rom(&self) -> &Rom<T> {
        &self.bus.rom
    }

    /// Retrieve the screen
    pub fn screen(&mut self) -> &mut S {
        &mut self.screen
    }

    /// Retrieve the serial output
    pub fn serial(&mut self) -> &mut SO {
        &mut self.serial_output
    }

    /// Retrieve the speaker
    pub fn speaker(&mut self) -> &mut AS {
        &mut self.speaker
    }

    /// Forward a button press to the joypad controller
    /// ```
    /// # use padme_core::*;
    /// # use padme_core::default::*;
    /// #
    /// # let mut bin = [0u8; 32 * 1024];
    /// # let mut rom = Rom::load(&mut bin[..]).unwrap();
    /// let mut emu = System::new(rom, NoScreen, NoSerial, NoSpeaker);
    /// emu.set_button(Button::A, true);
    /// emu.set_button(Button::Up, true);
    /// ```
    pub fn set_button(&mut self, button: Button, is_pressed: bool) {
        self.bus.joypad.set_button(button, is_pressed, &mut self.bus.it);
    }

    /// Sets the FPS (default = 60)
    pub fn set_frame_rate(&mut self, fps: u32) {
        if fps > 0 && fps < CLOCK_SPEED {
            self.cycles_per_frame = CLOCK_SPEED / fps;
        }
    }

    /// Execute enough steps to retrieve 1 frame
    /// ```
    /// # use padme_core::*;
    /// # use padme_core::default::*;
    /// # use std::time::Instant;
    /// # use std::thread::sleep;
    /// #
    /// # let mut bin = [0u8; 32 * 1024];
    /// # let mut rom = Rom::load(&mut bin[..]).unwrap();
    /// let mut emu = System::new(rom, NoScreen, NoSerial, NoSpeaker);
    /// // loop {
    ///     let t0 = Instant::now();
    ///     emu.update_frame();
    ///     let frame_time = t0.elapsed();
    ///     let min_frame_time = emu.min_frame_time();
    ///     if frame_time < min_frame_time {
    ///         sleep(min_frame_time - frame_time);
    ///     }
    /// // }
    /// ```
    pub fn update_frame(&mut self) -> u32 {
        let mut cycles = 0u32;
        while cycles < self.cycles_per_frame {
            cycles += self.step() as u32;
        }
        self.screen.update();
        cycles
    }

    /// Returns the minimum amount of time to wait between each frame
    /// Mostly depend on the FPS
    pub fn min_frame_time(&self) -> Duration {
        Duration::from_millis(1000 / (CLOCK_SPEED / self.cycles_per_frame) as u64)
    }
}
