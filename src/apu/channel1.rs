use crate::region::*;

use super::modulation::*;

//
// Default register values
//
const DEFAULT_REG_DMG_NR10: u8          = 0x80;
const DEFAULT_REG_DMG_NR11: u8          = 0xBF;
const DEFAULT_REG_DMG_NR12: u8          = 0xF3;
const DEFAULT_REG_DMG_NR13: u8          = 0xFF;
const DEFAULT_REG_DMG_NR14: u8          = 0xBF;

pub struct Channel1 {
    /// Whether this channel is enabled or not
    enabled: bool,
    /// Bit 6-4: Sweep Time
    /// Bit 3  : Sweep Increase/Decrease
    /// 0: Addition    (frequency increases)
    /// 1: Subtraction (frequency decreases)
    /// Bit 2-0: Number of sweep shift (n: 0-7)
    reg_nr10: u8,
    /// Bit 7-6: Wave Pattern Duty
    /// Bit 5-0: Sound length
    reg_nr11: u8,
    /// Bit 7-4: Initial Volume of envelope (0-0Fh) (0=No Sound)
    /// Bit 3  : Envelope Direction (0=Decrease, 1=Increase)
    /// Bit 2-0: Number of envelope sweep (n: 0-7)
    reg_nr12: u8,
    /// Frequency lower 8 bits
    reg_nr13: u8,
    /// Bit 7  : Initial (1=Restart Sound)
    /// Bit 6  : Counter/consecutive selection (Read/Write)
    /// Bit 2-0: Frequency's higher 3 bits (x) (Write Only)
    reg_nr14: u8,
    /// Volume between 0x0 and 0xF
    current_volume: u8,
    /// Envelope Period timer
    envelope_timer: u8,
    /// Wave cursor position
    wave_cursor: u8,
    /// Frequency timer between 4 and 8192
    frequency_timer: u16,
    /// Length counter between 0 and 64
    length_counter: u8,
    /// Length period is half
    length_half_period: bool,
    /// Sweep timer
    sweep_timer: u8,
    /// Sweep shadow frequency
    shadow_frequency: u16,
    /// Whether sweep is enabled
    sweep_enabled: bool,
    /// Fix issues with neg -> pos sweep
    sweep_was_decreasing: bool,
}

impl Channel1 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            reg_nr10: DEFAULT_REG_DMG_NR10,
            reg_nr11: DEFAULT_REG_DMG_NR11,
            reg_nr12: DEFAULT_REG_DMG_NR12,
            reg_nr13: DEFAULT_REG_DMG_NR13,
            reg_nr14: DEFAULT_REG_DMG_NR14,
            current_volume: DEFAULT_REG_DMG_NR12 >> 4,
            envelope_timer: DEFAULT_REG_DMG_NR12 & 0b111,
            wave_cursor: 0,
            frequency_timer: 4,
            length_counter: 64,
            length_half_period: false,
            sweep_timer: 0,
            shadow_frequency: 0,
            sweep_enabled: false,
            sweep_was_decreasing: false,
        }
    }
}

impl Channel for Channel1 {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_dac_enabled(&self) -> bool {
        (self.reg_nr12 & 0b1111_1000) != 0
    }

    fn trigger(&mut self) {
        if self.is_dac_enabled() {
            self.enabled = true;
        }
        if self.length_counter == 0 {
            self.set_length_counter(64);
            if self.is_length_enabled() && self.length_half_period {
                self.length_counter -= 1;
            }
        }
        self.reset_frequency_timer();
        self.reset_envelope();
        self.reset_sweep();
    }
}

impl Clock for Channel1 {
    fn frequency(&self) -> u32 {
        (((self.reg_nr14 & 0b0000_0111) as u32) << 8) | (self.reg_nr13 as u32)
    }

    fn set_frequency(&mut self, value: u32) {
        self.reg_nr14 = (self.reg_nr14 & 0b1111_1000) | (((value >> 8) & 0b0111) as u8);
        self.reg_nr13 = value as u8;
    }

    fn frequency_timer(&self) -> u32 {
        self.frequency_timer as u32
    }

    fn set_frequency_timer(&mut self, value: u32) {
        self.frequency_timer = value as u16;
    }
}

impl EnvelopeModulation for Channel1 {
    fn envelope_register(&self) -> u8 {
        self.reg_nr12
    }

    fn envelope_timer(&mut self) -> &mut u8 {
        &mut self.envelope_timer
    }

    fn envelope_volume(&self) -> u8 {
        self.current_volume
    }

    fn set_envelope_volume(&mut self, value: u8) {
        self.current_volume = value;
    }
}

impl LengthModulation for Channel1 {
    fn is_length_enabled(&self) -> bool {
        is_set!(self.reg_nr14, 0b0100_0000)
    }

    fn length_counter(&self) -> u16 {
        self.length_counter as u16
    }

    fn set_length_counter(&mut self, value: u16) {
        self.length_counter = value as u8;
    }

    fn set_half_length_period(&mut self, half: bool) {
        self.length_half_period = half;
    }
}

impl SweepModulation for Channel1 {
    fn sweep_register(&self) -> u8 {
        self.reg_nr10
    }

    fn sweep_timer(&mut self) -> &mut u8 {
        &mut self.sweep_timer
    }

    fn shadow_frequency(&mut self) -> &mut u16 {
        &mut self.shadow_frequency
    }

    fn is_sweep_enabled(&self) -> bool {
        self.sweep_enabled
    }

    fn set_sweep_enabled(&mut self, enabled: bool) {
        self.sweep_enabled = enabled
    }

    fn set_sweep_was_decreasing(&mut self, decreasing: bool) {
        self.sweep_was_decreasing = decreasing;
    }
}

impl WaveModulation for Channel1 {
    fn wave_cursor(&self) -> u8 {
        self.wave_cursor
    }

    fn set_wave_cursor(&mut self, value: u8) {
        self.wave_cursor = value;
    }

    fn wave_duty(&self) -> u8 {
        self.reg_nr11 >> 6
    }
}

impl MemoryRegion for Channel1 {
    fn read(&self, address: u16) -> u8 {
        match address {
            REG_NR10_ADDR => self.reg_nr10 | 0b1000_0000,
            REG_NR11_ADDR => self.reg_nr11 | 0b0011_1111,
            REG_NR12_ADDR => self.reg_nr12,
            REG_NR13_ADDR => 0xFF,
            REG_NR14_ADDR => self.reg_nr14 | 0b1011_1111,
            _ => unreachable!(),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            REG_NR10_ADDR => {
                self.reg_nr10 = value;
                if !self.is_sweep_decreasing() && self.sweep_was_decreasing {
                    self.enabled = false;
                }
            },
            REG_NR11_ADDR => {
                self.length_counter = 64 - (value & 0b0011_1111);
                self.reg_nr11 = value
            },
            REG_NR12_ADDR => {
                self.reg_nr12 = value;
                if !self.is_dac_enabled() {
                    self.enabled = false;
                }
            },
            REG_NR13_ADDR => self.reg_nr13 = value,
            REG_NR14_ADDR => {
                let trigger = is_set!(value, 0b1000_0000);
                let length_enabled = is_set!(value, 0b0100_0000);

                if self.length_half_period && !self.is_length_enabled() && length_enabled && self.length_counter > 0 {
                        self.length_counter -= 1;

                        if self.length_counter == 0 {
                            self.enabled = false;
                        }
                    }

                self.reg_nr14 = value;
                // trigger a channel restart
                if trigger {
                    self.trigger();
                }
            },
            _ => unreachable!(),
        }
    }
}
