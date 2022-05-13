use core::ops::Deref;

use crate::apu::Apu;
use crate::error::{io_error_read, io_error_write};
use crate::interrupt::InterruptHandler;
use crate::joypad::Joypad;
use crate::ppu::Ppu;
use crate::ram::Ram;
use crate::region::*;
use crate::rom::Rom;
use crate::serial::Serial;
use crate::timer::Timer;

pub struct Bus<T: Deref<Target=[u8]>> {
    /// Access to io APU ports
    pub apu: Apu,
    /// Access to io joypad ports
    pub joypad: Joypad,
    /// Access to io PPU ports
    pub ppu: Ppu,
    /// Access to io serial ports
    pub serial: Serial,
    /// Access to io timer ports
    pub timer: Timer,
    /// Access to cartridge
    pub rom: Rom<T>,
    /// Shareable it handler
    pub it: InterruptHandler,
    /// Working ram
    wram: Ram<WRAM_REGION_SIZE>,
    /// High ram
    hram: Ram<HRAM_REGION_SIZE>,
}

impl<T: Deref<Target=[u8]>> Bus<T> {
    pub fn new(rom: Rom<T>) -> Self {
        Self {
            apu: Apu::new(),
            joypad: Joypad::new(),
            ppu: Ppu::new(),
            serial: Serial::new(),
            timer: Timer::new(),
            rom,
            hram: Ram::new(),
            wram: Ram::new(),
            it: InterruptHandler::new(),
        }
    }

    pub fn set_rom(&mut self, rom: Rom<T>) {
        self.rom = rom;
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            ROM_REGION_START..=ROM_REGION_END => self.rom.read(address),
            VRAM_REGION_START..=VRAM_REGION_END => self.ppu.read(address),
            ERAM_REGION_START..=ERAM_REGION_END => self.rom.read(address),
            WRAM_REGION_START..=WRAM_REGION_END => self.wram.read(address - WRAM_REGION_START),
            ECHORAM_REGION_START..=ECHORAM_REGION_END => {
                self.wram.read(address - ECHORAM_REGION_START)
            },
            OAM_REGION_START..=OAM_REGION_END => self.ppu.read(address),
            // I/O Registers
            IO_JOYPAD_REGION => self.joypad.read(address),
            IO_SERIAL_REGION_START..=IO_SERIAL_REGION_END => self.serial.read(address),
            IO_TIMER_REGION_START..=IO_TIMER_REGION_END => self.timer.read(address),
            IO_SOUND_REGION_START..=IO_SOUND_REGION_END => self.apu.read(address),
            IO_PPU_REGION_START..=IO_PPU_REGION_END => self.ppu.read(address),
            HRAM_REGION_START..=HRAM_REGION_END => self.hram.read(address - HRAM_REGION_START),
            REG_IF_ADDR | REG_IE_ADDR => self.it.read(address),
            _ => {
                io_error_read(address);
                0xFF
            },
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            ROM_REGION_START..=ROM_REGION_END => self.rom.write(address, value),
            VRAM_REGION_START..=VRAM_REGION_END => self.ppu.write(address, value),
            ERAM_REGION_START..=ERAM_REGION_END => self.rom.write(address, value),
            WRAM_REGION_START..=WRAM_REGION_END => {
                self.wram.write(address - WRAM_REGION_START, value)
            },
            ECHORAM_REGION_START..=ECHORAM_REGION_END => {
                self.wram.write(address - ECHORAM_REGION_START, value)
            },
            OAM_REGION_START..=OAM_REGION_END => self.ppu.write(address, value),
            // I/O Registers
            IO_JOYPAD_REGION => self.joypad.write(address, value),
            IO_SERIAL_REGION_START..=IO_SERIAL_REGION_END => self.serial.write(address, value),
            IO_TIMER_REGION_START..=IO_TIMER_REGION_END => self.timer.write(address, value),
            IO_SOUND_REGION_START..=IO_SOUND_REGION_END => self.apu.write(address, value),
            IO_PPU_REGION_START..=IO_PPU_REGION_END => self.ppu.write(address, value),
            HRAM_REGION_START..=HRAM_REGION_END => {
                self.hram.write(address - HRAM_REGION_START, value)
            },
            REG_IF_ADDR | REG_IE_ADDR => self.it.write(address, value),
            _ => io_error_write(address),
        }
    }

    pub fn dma_tick(&mut self) {
        if !self.ppu.is_dma_active() {
            return;
        }
        // The bus can read addresses from 0x0000 to 0xDF9F
        let byte = self.read(self.ppu.dma_source());
        self.ppu.dma_write(byte);
    }
}
