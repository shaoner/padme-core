use crate::region::*;

//
// DMG default registers values
//
const DEFAULT_REG_DMG_IF: u8    = 0xE1;
const DEFAULT_REG_DMG_IE: u8    = 0x00;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum InterruptFlag {
    Vblank        = 0b00000001,
    Lcdc          = 0b00000010,
    TimerOverflow = 0b00000100,
    Serial        = 0b00001000,
    Joypad        = 0b00010000,
}

pub struct InterruptHandler {
    /// Interrupt flag
    reg_if: u8,
    /// Interrupt enable
    reg_ie: u8,
}

impl InterruptHandler {
    pub fn new() -> Self {
        Self {
            reg_if: DEFAULT_REG_DMG_IF,
            reg_ie: DEFAULT_REG_DMG_IE,
        }
    }

    /// Reset all registers & state
    pub fn reset(&mut self) {
        self.reg_if = DEFAULT_REG_DMG_IF;
        self.reg_ie = DEFAULT_REG_DMG_IE;
    }

    pub fn request(&mut self, flag: InterruptFlag) {
        self.reg_if |= flag as u8;
    }

    pub fn clear(&mut self, flag: InterruptFlag) {
        self.reg_if &= !(flag as u8);
    }
}

impl MemoryRegion for InterruptHandler {
    fn read(&self, address: u16) -> u8 {
        match address {
            REG_IF_ADDR => self.reg_if,
            REG_IE_ADDR => self.reg_ie,
            _ => unreachable!(),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            REG_IF_ADDR => self.reg_if = value,
            REG_IE_ADDR => self.reg_ie = value,
            _ => unreachable!(),
        }
    }
}
