use enum_dispatch::enum_dispatch;

use crate::error::{io_error_read, io_error_write};
use crate::region::*;

const DEFAULT_RAM_BANK: u8              = 0x00;
const DEFAULT_ROM_BANK: u8              = 0x01;

const RAM_ENABLE_START: u16             = 0x0000;
const RAM_ENABLE_END: u16               = 0x1FFF;
const ROM_BANK_SEL_START: u16           = 0x2000;
const ROM_BANK_SEL_END: u16             = 0x3FFF;
const RAM_BANK_SEL_START: u16           = 0x4000;
const RAM_BANK_SEL_END: u16             = 0x5FFF;
const BANK_MODE_START: u16              = 0x6000;
const BANK_MODE_END: u16                = 0x7FFF;

const ERAM_SIZE: usize                  = 32 * 1024;
const ROM_REGION_BANK0_START: u16       = ROM_REGION_START;
const ROM_REGION_BANK0_END: u16         = 0x3FFF;
const ROM_REGION_BANKN_START: u16       = 0x4000;
const ROM_REGION_BANKN_END: u16         = ROM_REGION_END;

const ROM_BANK_SIZE: usize              = (ROM_REGION_BANKN_END - ROM_REGION_BANKN_START + 1) as usize;
const RAM_BANK_SIZE: usize              = ERAM_REGION_SIZE;

#[enum_dispatch]
pub trait MbcController {
    fn read(&self, storage: &[u8], address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

#[enum_dispatch(MbcController)]
pub enum Mbc {
    Mbc0,
    Mbc1,
    Mbc3,
}

pub struct Mbc0;

impl MbcController for Mbc0 {
    fn read(&self, storage: &[u8], address: u16) -> u8 {
        match address {
            ROM_REGION_START..=ROM_REGION_END => {
                // We know storage.len() >= ROM_REGION_END (32K)
                storage[(address - ROM_REGION_START) as usize]
            },
            _ => {
                io_error_read(address);
                0xFF
            }
        }
    }

    fn write(&mut self, address: u16, _value: u8) {
        io_error_write(address);
    }
}

pub struct Mbc1 {
    /// External ram
    eram: [u8; ERAM_SIZE],
    /// Is ram enabled (mbc1)
    ram_enabled: bool,
    /// Select the rom bank
    rom_bank: u8,
    /// Select the ram bank
    ram_bank: u8,
    /// Whether bank mode is rom or ram
    ram_bank_mode: bool,
}

impl Mbc1 {
    pub fn new() -> Self {
        Self {
            eram: [0u8; ERAM_SIZE],
            ram_enabled: false,
            ram_bank: DEFAULT_RAM_BANK,
            rom_bank: DEFAULT_ROM_BANK,
            ram_bank_mode: false,
        }
    }

    fn set_rom_bank(&mut self, bank: u8) {
        self.rom_bank = match bank {
            0x00 | 0x20 | 0x40 | 0x60 => bank + 1,
            _ => bank
        }
    }
}

impl MbcController for Mbc1 {
    fn read(&self, storage: &[u8], address: u16) -> u8 {
        match address {
            ROM_REGION_BANK0_START..=ROM_REGION_BANK0_END => storage[address as usize],
            ROM_REGION_BANKN_START..=ROM_REGION_BANKN_END => {
                let offset = address - ROM_REGION_BANKN_START;
                let idx = offset as usize + (ROM_BANK_SIZE * self.rom_bank as usize);
                storage[idx]
            },
            ERAM_REGION_START..=ERAM_REGION_END => {
                if self.ram_enabled {
                    let offset = address - ERAM_REGION_START;
                    let idx = offset as usize + (RAM_BANK_SIZE * self.ram_bank as usize);
                    self.eram[idx]
                } else {
                    0xFF
                }
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            RAM_ENABLE_START..=RAM_ENABLE_END => self.ram_enabled = (value & 0xA) == 0xA,
            ROM_BANK_SEL_START..=ROM_BANK_SEL_END => {
                let bank = value & 0x1F;
                self.set_rom_bank((self.rom_bank & 0xE0) | bank);
            },
            RAM_BANK_SEL_START..=RAM_BANK_SEL_END => {
                let bank = value & 0x03;
                if self.ram_bank_mode {
                    self.ram_bank = bank;
                } else {
                    self.set_rom_bank(bank << 5 | self.rom_bank);
                }
            },
            BANK_MODE_START..=BANK_MODE_END => self.ram_bank_mode = is_set!(value, 0x01),
            ERAM_REGION_START..=ERAM_REGION_END => {
                if self.ram_enabled {
                    let offset = address - ERAM_REGION_START;
                    let idx = offset as usize + (RAM_BANK_SIZE * self.ram_bank as usize);
                    self.eram[idx] = value;
                }
            },
            _ => io_error_write(address),
        }
    }
}

pub struct Mbc3 {
    ram_timer_enabled: bool,
    rom_bank: u8,
    ram_bank: u8,
    reg_rtc: u8,
    rtc_mode: bool,
    eram: [u8; ERAM_SIZE],
}

impl Mbc3 {
    pub fn new() -> Self {
        Self {
            ram_timer_enabled: false,
            rom_bank: DEFAULT_ROM_BANK,
            ram_bank: DEFAULT_RAM_BANK,
            reg_rtc: 0,
            rtc_mode: false,
            eram: [0u8; ERAM_SIZE],
        }
    }
}

impl MbcController for Mbc3 {
    fn read(&self, storage: &[u8], address: u16) -> u8 {
        match address {
            ROM_REGION_BANK0_START..=ROM_REGION_BANK0_END => storage[address as usize],
            ROM_REGION_BANKN_START..=ROM_REGION_BANKN_END => {
                let offset = address - ROM_REGION_BANKN_START;
                let idx = offset as usize + (ROM_BANK_SIZE * self.rom_bank as usize);
                storage[idx]
            },
            ERAM_REGION_START..=ERAM_REGION_END => {
                if self.ram_timer_enabled {
                    if self.rtc_mode {
                        self.reg_rtc
                    } else {
                        let offset = address - ERAM_REGION_START;
                        let idx = offset as usize + (RAM_BANK_SIZE * self.ram_bank as usize);
                        self.eram[idx]
                    }
                } else {
                    0xFF
                }
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            RAM_ENABLE_START..=RAM_ENABLE_END => self.ram_timer_enabled = (value & 0xA) == 0xA,
            ROM_BANK_SEL_START..=ROM_BANK_SEL_END => self.rom_bank = value,
            RAM_BANK_SEL_START..=RAM_BANK_SEL_END => {
                if value <= 0x03 {
                    // Ram selection
                    self.rtc_mode = false;
                    self.ram_bank = value;
                } else if (0x08..=0x0C).contains(&value) {
                    self.rtc_mode = true;
                }
            },
            ERAM_REGION_START..=ERAM_REGION_END => {
                if self.ram_timer_enabled {
                    if self.rtc_mode {
                        self.reg_rtc = value;
                    } else {
                        let offset = address - ERAM_REGION_START;
                        let idx = offset as usize + (RAM_BANK_SIZE * self.ram_bank as usize);
                        self.eram[idx] = value;
                    }
                }
            },
            _ => io_error_write(address),
        }
    }
}
