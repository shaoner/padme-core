#[cfg(debug_assertions)]
use core::fmt;
use core::ops::Deref;
use core::str;

use crate::region::*;
use crate::Error;
use super::{CgbMode, CartridgeType, Licensee};
use super::mbc::*;

const HEADER_TITLE_START: usize         = 0x0134;
const HEADER_TITLE_END: usize           = 0x0143;
const HEADER_CGB_FLAG: usize            = 0x0143;
const HEADER_NEW_LICENSEE_CODE: usize   = 0x0144;
const HEADER_SGB_FLAG: usize            = 0x0146;
const HEADER_CARTRIDGE_TYPE: usize      = 0x0147;
const HEADER_ROM_SIZE: usize            = 0x0148;
const HEADER_RAM_SIZE: usize            = 0x0149;
const HEADER_DESTINATION_CODE: usize    = 0x014A;
const HEADER_OLD_LICENSEE_CODE: usize   = 0x014B;
const HEADER_VERSION: usize             = 0x014C;
const HEADER_HEADER_CHECKSUM: usize     = 0x014D;

pub struct Rom<T: Deref<Target=[u8]>> {
    /// Cartridge data, this is provided by the user depending on their platform
    /// This can be a Vec<u8>, a static array,
    /// Or generally any kind of structure that can be dereferenced to a u8
    storage: T,
    /// Support for Mbc0, Mbc1, etc
    mbc_ctrl: Mbc,
}

impl<T: Deref<Target=[u8]>> Rom<T> {
    /// Build a rom from a sequence of storage
    pub fn load(storage: T) -> Result<Self, Error> {
        if storage.len() < ROM_REGION_SIZE {
            Err(Error::InvalidRomSize(storage.len()))
        } else {
            let mut rom = Self {
                storage,
                mbc_ctrl: Mbc::from(Mbc0),
            };
            // MBC can be a dynamically dispatched on the stack
            // which is awesome in a no_std / no alloc environment
            // This still can be improved by extracting the Rom header
            // which would allow setting the mbc controller before creating the rom instance
            rom.mbc_ctrl = match rom.cartridge_type() {
                CartridgeType::RomOnly => Mbc::from(Mbc0),
                CartridgeType::Mbc1 |
                CartridgeType::Mbc1Ram |
                CartridgeType::Mbc1RamBattery => Mbc::from(Mbc1::new()),
                CartridgeType::Mbc3 |
                CartridgeType::Mbc3Ram |
                CartridgeType::Mbc3RamBattery |
                CartridgeType::Mbc3TimerBattery |
                CartridgeType::Mbc3TimerRamBattery => Mbc::from(Mbc3::new()),
                _ => unimplemented!(),
            };

            Ok(rom)
        }
    }

    /// Shortcut to retrieve header part
    pub fn header(&self) -> &[u8] {
        &self.storage[HEADER_TITLE_START..HEADER_HEADER_CHECKSUM]
    }

    /// Shortcut to retrieve the location of the title
    pub fn title(&self) -> Result<&str, str::Utf8Error> {
        let title_part = &self.storage[HEADER_TITLE_START..=HEADER_TITLE_END];
        for (i, &byte) in title_part.iter().enumerate() {
            if byte == 0x00 {
                return str::from_utf8(
                    &self.storage[HEADER_TITLE_START..(HEADER_TITLE_START + i)]
                );
            }
        }

        str::from_utf8(title_part)
    }

    /// Shortcut to retrieve the cgb mode from the header
    pub fn cgb_mode(&self) -> CgbMode {
        let cgb_flag = self.storage[HEADER_CGB_FLAG];

        match cgb_flag {
            0xC0 => CgbMode::Cgb,
            cgb_flag if cgb_flag & 0x80 == 0x80 => CgbMode::Both,
            _ => CgbMode::None,
        }
    }

    /// Shortcut to retrieve the rom size from the header
    pub fn size(&self) -> u16 {
        let n = self.storage[HEADER_ROM_SIZE];

        match n {
            0x00..=0x08 => (32 << n) as u16,
            _ => 0u16,
        }
    }

    /// Shortcut to retrieve the ram size from the header
    pub fn ram_size(&self) -> u16 {
        match self.storage[HEADER_RAM_SIZE] {
            0x00 => 0u16,
            0x02 => 8u16,
            0x03 => 32u16,
            0x04 => 128u16,
            0x05 => 64u16,
            _ => 0x0u16,
        }
    }

    /// Shortcut to retrieve if the rom supports sgb from the header
    pub fn is_sgb(&self) -> bool {
        self.storage[HEADER_SGB_FLAG] == 0x03
    }

    /// Shortcut to retrieve the cartridge type from the header
    pub fn cartridge_type(&self) -> CartridgeType {
        match self.storage[HEADER_CARTRIDGE_TYPE] {
            0x00 => CartridgeType::RomOnly,
            0x01 => CartridgeType::Mbc1,
            0x02 => CartridgeType::Mbc1Ram,
            0x03 => CartridgeType::Mbc1RamBattery,
            0x05 => CartridgeType::Mbc2,
            0x06 => CartridgeType::Mbc2Battery,
            0x08 => CartridgeType::RomRam,
            0x09 => CartridgeType::RomRamBattery,
            0x0B => CartridgeType::Mmm01,
            0x0C => CartridgeType::Mmm01Ram,
            0x0D => CartridgeType::Mmm01RamBattery,
            0x0F => CartridgeType::Mbc3TimerBattery,
            0x10 => CartridgeType::Mbc3TimerRamBattery,
            0x11 => CartridgeType::Mbc3,
            0x12 => CartridgeType::Mbc3Ram,
            0x13 => CartridgeType::Mbc3RamBattery,
            0x19 => CartridgeType::Mbc5,
            0x1A => CartridgeType::Mbc5Ram,
            0x1B => CartridgeType::Mbc5RamBattery,
            0x1C => CartridgeType::Mbc5Rumble,
            0x1D => CartridgeType::Mbc5RumbleRam,
            0x1E => CartridgeType::Mbc5RumbleRamBattery,
            0x20 => CartridgeType::Mbc6,
            0x22 => CartridgeType::Mbc7SensorRumbleRamBattery,
            0xFC => CartridgeType::PocketCamera,
            0xFD => CartridgeType::BandaiTama5,
            0xFE => CartridgeType::HuC3,
            0xFF => CartridgeType::HuC1RamBattery,
            _ => CartridgeType::Unknown,
        }
    }

    /// Shortcut to retrieve if the cartridge is japanese from the header
    pub fn is_jp(&self) -> bool {
        self.storage[HEADER_DESTINATION_CODE] == 0x00
    }

    /// Shortcut to retrieve the version from the header
    pub fn version(&self) -> u8 {
        self.storage[HEADER_VERSION]
    }

    /// Verify the checksum from the header
    pub fn verify_header_checksum(&self) -> bool {
        let mut x = 0u8;

        for &byte in self.header().iter() {
            x = x.wrapping_sub(byte).wrapping_sub(1);
        }

        x == self.storage[HEADER_HEADER_CHECKSUM]
    }

    /// Shortcut to retrieve the licensee from the header
    pub fn licensee(&self) -> Licensee {
        let old_licensee_code = self.storage[HEADER_OLD_LICENSEE_CODE];

        match old_licensee_code {
            0x00 => Licensee::None,
            0x01 => Licensee::Nintendo,
            0x0C => Licensee::EliteSystems,
            0x13 => Licensee::ElectronicArts,
            0x18 => Licensee::HudsonSoft,
            0x19 => Licensee::ItcEntertainment,
            0x1A => Licensee::Yanoman,
            0x1D => Licensee::Clary,
            0x1F => Licensee::Virgin,
            0x24 => Licensee::PcmComplete,
            0x25 => Licensee::SanX,
            0x28 => Licensee::KotobukiSystems,
            0x29 => Licensee::Seta,
            0x30 => Licensee::Infogrames,
            0x31 => Licensee::Nintendo,
            0x32 => Licensee::Bandai,
            0x34 => Licensee::Konami,
            0x35 => Licensee::Hector,
            0x38 => Licensee::Capcom,
            0x39 => Licensee::Banpresto,
            0x3C => Licensee::EntertainmentI,
            0x3E => Licensee::Gremlin,
            0x41 => Licensee::Ubisoft,
            0x42 => Licensee::Atlus,
            0x44 => Licensee::Malibu,
            0x46 => Licensee::Angel,
            0x47 => Licensee::SpectrumHoloby,
            0x49 => Licensee::Irem,
            0x4A => Licensee::Virgin,
            0x4D => Licensee::Malibu,
            0x4F => Licensee::UsGold,
            0x50 => Licensee::Absolute,
            0x51 => Licensee::Acclaim,
            0x52 => Licensee::Activision,
            0x53 => Licensee::AmericanSammy,
            0x54 => Licensee::Gametek,
            0x55 => Licensee::ParkPlace,
            0x56 => Licensee::Ljn,
            0x57 => Licensee::Matchbox,
            0x59 => Licensee::MiltonBradley,
            0x5A => Licensee::Mindscape,
            0x5B => Licensee::Romstar,
            0x5C => Licensee::NaxatSoft,
            0x5D => Licensee::Tradewest,
            0x60 => Licensee::Titus,
            0x61 => Licensee::Virgin,
            0x67 => Licensee::Ocean,
            0x69 => Licensee::ElectronicArts,
            0x6E => Licensee::EliteSystems,
            0x6F => Licensee::ElectroBrain,
            0x70 => Licensee::Infogrames,
            0x71 => Licensee::Interplay,
            0x72 => Licensee::Broderbund,
            0x73 => Licensee::SculpteredSoft,
            0x75 => Licensee::TheSalesCurve,
            0x78 => Licensee::Thq,
            0x79 => Licensee::Accolade,
            0x7A => Licensee::TriffixEntertainment,
            0x7C => Licensee::Microprose,
            0x7F => Licensee::Kemco,
            0x80 => Licensee::Misawa,
            0x83 => Licensee::Lozc,
            0x86 => Licensee::TokumaShoten,
            0x8B => Licensee::BulletProof,
            0x8C => Licensee::VicTokai,
            0x8E => Licensee::Ape,
            0x8F => Licensee::Imax,
            0x91 => Licensee::ChunSoft,
            0x92 => Licensee::VideoSystem,
            0x93 => Licensee::Tsuburava,
            0x95 => Licensee::Varie,
            0x96 => Licensee::YonezawaSpal,
            0x97 => Licensee::Kaneko,
            0x99 => Licensee::Arc,
            0x9A => Licensee::NihonBussan,
            0x9B => Licensee::Tecmo,
            0x9C => Licensee::Imagineer,
            0x9D => Licensee::Banpresto,
            0x9F => Licensee::Nova,
            0xA1 => Licensee::HoriElectric,
            0xA2 => Licensee::Bandai,
            0xA4 => Licensee::Konami,
            0xA6 => Licensee::Kawada,
            0xA7 => Licensee::Takara,
            0xA9 => Licensee::TechnosJapan,
            0xAA => Licensee::Broderbund,
            0xAC => Licensee::ToeiAnimation,
            0xAD => Licensee::Toho,
            0xAF => Licensee::Namco,
            0xB0 => Licensee::Acclaim,
            0xB1 => Licensee::Nexoft,
            0xB2 => Licensee::Bandai,
            0xB4 => Licensee::Enix,
            0xB6 => Licensee::Hal,
            0xB7 => Licensee::Snk,
            0xB9 => Licensee::PonyCanyon,
            0xBA => Licensee::CultureBrain,
            0xBB => Licensee::Sunsoft,
            0xBD => Licensee::Sony,
            0xBF => Licensee::Sammy,
            0xC0 => Licensee::Taito,
            0xC2 => Licensee::Kemco,
            0xC3 => Licensee::SquareSoft,
            0xC4 => Licensee::TokumaShotenIntermedia,
            0xC5 => Licensee::DataEast,
            0xC6 => Licensee::TonkinHouse,
            0xC8 => Licensee::Koei,
            0xC9 => Licensee::Ufl,
            0xCA => Licensee::Ultra,
            0xCB => Licensee::Vap,
            0xCC => Licensee::Use,
            0xCD => Licensee::Meldac,
            0xCE => Licensee::PonyCanyon,
            0xCF => Licensee::Angel,
            0xD0 => Licensee::Taito,
            0xD1 => Licensee::Sofel,
            0xD2 => Licensee::Quest,
            0xD3 => Licensee::Sigma,
            0xD4 => Licensee::AskKodansha,
            0xD6 => Licensee::NaxatSoft,
            0xD7 => Licensee::CopyaSystems,
            0xD9 => Licensee::Banpresto,
            0xDA => Licensee::Tomy,
            0xDB => Licensee::Ljn,
            0xDD => Licensee::Ncs,
            0xDE => Licensee::Human,
            0xDF => Licensee::Altron,
            0xE0 => Licensee::Jaleco,
            0xE1 => Licensee::Towachiki,
            0xE2 => Licensee::Uutaka,
            0xE3 => Licensee::Varie,
            0xE5 => Licensee::Epoch,
            0xE7 => Licensee::Athena,
            0xE8 => Licensee::Asmik,
            0xE9 => Licensee::Natsume,
            0xEA => Licensee::KingRecords,
            0xEB => Licensee::Atlus,
            0xEC => Licensee::EpicSonyRecords,
            0xEE => Licensee::Igs,
            0xF0 => Licensee::AWave,
            0xF3 => Licensee::ExtremeEntertainment,
            0xFF => Licensee::Ljn,
            0x33 => {
                let new_licensee_code = make_u16!(
                    self.storage[HEADER_NEW_LICENSEE_CODE],
                    self.storage[HEADER_NEW_LICENSEE_CODE + 1]
                );
                match new_licensee_code {
                    0x3030 => Licensee::None,
                    0x3031 => Licensee::Nintendo,
                    0x3038 => Licensee::Capcom,
                    0x3133 => Licensee::ElectronicArts,
                    0x3138 => Licensee::HudsonSoft,
                    0x3139 => Licensee::BAi,
                    0x3230 => Licensee::Kss,
                    0x3232 => Licensee::Pow,
                    0x3234 => Licensee::PcmComplete,
                    0x3235 => Licensee::SanX,
                    0x3238 => Licensee::KemcoJapan,
                    0x3239 => Licensee::Seta,
                    0x3330 => Licensee::Viacom,
                    0x3331 => Licensee::Nintendo,
                    0x3332 => Licensee::Bandai,
                    0x3333 => Licensee::OceanAcclaim,
                    0x3334 => Licensee::Konami,
                    0x3335 => Licensee::Hector,
                    0x3337 => Licensee::Taito,
                    0x3338 => Licensee::Hudson,
                    0x3339 => Licensee::Banpresto,
                    0x3431 => Licensee::Ubisoft,
                    0x3432 => Licensee::Atlus,
                    0x3434 => Licensee::Malibu,
                    0x3436 => Licensee::Angel,
                    0x3437 => Licensee::BulletProof,
                    0x3439 => Licensee::Irem,
                    0x3530 => Licensee::Absolute,
                    0x3531 => Licensee::Acclaim,
                    0x3532 => Licensee::Activision,
                    0x3533 => Licensee::AmericanSammy,
                    0x3534 => Licensee::Konami,
                    0x3535 => Licensee::HitechEntertainment,
                    0x3536 => Licensee::Ljn,
                    0x3537 => Licensee::Matchbox,
                    0x3538 => Licensee::Mattel,
                    0x3539 => Licensee::MiltonBradley,
                    0x3630 => Licensee::Titus,
                    0x3631 => Licensee::Virgin,
                    0x3634 => Licensee::LucasArts,
                    0x3637 => Licensee::Ocean,
                    0x3639 => Licensee::ElectronicArts,
                    0x3730 => Licensee::Infogrames,
                    0x3731 => Licensee::Interplay,
                    0x3732 => Licensee::Broderbund,
                    0x3733 => Licensee::Sculptured,
                    0x3735 => Licensee::Sci,
                    0x3738 => Licensee::Thq,
                    0x3739 => Licensee::Accolade,
                    0x3830 => Licensee::Misawa,
                    0x3833 => Licensee::Lozc,
                    0x3836 => Licensee::TokumaShotenIntermedia,
                    0x3837 => Licensee::TsukudaOriginal,
                    0x3931 => Licensee::Chunsoft,
                    0x3932 => Licensee::VideoSystem,
                    0x3933 => Licensee::OceanAcclaim,
                    0x3935 => Licensee::Varie,
                    0x3936 => Licensee::YonezawaSpal,
                    0x3937 => Licensee::Kaneko,
                    0x3939 => Licensee::PackInSoft,
                    0x4134 => Licensee::Konami,
                    _ => Licensee::Unknown,
                }
            },
            _ => Licensee::Unknown,
        }
    }
}

impl<T: Deref<Target=[u8]>> MemoryRegion for Rom<T> {
    fn read(&self, address: u16) -> u8 {
        self.mbc_ctrl.read(&self.storage, address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.mbc_ctrl.write(address, value)
    }
}

#[cfg(debug_assertions)]
impl<T: Deref<Target=[u8]>> fmt::Debug for Rom<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ROM \n\
                   ---\n\
                   Cartridge type: {:?}\n\
                   Licensee: {:?}\n\
                   Size: {} | Ram size: {}\n\
                   CGB: {:?} | SGB: {} | Japanese: {}\n\
                   Version: {}\n\
                   Checksum: {}\n\
                   ",
               self.cartridge_type(), self.licensee(), self.size(), self.ram_size(),
               self.cgb_mode(), self.is_sgb(), self.is_jp(), self.version(),
               self.verify_header_checksum(),
        )
    }
}
