use core::ops::Deref;
use core::time::Duration;

use crate::{Button, Error, Rom, Screen, SerialOutput};
use crate::bus::Bus;
use crate::cpu::{Cpu, CLOCK_SPEED};

pub const DEFAULT_FRAME_RATE: u32 = 60;

pub struct System<T: Deref<Target=[u8]>, S: Screen, SO: SerialOutput> {
    /// Address bus
    bus: Bus<T>,
    /// To execute instructions
    cpu: Cpu,
    /// A screen to give to the PPU
    screen: S,
    /// A serial output to give to the serial controller
    serial_output: SO,
    /// Keep the number of cycles before a frame is refreshed
    cycles_per_frame: u32,
}

impl<T: Deref<Target=[u8]>, S: Screen, SO: SerialOutput> System<T, S, SO> {
    pub fn new(rom: Rom<T>, screen: S, serial_output: SO) -> Self {
        let bus = Bus::new(rom);
        let cpu = Cpu::new();

        System {
            bus,
            cpu,
            screen,
            serial_output,
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
        Ok(self.bus.set_rom(rom))
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

    /// Forward a button press to the joypad controller
    /// ```
    /// # use padme_core::{Button, Pixel, Rom, Screen, SerialOutput, System};
    /// # use std::thread::sleep;
    /// # use std::time::Instant;
    /// #
    /// # struct Lcd;
    /// #
    /// # impl Screen for Lcd {
    /// #    fn set_pixel(&mut self, px: &Pixel, x: u8, y: u8) {}
    /// #
    /// #    fn update(&mut self) {}
    /// # }
    /// #
    /// # struct Console;
    /// #
    /// # impl SerialOutput for Console {
    /// #    fn putchar(&mut self, c: u8) {}
    /// # }
    /// #
    /// # let mut bin = [0u8; 32 * 1024];
    /// # let mut rom = Rom::load(&mut bin[..]).unwrap();
    /// let mut emu = System::new(rom, Lcd, Console);
    /// emu.set_button(Button::A, true);
    /// emu.set_button(Button::Up, true);
    /// ```
    pub fn set_button(&mut self, button: Button, is_pressed: bool) {
        self.bus.joypad.set_button(button, is_pressed);
    }

    /// Sets the FPS (default = 60)
    pub fn set_frame_rate(&mut self, fps: u32) {
        if fps > 0 && fps < CLOCK_SPEED {
            self.cycles_per_frame = CLOCK_SPEED / fps;
        }
    }

    /// Execute enough steps to retrieve 1 frame
    /// ```
    /// # use padme_core::{Button, Pixel, Rom, Screen, SerialOutput, System};
    /// # use std::thread::sleep;
    /// # use std::time::Instant;
    /// #
    /// # struct Lcd;
    /// #
    /// # impl Screen for Lcd {
    /// #    fn set_pixel(&mut self, px: &Pixel, x: u8, y: u8) {}
    /// #
    /// #    fn update(&mut self) {}
    /// # }
    /// #
    /// # struct Console;
    /// #
    /// # impl SerialOutput for Console {
    /// #    fn putchar(&mut self, c: u8) {}
    /// # }
    /// #
    /// # let mut bin = [0u8; 32 * 1024];
    /// # let mut rom = Rom::load(&mut bin[..]).unwrap();
    /// let mut emu = System::new(rom, Lcd, Console);
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
