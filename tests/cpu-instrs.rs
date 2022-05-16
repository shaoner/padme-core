use std::fs;
use padme_core::*;
use padme_core::default::{NoScreen, NoSpeaker};

struct SerialBuffer {
    pub data: String,
}

impl SerialOutput for SerialBuffer {
    fn putchar(&mut self, c: u8) {
        self.data += &String::from(c as char);
    }
}

fn get_bin(name: &str) -> Vec<u8> {
    fs::read(format!("tests/roms/cpu_instrs/{}.gb", name)).unwrap()
}

fn check_output(bin_name: &str, max_ticks: usize) -> bool {
    let bin = get_bin(bin_name);
    let rom = Rom::load(bin).unwrap();
    let mut emu = System::new(rom, NoScreen, SerialBuffer { data: "".to_owned() }, NoSpeaker);
    let mut ticks: usize = 0;

    loop {
        ticks += emu.step() as usize;
        if ticks >= max_ticks {
            break;
        }
    }

    return emu.serial().data.contains(&format!("{}\n\n\nPassed", bin_name));
}

#[test]
#[ignore]
fn cpu_instrs_special() {
    assert!(check_output("01-special", 9747268));
}

#[test]
#[ignore]
fn cpu_instrs_interrupts() {
    assert!(check_output("02-interrupts", 1741744));
}

#[test]
#[ignore]
fn cpu_instrs_op_sp_hl() {
    assert!(check_output("03-op sp,hl", 9747028));
}

#[test]
#[ignore]
fn cpu_instrs_op_r_imm() {
    assert!(check_output("04-op r,imm", 11432356));
}

#[test]
#[ignore]
fn cpu_instrs_op_rp() {
    assert!(check_output("05-op rp", 15646524));
}

#[test]
#[ignore]
fn cpu_instrs_ld_r_r() {
    assert!(check_output("06-ld r,r", 2373928));
}

#[test]
#[ignore]
fn cpu_instrs_jr_jp_call_ret_rst() {
    assert!(check_output("07-jr,jp,call,ret,rst", 2935484));
}

#[test]
#[ignore]
fn cpu_instrs_misc() {
    assert!(check_output("08-misc instrs", 2233692));
}

#[test]
#[ignore]
fn cpu_instrs_op_r_r() {
    assert!(check_output("09-op r,r", 38117856));
}

#[test]
#[ignore]
fn cpu_instrs_bitops() {
    assert!(check_output("10-bit ops", 57921392));
}

#[test]
#[ignore]
fn cpu_instrs_op_a_hl() {
    assert!(check_output("11-op a,(hl)", 73370352));
}
