use crate::region::*;

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
    reg_p1: u8,
    button_state: u8,
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

    pub fn set_button(&mut self, button: Button, is_pressed: bool) {
        let button = button as u8;
        if is_set!(button, FLAG_ACTION_BUTTON) {
            if is_pressed {
                self.button_state |= button;
            } else {
                self.button_state &= !button;
            }
        } else if is_set!(button, FLAG_DIR_BUTTON) {
            if is_pressed {
                self.dir_state |= button;
            } else {
                self.dir_state &= !button;
            }
        }
    }
}

impl MemoryRegion for Joypad {
    fn read(&self, _address: u16) -> u8 {
        let select = self.reg_p1 & 0x30;
        match select {
            0x10 => select | !self.dir_state,
            0x20 => select | !self.button_state,
            _ => self.reg_p1,
        }
    }

    fn write(&mut self, _address: u16, value: u8) {
        self.reg_p1 = !value;
    }
}
