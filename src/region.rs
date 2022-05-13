//
// Memory registers
//

// --- Serial ---
// Serial transfer data
pub const REG_SB_ADDR: u16              = 0xFF01;
// Serial transfer control
pub const REG_SC_ADDR: u16              = 0xFF02;

// --- Timer ---
// Divider
pub const REG_DIV_ADDR: u16             = 0xFF04;
// Timer counter
pub const REG_TIMA_ADDR: u16            = 0xFF05;
// Timer reset value
pub const REG_TMA_ADDR: u16             = 0xFF06;
// Timer control
pub const REG_TAC_ADDR: u16             = 0xFF07;

// --- Sound ---
// Channel 1: Sweep
pub const REG_NR10_ADDR: u16            = 0xFF10;
// Channel 1: Sound Length / Wave Pattern Duty
pub const REG_NR11_ADDR: u16            = 0xFF11;
// Channel 1: Volume Envelope
pub const REG_NR12_ADDR: u16            = 0xFF12;
// Channel 1: Frequency lo data
pub const REG_NR13_ADDR: u16            = 0xFF13;
// Channel 1: Restart / Frequency hi data
pub const REG_NR14_ADDR: u16            = 0xFF14;
// Channel 2: Sound Length / Wave Pattern Duty
pub const REG_NR21_ADDR: u16            = 0xFF16;
// Channel 2: Volume Envelope
pub const REG_NR22_ADDR: u16            = 0xFF17;
// Channel 2: Frequency lo data
pub const REG_NR23_ADDR: u16            = 0xFF18;
// Channel 2: Restart / Frequency hi data
pub const REG_NR24_ADDR: u16            = 0xFF19;
// Channel 3: Sound on / off
pub const REG_NR30_ADDR: u16            = 0xFF1A;
// Channel 3: Sound length
pub const REG_NR31_ADDR: u16            = 0xFF1B;
// Channel 3: Volume
pub const REG_NR32_ADDR: u16            = 0xFF1C;
// Channel 3: Frequency lo data
pub const REG_NR33_ADDR: u16            = 0xFF1D;
// Channel 3: Restart / Frequency high data
pub const REG_NR34_ADDR: u16            = 0xFF1E;
// Channel 3: Wave pattern ram = 32 x 4bit
pub const WAVE_PATTERN_RAM_START: u16   = 0xFF30;
pub const WAVE_PATTERN_RAM_END: u16     = 0xFF3F;
// Channel 4: Sound Length
pub const REG_NR41_ADDR: u16            = 0xFF20;
// Channel 4: Volume Envelope
pub const REG_NR42_ADDR: u16            = 0xFF21;
// Channel 4: Polynomial counter
pub const REG_NR43_ADDR: u16            = 0xFF22;
// Channel 4: Restart / initial length
pub const REG_NR44_ADDR: u16            = 0xFF23;
// Sound controller: Channel control / ON-OFF / Volume
pub const REG_NR50_ADDR: u16            = 0xFF24;
// Sound controller: Selection of Sound output terminal
pub const REG_NR51_ADDR: u16            = 0xFF25;
// Sound controller: Channel on/off
pub const REG_NR52_ADDR: u16            = 0xFF26;

// --- PPU ---
// LCD control
pub const REG_LCDC_ADDR: u16            = 0xFF40;
// LCD status
pub const REG_STAT_ADDR: u16            = 0xFF41;
// Scroll Y
pub const REG_SCY_ADDR: u16             = 0xFF42;
// Scroll X
pub const REG_SCX_ADDR: u16             = 0xFF43;
// LCD Y
pub const REG_LY_ADDR: u16              = 0xFF44;
// LCD Y Compare
pub const REG_LYC_ADDR: u16             = 0xFF45;
// DMA
pub const REG_DMA_ADDR: u16             = 0xFF46;
// BG Palette - Non CGB Mode Only
pub const REG_BGP_ADDR: u16             = 0xFF47;
// Obj Palette 0 - Non CGB Mode Only
pub const REG_OBP0_ADDR: u16            = 0xFF48;
// Obj Palette 1 - Non CGB Mode Only
pub const REG_OBP1_ADDR: u16            = 0xFF49;
// Window Y
pub const REG_WY_ADDR: u16              = 0xFF4A;
// Window X + 7
pub const REG_WX_ADDR: u16              = 0xFF4B;
// Interrupts flags
pub const REG_IF_ADDR: u16              = 0xFF0F;
// Interrupts enable
pub const REG_IE_ADDR: u16              = 0xFFFF;

//
// Memory Map of regions
//
// 0x0000 - ROM bank 0: 16KB (in cartridge, fixed)
pub const ROM_REGION_START: u16         = 0x0000;
pub const ROM_REGION_END: u16           = 0x7FFF;
pub const ROM_REGION_SIZE: usize        = (ROM_REGION_END - ROM_REGION_START + 1) as usize;
// 0x7FFF ---
// 0x8000 - Video RAM: 8KB (switchable bank 0-1 in CGB Mode)
pub const VRAM_REGION_START: u16        = 0x8000;
pub const VRAM_REGION_END: u16          = 0x9FFF;
pub const VRAM_REGION_SIZE: usize       = (VRAM_REGION_END - VRAM_REGION_START + 1) as usize;

// 0x9FFF ---
// 0xA000 - External RAM: 8KB (in cartridge, switchable bank, if any)
pub const ERAM_REGION_START: u16        = 0xA000;
pub const ERAM_REGION_END: u16          = 0xBFFF;
pub const ERAM_REGION_SIZE: usize       = (ERAM_REGION_END - ERAM_REGION_START + 1) as usize;
// 0xBFFF ---
// 0xC000 - Working RAM bank 0 + switchable: 8KB
pub const WRAM_REGION_START: u16        = 0xC000;
pub const WRAM_REGION_END: u16          = 0xDFFF;
pub const WRAM_REGION_SIZE: usize       = (WRAM_REGION_END - WRAM_REGION_START + 1) as usize;
// 0xDFFF ---
// 0xE000 - Echo RAM of C000-DDFF: 8KB - 512 (typically unused)
pub const ECHORAM_REGION_START: u16     = 0xE000;
pub const ECHORAM_REGION_END: u16       = 0xFDFF;
// 0xFDFF ---
// 0xFE00 - Object Attribute Memory: 160B
pub const OAM_REGION_START: u16         = 0xFE00;
pub const OAM_REGION_END: u16           = 0xFE9F;
pub const OAM_REGION_SIZE: usize        = (OAM_REGION_END - OAM_REGION_START + 1) as usize;
// 0xFE9F ---
// 0xFEA0 - Unusable
// 0xFEFF ---
// 0xFF00 - Hardware I/O ports: 128B
pub const IO_REGION_START: u16          = 0xFF00;
pub const IO_JOYPAD_REGION: u16         = 0xFF00;
pub const IO_SERIAL_REGION_START: u16   = 0xFF01;
pub const IO_SERIAL_REGION_END: u16     = 0xFF02;
pub const IO_TIMER_REGION_START: u16    = 0xFF04;
pub const IO_TIMER_REGION_END: u16      = 0xFF07;
pub const IO_SOUND_REGION_START: u16    = 0xFF10;
pub const IO_SOUND_REGION_END: u16      = 0xFF3F;
pub const IO_PPU_REGION_START: u16      = 0xFF40;
pub const IO_PPU_REGION_END: u16        = 0xFF4B;
// 0xFF7F ---
// 0xFF80 - High ram: 127B
pub const HRAM_REGION_START: u16        = 0xFF80;
pub const HRAM_REGION_END: u16          = 0xFFFE;
pub const HRAM_REGION_SIZE: usize       = (HRAM_REGION_END - HRAM_REGION_START + 1) as usize;
// 0xFFFE ---
// 0xFFFF - Interrupt enable register
// ---------------------------------------------

/// All memory mapped devices should implement this trait
pub trait MemoryRegion {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}
