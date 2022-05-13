//
// All wave duty patterns
//
const WAVE_DUTY_PATTERNS: [[u8; 8]; 4] = [
    // 12.5% (_-------_-------_-------)
    [0, 0, 0, 0, 0, 0, 0, 1],
    // 25% (__------__------__------)
    [1, 0, 0, 0, 0, 0, 0, 1],
    // 50% (____----____----____----)
    [1, 0, 0, 0, 0, 1, 1, 1],
    // 75% (______--______--______--)
    [0, 1, 1, 1, 1, 1, 1, 0]
];

pub trait DigitalAmplitude {
    fn digital_amplitude(&self) -> u8;
}

// pub trait Clock {
//     fn step(&mut self);
// }

pub trait Clock {
    fn frequency(&self) -> u32;

    fn frequency_timer(&self) -> u32;

    fn set_frequency_timer(&mut self, value: u32);

    fn set_frequency(&mut self, _value: u32) {
    }

    fn reset_frequency_timer(&mut self) {
        let timer = (0x800 - self.frequency()) * 4;
        self.set_frequency_timer(timer);
    }
}

pub trait Sample {
    fn sample(&self) -> u8;
}

pub trait Step {
    fn step(&mut self);
}

pub trait Channel: DigitalAmplitude + Clock + Sample + Step {
    fn is_enabled(&self) -> bool;

    fn set_enabled(&mut self, enabled: bool);

    fn is_dac_enabled(&self) -> bool;

    fn trigger(&mut self);

    fn dac_output(&self) -> f32 {
        if self.is_enabled() && self.is_dac_enabled() {
            // from [0x0; 0xF] to [-1; 1]
            (self.digital_amplitude() as f32 / 7.5) - 1.0
        } else {
            0.0
        }
    }
}

pub trait EnvelopeModulation {
    fn envelope_register(&self) -> u8;

    fn envelope_timer(&mut self) -> &mut u8;

    fn envelope_volume(&self) -> u8;

    fn set_envelope_volume(&mut self, value: u8);

    fn envelope_init_volume(&self) -> u8 {
        self.envelope_register() >> 4
    }

    fn is_envelope_increasing(&self) -> bool {
        is_set!(self.envelope_register(), 0b0000_1000)
    }

    fn envelope_period(&self) -> u8 {
        self.envelope_register() & 0b0000_0111
    }

    fn reset_envelope(&mut self) {
        let init_volume = self.envelope_init_volume();
        self.set_envelope_volume(init_volume);
        *self.envelope_timer() = self.envelope_period();
    }

    fn volume_step(&mut self) {
        let period = self.envelope_period();
        if period == 0 {
            return;
        }

        let env_timer = self.envelope_timer();
        if *env_timer > 0 {
            *env_timer -= 1;

            // Adjust time when we reached the period
            if *env_timer == 0 {
                *env_timer = period;
                let increasing = self.is_envelope_increasing();
                let volume = self.envelope_volume();
                if volume < 0xF && increasing {
                    self.set_envelope_volume(volume + 1);
                } else if volume > 0x0 && !increasing {
                    self.set_envelope_volume(volume - 1);
                }
            }
        }
    }
}

pub trait LengthModulation: Channel {
    fn is_length_enabled(&self) -> bool;

    fn length_counter(&self) -> u16;

    fn set_length_counter(&mut self, value: u16);

    fn set_half_length_period(&mut self, half: bool);

    fn length_step(&mut self) {
        let mut counter = self.length_counter();

        if self.is_length_enabled() && counter > 0 {
            counter -= 1;
            self.set_length_counter(counter);
            if counter == 0 {
                self.set_enabled(false);
            }
        }
    }
}

pub trait SweepModulation: Channel + WaveModulation {
    fn sweep_register(&self) -> u8;

    fn sweep_timer(&mut self) -> &mut u8;

    fn shadow_frequency(&mut self) -> &mut u16;

    fn is_sweep_enabled(&self) -> bool;

    fn set_sweep_enabled(&mut self, enabled: bool);

    fn set_sweep_was_decreasing(&mut self, decreasing: bool);

    #[inline]
    fn sweep_period(&self) -> u8 {
        (self.sweep_register() >> 4) & 0b0000_0111
    }

    #[inline]
    fn is_sweep_decreasing(&self) -> bool {
        is_set!(self.sweep_register(), 0b0000_1000)
    }

    #[inline]
    fn sweep_shift(&self) -> u8 {
        self.sweep_register() & 0b0000_0111
    }

    fn reset_sweep(&mut self) {
        self.set_sweep_was_decreasing(false);
        *self.shadow_frequency() = self.frequency() as u16;
        let period = self.sweep_period();
        let period_timer = self.sweep_timer();
        *period_timer = if period > 0 {
            period
        } else {
            8
        };
        let sweep_shift = self.sweep_shift();

        self.set_sweep_enabled(period != 0 || sweep_shift != 0);

        if sweep_shift > 0 {
            self.calculate_sweep_frequency();
        }
    }

    fn calculate_sweep_frequency(&mut self) -> u16 {
        let shadow_frequency = *self.shadow_frequency();
        let new_frequency = shadow_frequency >> self.sweep_shift();

        let new_frequency = if self.is_sweep_decreasing() {
            self.set_sweep_was_decreasing(true);
            shadow_frequency - new_frequency
        } else {
            shadow_frequency + new_frequency
        };

        // overflow check
        if new_frequency > 0x7FF {
            self.set_enabled(false);
        }

        new_frequency
    }

    fn sweep_step(&mut self) {
        let period = self.sweep_period();
        let timer = self.sweep_timer();
        if *timer > 0 {
            *timer -= 1;
        }

        if *timer == 0 {
            *timer = if period > 0 {
                period
            } else {
                8
            };

            if self.is_sweep_enabled() && self.sweep_period() > 0 {
                let new_frequency = self.calculate_sweep_frequency();
                if new_frequency <= 0x7FF && self.sweep_shift() != 0 {
                    *self.shadow_frequency() = new_frequency;
                    self.set_frequency(new_frequency as u32);
                    self.calculate_sweep_frequency();
                }
            }
        }
    }
}

pub trait WaveModulation {
    fn wave_cursor(&self) -> u8;

    fn wave_duty(&self) -> u8;

    fn set_wave_cursor(&mut self, value: u8);

    fn inc_wave_cursor(&mut self) {
        let wave_cursor = self.wave_cursor();
        self.set_wave_cursor((wave_cursor + 1) % 8)
    }

    fn reset_wave(&mut self) {
        self.set_wave_cursor(0);
    }

    fn wave_sample(&self) -> u8 {
        let duty = self.wave_duty() as usize;
        let cursor = self.wave_cursor() as usize;

        WAVE_DUTY_PATTERNS[duty][cursor]
    }
}

impl<T: WaveModulation> Sample for T {
    fn sample(&self) -> u8 {
        self.wave_sample()
    }
}

impl<T: Sample + EnvelopeModulation> DigitalAmplitude for T {
    fn digital_amplitude(&self) -> u8 {
        let sample = self.sample();
        let volume = self.envelope_volume();

        sample * volume
    }
}

impl<T: WaveModulation + Channel> Step for T {
    fn step(&mut self) {
        let timer = self.frequency_timer() - 1;

        self.set_frequency_timer(timer);

        if timer == 0 {
            self.reset_frequency_timer();
            self.inc_wave_cursor();
        }
    }
}
