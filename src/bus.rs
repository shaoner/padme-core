use core::ops::Deref;

use log::error;

use crate::interrupt::InterruptHandler;
use crate::ram::{HighRam, WorkRam};
use crate::region::*;
use crate::rom::Rom;
use crate::serial::Serial;
use crate::timer::Timer;

pub struct Bus<T: Deref<Target=[u8]>> {
    pub serial: Serial,
    pub it: InterruptHandler,
    pub timer: Timer,

    rom: Rom<T>,
    wram: WorkRam,
    hram: HighRam,
}

impl<T: Deref<Target=[u8]>> Bus<T> {
    pub fn new(rom: Rom<T>) -> Self {
        Self {
            serial: Serial::new(),
            timer: Timer::new(),
            rom,
            hram: HighRam::new(),
            wram: WorkRam::new(),
            it: InterruptHandler::new(),
        }
    }

    pub fn set_rom(&mut self, rom: Rom<T>) {
        self.rom = rom;
    }

    pub fn read(&self, address: u16) -> u8 {
        let value = match address {
            ROM_REGION_START..=ROM_REGION_END => self.rom.read(address),
            ERAM_REGION_START..=ERAM_REGION_END => self.rom.read(address),
            WRAM_REGION_START..=WRAM_REGION_END => self.wram.read(address),
            IO_SERIAL_REGION_START..=IO_SERIAL_REGION_END => self.serial.read(address),
            IO_TIMER_REGION_START..=IO_TIMER_REGION_END => self.timer.read(address),
            HRAM_REGION_START..=HRAM_REGION_END => self.hram.read(address),
            REG_IF_ADDR | REG_IE_ADDR => self.it.read(address),
            _ => {
                error!("Cannot read memory region {:04X}", address);
                0xFF
            },
        };

        value
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            ROM_REGION_START..=ROM_REGION_END => self.rom.write(address, value),
            ERAM_REGION_START..=ERAM_REGION_END => self.rom.write(address, value),
            WRAM_REGION_START..=WRAM_REGION_END => self.wram.write(address, value),
            // IO registers
            IO_SERIAL_REGION_START..=IO_SERIAL_REGION_END => self.serial.write(address, value),
            IO_TIMER_REGION_START..=IO_TIMER_REGION_END => self.timer.write(address, value),
            HRAM_REGION_START..=HRAM_REGION_END => self.hram.write(address, value),
            REG_IF_ADDR | REG_IE_ADDR => self.it.write(address, value),
            _ => {
                error!("Cannot write region {:04X}", address);
            }
        }
    }
}
