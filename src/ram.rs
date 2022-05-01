use crate::region::*;

pub struct Ram<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> Ram<N> {
    pub fn new() -> Self {
        Self { bytes: [0u8; N] }
    }
}


impl<const N: usize> MemoryRegion for Ram<N> {
    fn read(&self, address: u16) -> u8 {
        self.bytes[address as usize]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.bytes[address as usize] = value;
    }
}
