use log::trace;

use crate::interrupt::{InterruptHandler, InterruptFlag};
use crate::region::*;

// Default registers
const DEFAULT_REG_SB: u8        = 0x00;
const DEFAULT_REG_SC: u8        = 0x7E;

const FLAG_SC_TRANSFER: u8      = 0x80;
const FLAG_SC_INT_CLOCK: u8     = 0x01;

pub trait SerialOutput {
    fn putchar(&mut self, c: u8);
}

pub struct Serial {
    /// Serial transfer data (R/W)
    reg_sb: u8,
    /// Serial transfer control (R/W)
    reg_sc: u8,
}

impl Serial {
    pub fn new() -> Self {
        Self {
            reg_sb: DEFAULT_REG_SB,
            reg_sc: DEFAULT_REG_SC,
        }
    }

    /// Reset all registers and states
    pub fn reset(&mut self) {
        self.reg_sb = DEFAULT_REG_SB;
        self.reg_sc = DEFAULT_REG_SC;
    }

    pub fn step<SO>(&mut self, out: &mut SO, it: &mut InterruptHandler)
        where SO: SerialOutput
    {
        const NEW_CHAR_FLAG: u8 = FLAG_SC_TRANSFER | FLAG_SC_INT_CLOCK;

        if (self.reg_sc & NEW_CHAR_FLAG) == NEW_CHAR_FLAG {
            self.reg_sc &= !FLAG_SC_TRANSFER;
            trace!("write character: 0x{:02X} ({})", self.reg_sb, self.reg_sb as char);
            out.putchar(self.reg_sb);
            it.request(InterruptFlag::Serial);
        }
    }
}

impl MemoryRegion for Serial {
    fn read(&self, address: u16) -> u8 {
        match address {
            REG_SB_ADDR => self.reg_sb,
            REG_SC_ADDR => self.reg_sc,
            _ => unreachable!(),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            REG_SB_ADDR => self.reg_sb = value,
            REG_SC_ADDR => self.reg_sc = value,
            _ => unreachable!(),
        }
    }
}
