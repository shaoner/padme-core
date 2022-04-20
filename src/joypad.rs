use crate::region::*;
use crate::interrupt::{InterruptFlag, InterruptHandler};

// Default register values
const DEFAULT_REG_DMG_P1: u8    = 0xCF;

const FLAG_ACTION_BUTTON: u8    = 0x20;
const FLAG_DIR_BUTTON: u8       = 0x10;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Button {
    Start       = 0b00101000,
    Select      = 0b00100100,
    B           = 0b00100010,
    A           = 0b00100001,
    Down        = 0b00011000,
    Up          = 0b00010100,
    Left        = 0b00010010,
    Right       = 0b00010001,
}

pub struct Joypad {
    /// Joypad register @ 0xFF00, only for bit 4 and 5
    reg_p1: u8,
    /// Keep register state in button mode
    button_state: u8,
    /// Keep register state in direction mode
    dir_state: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            reg_p1: DEFAULT_REG_DMG_P1,
            button_state: 0,
            dir_state: 0,
        }
    }

    /// Reset all registers and state
    pub fn reset(&mut self) {
        self.reg_p1 = DEFAULT_REG_DMG_P1;
        self.button_state = 0;
        self.dir_state = 0;
    }

    pub fn set_button(&mut self, button: Button, is_pressed: bool, it: &mut InterruptHandler) {
        let button = button as u8;
        if is_set!(button, FLAG_ACTION_BUTTON) {
            if is_pressed {
                self.button_state |= button;
                it.request(InterruptFlag::Joypad);
            } else {
                self.button_state &= !button;
            }
        } else if is_set!(button, FLAG_DIR_BUTTON) {
            if is_pressed {
                self.dir_state |= button;
                it.request(InterruptFlag::Joypad);
            } else {
                self.dir_state &= !button;
            }
        }
        // Not clear what to do if both are enabled or disabled so do nothing
    }
}

impl MemoryRegion for Joypad {
    fn read(&self, _address: u16) -> u8 {
        // retrieve state depending on the current mode
        let select = self.reg_p1 & 0x30;
        match select {
            0x10 => select | !self.dir_state,
            0x20 => select | !self.button_state,
            _ => self.reg_p1,
        }
    }

    fn write(&mut self, _address: u16, value: u8) {
        // 0 means enabled, so we only care about storing ~bit4 and ~bit5
        // so during read, we can just apply a mask to bit4 and bit5
        self.reg_p1 = !value;
    }
}
