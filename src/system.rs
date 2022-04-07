use core::ops::Deref;

use crate::{Button, Error, Rom, Screen, SerialOutput};
use crate::bus::Bus;
use crate::cpu::{Cpu, CLOCK_SPEED};

pub const DEFAULT_FRAME_RATE: u32 = 60;

pub struct System<T: Deref<Target=[u8]>, S: Screen, SO: SerialOutput> {
    bus: Bus<T>,
    cpu: Cpu,
    screen: S,
    serial_output: SO,
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

    pub fn load_bin(&mut self, bytes: T) -> Result<(), Error> {
        let rom = Rom::load(bytes)?;

        Ok(self.bus.set_rom(rom))
    }

    pub fn load_rom(&mut self, rom: Rom<T>) {
        self.bus.set_rom(rom);
    }

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

    pub fn screen(&mut self) -> &mut S {
        &mut self.screen
    }

    pub fn serial(&mut self) -> &mut SO {
        &mut self.serial_output
    }

    pub fn set_button(&mut self, button: Button, is_pressed: bool) {
        self.bus.joypad.set_button(button, is_pressed);
    }

    pub fn set_frame_rate(&mut self, fps: u32) {
        if fps > 0 && fps < CLOCK_SPEED {
            self.cycles_per_frame = CLOCK_SPEED / fps;
        }
    }

    pub fn update_frame(&mut self) -> u32 {
        let mut cycles = 0u32;
        while cycles < self.cycles_per_frame {
            cycles += self.step() as u32;
        }
        self.screen.update();
        cycles
    }
}
