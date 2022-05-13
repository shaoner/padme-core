use crate::cpu::CLOCK_SPEED;
use crate::region::*;

use super::{Channel1, Channel2, Channel3, Channel4};
use super::modulation::*;

pub const AUDIO_SAMPLE_RATE: u32        = 48000; // Hz

const SAMPLE_PERIOD: u32                = CLOCK_SPEED / AUDIO_SAMPLE_RATE;
const FRAME_SEQUENCER_RATE: u32         = 512; // Hz
const FRAME_SEQUENCER_PERIOD: u32       = CLOCK_SPEED / FRAME_SEQUENCER_RATE;

//
// Default register values
//
const DEFAULT_REG_DMG_NR50: u8          = 0x77;
const DEFAULT_REG_DMG_NR51: u8          = 0xF3;
const DEFAULT_REG_DMG_NR52: u8          = 0xF1;

pub trait AudioSpeaker {
    fn set_samples(&mut self, left: f32, right: f32);
}

pub struct Apu {
    /// Channel control / ON-OFF / Volume (R/W)
    /// Bit   7: Output Vin to SO2 terminal (1=Enable)
    /// Bit 6-4: SO2 output level (volume)  (0-7)
    /// Bit   3: Output Vin to SO1 terminal (1=Enable)
    /// Bit 2-0: SO1 output level (volume)  (0-7)
    reg_nr50: u8,
    /// Selection of Sound output terminal (R/W)
    /// Bit   7: Output sound 4 to SO2 terminal
    /// Bit   6: Output sound 3 to SO2 terminal
    /// Bit   5: Output sound 2 to SO2 terminal
    /// Bit   4: Output sound 1 to SO2 terminal
    /// Bit   3: Output sound 4 to SO1 terminal
    /// Bit   2: Output sound 3 to SO1 terminal
    /// Bit   1: Output sound 2 to SO1 terminal
    /// Bit   0: Output sound 1 to SO1 terminal
    reg_nr51: u8,
    /// Sound on/off
    /// Bit   7: All sound on/off  (0: stop all sound circuits) (Read/Write)
    /// Bit   3: Sound 4 ON flag (Read Only)
    /// Bit   2: Sound 3 ON flag (Read Only)
    /// Bit   1: Sound 2 ON flag (Read Only)
    /// Bit   0: Sound 1 ON flag (Read Only)
    reg_nr52: u8,
    /// Number of ticks before stepping up the frame sequencer
    ticks: u32,
    /// Frame sequencer step % 8
    fs_step: u8,
    /// Sound Channel 1 - Tone & Sweep
    channel_1: Channel1,
    /// Sound Channel 2 - Tone
    channel_2: Channel2,
    /// Sound Channel 3 - Wave Output
    channel_3: Channel3,
    /// Sound Channel 4 - Noise
    channel_4: Channel4,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            reg_nr50: DEFAULT_REG_DMG_NR50,
            reg_nr51: DEFAULT_REG_DMG_NR51,
            reg_nr52: DEFAULT_REG_DMG_NR52,
            ticks: 0,
            fs_step: 0,
            channel_1: Channel1::new(),
            channel_2: Channel2::new(),
            channel_3: Channel3::new(),
            channel_4: Channel4::new(),
        }
    }

    #[inline]
    fn is_enabled(&self) -> bool {
        (self.reg_nr52 >> 7) != 0
    }

    #[inline]
    fn volume_left(&self) -> u8 {
        (self.reg_nr50 >> 4) & 0b0000_0111
    }

    #[inline]
    fn volume_right(&self) -> u8 {
        self.reg_nr50 & 0b0000_0111
    }

    fn handle_fs_step(&mut self) {
        let is_length_period = (self.fs_step % 2) == 0;
        self.channel_1.set_half_length_period(is_length_period);
        self.channel_2.set_half_length_period(is_length_period);
        self.channel_3.set_half_length_period(is_length_period);
        self.channel_4.set_half_length_period(is_length_period);

        // Step   Length Ctr  Vol Env     Sweep
        // ---------------------------------------
        // 0      Clock       -           -
        // 1      -           -           -
        // 2      Clock       -           Clock
        // 3      -           -           -
        // 4      Clock       -           -
        // 5      -           -           -
        // 6      Clock       -           Clock
        // 7      -           Clock       -
        // ---------------------------------------
        // Rate   256 Hz      64 Hz       128 Hz
        if is_length_period {
            // handle length
            self.channel_1.length_step();
            self.channel_2.length_step();
            self.channel_3.length_step();
            self.channel_4.length_step();
            if self.fs_step == 2 || self.fs_step == 6 {
                // handle sweep
                self.channel_1.sweep_step();
            }
        } else if self.fs_step == 7 {
            // handle volume
            self.channel_1.volume_step();
            self.channel_2.volume_step();
            self.channel_4.volume_step();
        }
        self.fs_step = (self.fs_step + 1) % 8;
    }

    fn mix_channels(&mut self, flag_offset: u8, volume: u8) -> f32 {
        // normalize volume
        let volume = (volume as f32) / 7.0;
        let mut sample = 0.0f32;

        if is_set!(self.reg_nr51, flag_offset) {
            sample += self.channel_1.dac_output();
        }
        if is_set!(self.reg_nr51, flag_offset << 1) {
            sample += self.channel_2.dac_output();
        }
        if is_set!(self.reg_nr51, flag_offset << 2) {
            sample += self.channel_3.dac_output();
        }
        if is_set!(self.reg_nr51, flag_offset << 3) {
            sample += self.channel_4.dac_output();
        }
        (sample * volume) / 4.0
    }

    pub fn step<AS: AudioSpeaker>(&mut self, speaker: &mut AS) {
        self.ticks = self.ticks.wrapping_add(1);

        self.channel_3.wave_just_read = false;

        self.channel_1.step();
        self.channel_2.step();
        self.channel_3.step();
        self.channel_4.step();

        // Every 8192 T-cycles, the frame sequencer is stepped
        if self.ticks % FRAME_SEQUENCER_PERIOD == 0 {
            self.handle_fs_step();
        }

        // Every sample period, we can send the current sample to the speaker
        // It's up to the speaker to store an audio buffer and play it a regular interval
        if self.ticks % SAMPLE_PERIOD == 0 {

            let left_volume = self.volume_left();
            let right_volume = self.volume_right();

            let s02 = self.mix_channels(0x10, left_volume);
            let s01 = self.mix_channels(0x01, right_volume);

            speaker.set_samples(s02, s01);
        }
    }
}

impl MemoryRegion for Apu {
    fn read(&self, address: u16) -> u8 {
        match address {
            REG_NR10_ADDR |
            REG_NR11_ADDR |
            REG_NR12_ADDR |
            REG_NR13_ADDR |
            REG_NR14_ADDR => {
                self.channel_1.read(address)
            },
            REG_NR21_ADDR |
            REG_NR22_ADDR |
            REG_NR23_ADDR |
            REG_NR24_ADDR => {
                self.channel_2.read(address)
            },
            REG_NR30_ADDR |
            REG_NR31_ADDR |
            REG_NR32_ADDR |
            REG_NR33_ADDR |
            REG_NR34_ADDR |
            WAVE_PATTERN_RAM_START..=WAVE_PATTERN_RAM_END => {
                self.channel_3.read(address)
            },
            REG_NR41_ADDR |
            REG_NR42_ADDR |
            REG_NR43_ADDR |
            REG_NR44_ADDR => {
                self.channel_4.read(address)
            },
            REG_NR50_ADDR => self.reg_nr50,
            REG_NR51_ADDR => self.reg_nr51,
            REG_NR52_ADDR => {
                let mut value = (self.reg_nr52 & 0b1000_0000) | 0b0111_0000;
                value |= self.channel_1.is_enabled() as u8;
                value |= (self.channel_2.is_enabled() as u8) << 1;
                value |= (self.channel_3.is_enabled() as u8) << 2;
                value |= (self.channel_4.is_enabled() as u8) << 3;
                value
            },
            _ => 0xFF,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        if !self.is_enabled() && !(WAVE_PATTERN_RAM_START..=WAVE_PATTERN_RAM_START).contains(&address)
            && address != REG_NR11_ADDR
            && address != REG_NR21_ADDR
            && address != REG_NR31_ADDR
            && address != REG_NR41_ADDR
            && address != REG_NR52_ADDR {
                return;
        }
        match address {
            REG_NR10_ADDR |
            REG_NR11_ADDR |
            REG_NR12_ADDR |
            REG_NR13_ADDR |
            REG_NR14_ADDR => {
                self.channel_1.write(address, value)
            },
            REG_NR21_ADDR |
            REG_NR22_ADDR |
            REG_NR23_ADDR |
            REG_NR24_ADDR => {
                self.channel_2.write(address, value)
            },
            REG_NR30_ADDR |
            REG_NR31_ADDR |
            REG_NR32_ADDR |
            REG_NR33_ADDR |
            REG_NR34_ADDR |
            WAVE_PATTERN_RAM_START..=WAVE_PATTERN_RAM_END => {
                self.channel_3.write(address, value)
            },
            REG_NR41_ADDR |
            REG_NR42_ADDR |
            REG_NR43_ADDR |
            REG_NR44_ADDR => {
                self.channel_4.write(address, value)
            },
            REG_NR50_ADDR => self.reg_nr50 = value,
            REG_NR51_ADDR => self.reg_nr51 = value,
            REG_NR52_ADDR => {
                let enabled = is_set!(value, 0b1000_0000);
                let len_ch1 = self.channel_1.length_counter();
                let len_ch2 = self.channel_2.length_counter();
                let len_ch3 = self.channel_3.length_counter();
                let len_ch4 = self.channel_4.length_counter();

                if enabled && !self.is_enabled() {
                    self.fs_step = 0;
                } else if !enabled && self.is_enabled() {
                    for addr in REG_NR10_ADDR..REG_NR52_ADDR {
                        self.write(addr, 0x00);
                    }
                }
                // restore old counters
                self.channel_1.set_length_counter(len_ch1);
                self.channel_2.set_length_counter(len_ch2);
                self.channel_3.set_length_counter(len_ch3);
                self.channel_4.set_length_counter(len_ch4);

                self.reg_nr52 = value & 0x80
            },
            _ => (),
        }
    }
}
