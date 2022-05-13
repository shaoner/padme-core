use crate::region::*;

use super::modulation::*;

//
// Default register values
//
const DEFAULT_REG_DMG_NR30: u8          = 0x7F;
const DEFAULT_REG_DMG_NR31: u8          = 0xFF;
const DEFAULT_REG_DMG_NR32: u8          = 0x9F;
const DEFAULT_REG_DMG_NR33: u8          = 0xFF;
const DEFAULT_REG_DMG_NR34: u8          = 0xBF;

pub struct Channel3 {
    enabled: bool,
    /// Bit 7  : Sound Channel 3 Off  (0=Stop, 1=Playback) (Read/Write)
    reg_nr30: u8,
    /// Bit 7-0: Sound length (Write only) (t1: 0 - 255)
    reg_nr31: u8,
    /// Bits 6-5: Select output level (Read/Write)
    /// %00 Mute (No sound)
    /// %01 100% volume (Produce Wave Pattern RAM Data as it is)
    /// %10 50% volume (Produce Wave Pattern RAM data shifted once to the right)
    /// %11 25% volume (Produce Wave Pattern RAM data shifted twice to the right)
    reg_nr32: u8,
    /// Frequency lower 8 bits
    reg_nr33: u8,
    /// Bit 7  : Initial (1=Restart Sound)
    /// Bit 6  : Counter/consecutive selection (Read/Write)
    /// Bit 2-0: Frequency's higher 3 bits (x) (Write Only)
    reg_nr34: u8,
    /// Length counter between 0 and 64
    length_counter: u16,
    /// Length period is half
    length_half_period: bool,
    /// Frequency timer between 4 and 8192
    frequency_timer: u16,
    /// Wave cursor position in ram
    wave_cursor: u8,
    /// Wave ram as 32 x 4bits
    wave_ram: [u8; 16],
    /// Current wave sample taken from the wave ram
    current_wave_sample: u8,
    /// DMG needs can only reads wave after a few apu cycles
    pub wave_just_read: bool,
}

impl Channel3 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            reg_nr30: DEFAULT_REG_DMG_NR30,
            reg_nr31: DEFAULT_REG_DMG_NR31,
            reg_nr32: DEFAULT_REG_DMG_NR32,
            reg_nr33: DEFAULT_REG_DMG_NR33,
            reg_nr34: DEFAULT_REG_DMG_NR34,
            length_counter: 256,
            length_half_period: false,
            frequency_timer: 4,
            wave_cursor: 0,
            wave_ram: [0; 16],
            current_wave_sample: 0,
            wave_just_read: false,
        }
    }

    #[inline]
    fn output_level(&self) -> u8 {
        (self.reg_nr32 >> 5) & 0b0000_0011
    }
}

impl Channel for Channel3 {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_dac_enabled(&self) -> bool {
        is_set!(self.reg_nr30, 0b1000_0000)
    }

    fn trigger(&mut self) {
        if self.is_dac_enabled() {
            self.enabled = true;
        }
        if self.length_counter == 0 {
            self.set_length_counter(256);
            if self.is_length_enabled() && self.length_half_period {
                self.length_counter -= 1;
            }
        }
        self.reset_frequency_timer();
        // Not my proudest hack, it just seems that adding 6 ticks helps syncing
        // wave_just_read to be set at the right time
        self.frequency_timer += 6;
        self.reset_wave();
    }
}

impl Clock for Channel3 {
    fn frequency(&self) -> u32 {
        (((self.reg_nr34 & 0b0000_0111) as u32) << 8) | (self.reg_nr33 as u32)
    }

    fn frequency_timer(&self) -> u32 {
        self.frequency_timer as u32
    }

    fn set_frequency_timer(&mut self, value: u32) {
        self.frequency_timer = value as u16;
    }

    fn reset_frequency_timer(&mut self) {
        let timer = (0x800 - self.frequency()) * 2;
        self.set_frequency_timer(timer);
    }
}

impl LengthModulation for Channel3 {
    fn is_length_enabled(&self) -> bool {
        is_set!(self.reg_nr34, 0b0100_0000)
    }

    fn length_counter(&self) -> u16 {
        self.length_counter
    }

    fn set_length_counter(&mut self, value: u16) {
        self.length_counter = value;
    }

    fn set_half_length_period(&mut self, half: bool) {
        self.length_half_period = half;
    }
}

impl WaveModulation for Channel3 {
    fn wave_cursor(&self) -> u8 {
        self.wave_cursor
    }

    fn set_wave_cursor(&mut self, value: u8) {
        self.wave_cursor = value;
    }

    fn inc_wave_cursor(&mut self) {
        self.wave_cursor = (self.wave_cursor + 1) % 32;
        self.current_wave_sample = self.wave_ram[(self.wave_cursor / 2) as usize];
        self.wave_just_read = true;
    }

    fn wave_duty(&self) -> u8 {
        0
    }

    fn wave_sample(&self) -> u8 {
        if self.wave_cursor & 0x1 == 0x1 {
            self.current_wave_sample >> 4
        } else {
            self.current_wave_sample & 0xF
        }
    }
}

impl DigitalAmplitude for Channel3 {
    fn digital_amplitude(&self) -> u8 {
        let sample = self.wave_sample() as u8;
        let volume_shift = match self.output_level() {
            0x00 => 4,
            0x01 => 0,
            0x02 => 1,
            0x03 => 2,
            _ => unreachable!(),
        };

        sample >> volume_shift
    }
}

impl MemoryRegion for Channel3 {
    fn read(&self, address: u16) -> u8 {
        match address {
            REG_NR30_ADDR => self.reg_nr30 | 0b0111_1111,
            REG_NR31_ADDR => 0xFF,
            REG_NR32_ADDR => self.reg_nr32 | 0b1001_1111,
            REG_NR33_ADDR => 0xFF,
            REG_NR34_ADDR => self.reg_nr34 | 0b1011_1111,
            WAVE_PATTERN_RAM_START..=WAVE_PATTERN_RAM_END => {
                if self.enabled {
                    if !self.wave_just_read {
                        0xFF
                    } else {
                        self.wave_ram[(self.wave_cursor / 2) as usize]
                    }
                } else {
                    self.wave_ram[(address - WAVE_PATTERN_RAM_START) as usize]
                }
            },
            _ => unreachable!(),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            REG_NR30_ADDR => {
                self.reg_nr30 = value;
                if !self.is_dac_enabled() {
                    self.enabled = false;
                }
            },
            REG_NR31_ADDR => {
                self.set_length_counter(256 - (value as u16));
                self.reg_nr31 = value;
            },
            REG_NR32_ADDR => self.reg_nr32 = value,
            REG_NR33_ADDR => self.reg_nr33 = value,
            REG_NR34_ADDR => {
                let trigger = is_set!(value, 0b1000_0000);
                let length_enabled = is_set!(value, 0b0100_0000);

                if self.length_half_period && !self.is_length_enabled() && length_enabled && self.length_counter > 0 {
                    self.length_counter -= 1;

                    if self.length_counter == 0 {
                        self.enabled = false;
                    }
                }

                self.reg_nr34 = value;
                // trigger a channel restart
                if trigger {
                    self.trigger();
                }
            },
            WAVE_PATTERN_RAM_START..=WAVE_PATTERN_RAM_END => {
                if self.enabled {
                    if self.wave_just_read {
                        self.wave_ram[(self.wave_cursor / 2) as usize] = value
                    }
                } else {
                    self.wave_ram[(address - WAVE_PATTERN_RAM_START) as usize] = value
                }
            },
            _ => unreachable!(),
        }
    }
}
