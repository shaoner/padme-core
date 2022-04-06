use core::ops::Deref;

use crate::{Error, Rom, Screen, SerialOutput};
use crate::bus::Bus;
use crate::cpu::Cpu;

pub struct System<T: Deref<Target=[u8]>, S: Screen, SO: SerialOutput> {
    bus: Bus<T>,
    cpu: Cpu,
    screen: S,
    serial_output: SO,
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
        let cycles = self.cpu.step(&mut self.bus);

        for _ in 0..cycles {
            self.bus.ppu.step(&mut self.screen, &mut self.bus.it);
            self.bus.timer.step(&mut self.bus.it);
        }

        self.bus.serial.step(&mut self.serial_output, &mut self.bus.it);

        self.bus.dma_tick();

        cycles
    }

    pub fn screen(&mut self) -> &mut S {
        &mut self.screen
    }

    pub fn serial(&mut self) -> &mut SO {
        &mut self.serial_output
    }
}
