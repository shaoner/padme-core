use std::fs;

use padme_core::*;

static TEST_ROM_1: &str = "cpu_instrs";

fn get_rom_bin(name: &str) -> Vec<u8> {
    fs::read(format!("tests/roms/{}.gb", name)).unwrap()
}

#[test]
fn it_loads_checks_rom_title() {
    let bin = get_rom_bin(TEST_ROM_1);
    let rom = Rom::load(bin).unwrap();

    assert_eq!(rom.title().unwrap(), "CPU_INSTRS");
}

#[test]
fn it_checks_rom_cgb_mode() {
    let bin = get_rom_bin(TEST_ROM_1);
    let rom = Rom::load(bin).unwrap();

    assert_eq!(rom.cgb_mode(), CgbMode::Both);
}

#[test]
fn it_loads_checks_rom_sgb() {
    let bin = get_rom_bin(TEST_ROM_1);
    let rom = Rom::load(bin).unwrap();

    assert_eq!(rom.is_sgb(), false);
}

#[test]
fn it_checks_rom_cartridge_type() {
    let bin = get_rom_bin(TEST_ROM_1);
    let rom = Rom::load(bin).unwrap();

    assert_eq!(rom.cartridge_type(), CartridgeType::Mbc1);
}

#[test]
fn it_checks_rom_size() {
    let bin = get_rom_bin(TEST_ROM_1);
    let len = bin.len();
    let rom = Rom::load(bin).unwrap();

    assert_eq!((rom.size() as usize) * 1024, len);
}

#[test]
fn it_checks_rom_ram_size() {
    let bin = get_rom_bin(TEST_ROM_1);
    let rom = Rom::load(bin).unwrap();

    assert_eq!(rom.ram_size(), 0u16);
}

#[test]
fn it_checks_rom_japanese_mode() {
    let bin = get_rom_bin(TEST_ROM_1);
    let rom = Rom::load(bin).unwrap();

    assert_eq!(rom.is_jp(), true);
}

#[test]
fn it_checks_rom_licensee() {
    let bin = get_rom_bin(TEST_ROM_1);
    let rom = Rom::load(bin).unwrap();

    assert_eq!(rom.licensee(), Licensee::None);
}

#[test]
fn it_checks_rom_version() {
    let bin = get_rom_bin(TEST_ROM_1);
    let rom = Rom::load(bin).unwrap();

    assert_eq!(rom.version(), 0u8);
}

#[test]
fn it_checks_rom_checksum() {
    let bin = get_rom_bin(TEST_ROM_1);
    let rom = Rom::load(bin).unwrap();

    assert_eq!(rom.verify_header_checksum(), true);
}
