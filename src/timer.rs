use log::trace;

use crate::cpu::CLOCK_SPEED;
use crate::interrupt::{InterruptHandler, InterruptFlag};
use crate::region::*;

const DIV_PERIOD: u32 = CLOCK_SPEED / 16384;

// Default DMG register values
const DEFAULT_REG_DIV: u8       = 0x18;
const DEFAULT_REG_TIMA: u8      = 0x00;
const DEFAULT_REG_TMA: u8       = 0x00;
const DEFAULT_REG_TAC: u8       = 0xF8;

// TAC flags
const FLAG_TIMER_ENABLED: u8    = 0b00000100;
const FLAG_INPUT_CLOCK_SEL: u8  = 0b00000011;

// Input clock values
const INPUT_CLOCK_SEL_1024: u8  = 0x00;
const INPUT_CLOCK_SEL_16: u8    = 0x01;
const INPUT_CLOCK_SEL_64: u8    = 0x02;
const INPUT_CLOCK_SEL_256: u8   = 0x03;

pub struct Timer {
    /// Divider
    reg_div: u8,
    /// Timer counter
    reg_tima: u8,
    /// Timer modulo
    reg_tma: u8,
    /// Timer control
    reg_tac: u8,
    /// divider counter cycles (max = 255 + max(CPU_ticks))
    div_cycles: u16,
    /// tma counter of cycles
    tima_cycles: u16,
    /// keep track of the clock period
    tima_period: u16,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            reg_div: DEFAULT_REG_DIV,
            reg_tima: DEFAULT_REG_TIMA,
            reg_tma: DEFAULT_REG_TMA,
            reg_tac: DEFAULT_REG_TAC,
            div_cycles: 0,
            tima_cycles: 0,
            tima_period: Timer::period_from_tac(DEFAULT_REG_TAC),
        }
    }

    /// Reset all registers and state
    pub fn reset(&mut self) {
        self.reg_div = DEFAULT_REG_DIV;
        self.reg_tima = DEFAULT_REG_TIMA;
        self.reg_tma = DEFAULT_REG_TMA;
        self.reg_tac = DEFAULT_REG_TAC;
        self.div_cycles = 0;
        self.tima_cycles = 0;
        self.tima_period = Timer::period_from_tac(DEFAULT_REG_TAC);
    }

    /// Determine how many ticks to wait
    fn period_from_tac(tac: u8) -> u16 {
        match tac & FLAG_INPUT_CLOCK_SEL {
            INPUT_CLOCK_SEL_1024 => 1024,
            INPUT_CLOCK_SEL_16 => 16,
            INPUT_CLOCK_SEL_64 => 64,
            INPUT_CLOCK_SEL_256 => 256,
            _ => unreachable!(),
        }
    }

    /// Single timer step for each cpu T-cycle
    pub fn step(&mut self, ir: &mut InterruptHandler) {
        self.div_cycles += 1;

        if self.div_cycles > DIV_PERIOD as u16 {
            self.reg_div = self.reg_div.wrapping_add(1);
            self.div_cycles = 0;
        }

        let new_tima_period = Timer::period_from_tac(self.reg_tac);

        if new_tima_period != self.tima_period {
            // period changed
            self.tima_period = new_tima_period;
            self.tima_cycles = 0;
        } else if (self.reg_tac & FLAG_TIMER_ENABLED) == FLAG_TIMER_ENABLED {
            self.tima_cycles += 1;

            if self.tima_cycles >= self.tima_period {
                // Reached cycles limit, increment tima
                self.reg_tima = self.reg_tima.wrapping_add(1);
                self.tima_cycles = 0;
                if self.reg_tima == 0xFF {
                    trace!("timer overflow, reset to 0x{:02X}", self.reg_tma);
                    self.reg_tima = self.reg_tma;
                    ir.request(InterruptFlag::TimerOverflow);
                }
            }
        }
    }
}

impl MemoryRegion for Timer {
    fn read(&self, address: u16) -> u8 {
        match address {
            REG_DIV_ADDR => self.reg_div,
            REG_TIMA_ADDR => self.reg_tima,
            REG_TMA_ADDR => self.reg_tma,
            REG_TAC_ADDR => self.reg_tac,
            _ => unreachable!(),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            REG_DIV_ADDR => self.reg_div = 0,
            REG_TIMA_ADDR => self.reg_tima = value,
            REG_TMA_ADDR => self.reg_tma = value,
            REG_TAC_ADDR => self.reg_tac = value,
            _ => unreachable!(),
        }
    }
}
