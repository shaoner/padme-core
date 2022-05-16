use crate::{AudioSpeaker, Pixel, Screen, SerialOutput};

pub struct NoScreen;

impl Screen for NoScreen {
    fn set_pixel(&mut self, _px: &Pixel, _x: u8, _y: u8) {
    }

    fn update(&mut self) {
    }
}

pub struct NoSpeaker;

impl AudioSpeaker for NoSpeaker {
    fn set_samples(&mut self, _left: f32, _right: f32) {
    }
}

pub struct NoSerial;

impl SerialOutput for NoSerial {
    fn putchar(&mut self, _ch: u8) {
    }
}
