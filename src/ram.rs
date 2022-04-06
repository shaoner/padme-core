use crate::region::*;

pub struct WorkRam {
    bytes: [u8; WRAM_REGION_SIZE]
}

impl WorkRam {
    pub fn new() -> Self {
        Self { bytes: [0u8; WRAM_REGION_SIZE] }
    }
}

impl MemoryRegion for WorkRam {
    fn read(&self, address: u16) -> u8 {
        self.bytes[(address - WRAM_REGION_START) as usize]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.bytes[(address - WRAM_REGION_START) as usize] = value;
    }
}


pub struct HighRam {
    bytes: [u8; HRAM_REGION_SIZE]
}

impl HighRam {
    pub fn new() -> Self {
        Self { bytes: [0u8; HRAM_REGION_SIZE] }
    }
}

impl MemoryRegion for HighRam {
    fn read(&self, address: u16) -> u8 {
        self.bytes[(address - HRAM_REGION_START) as usize]
    }

    fn write(&mut self, address: u16, value: u8) {
        self.bytes[(address - HRAM_REGION_START) as usize] = value;
    }
}
