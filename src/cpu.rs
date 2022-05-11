use core::ops::Deref;

use log::error;
#[cfg(debug_assertions)]
use log::trace;

use crate::bus::Bus;
use crate::interrupt::InterruptFlag;
use crate::region::*;

pub const CLOCK_SPEED: u32              = 4194304;

// Vector table
const IR_VBLANK_ADDR: u16               = 0x0040;
const IR_LCDC_STATUS_ADDR: u16          = 0x0048;
const IR_TIMER_OVERFLOW_ADDR: u16       = 0x0050;
const IR_SERIAL_TRANSFER_ADDR: u16      = 0x0058;
const IR_JOYPAD_PRESS_ADDR: u16         = 0x0060;

// Flags for register F
const FLAG_ZERO: u8                     = 0x80;
const FLAG_SUBSTRACT: u8                = 0x40;
const FLAG_HALF_CARRY: u8               = 0x20;
const FLAG_CARRY: u8                    = 0x10;

// Default register values
const DEFAULT_REG_A: u8                 = 0x01;
const DEFAULT_REG_F: u8                 = 0xB0;
const DEFAULT_REG_B: u8                 = 0x00;
const DEFAULT_REG_C: u8                 = 0x13;
const DEFAULT_REG_D: u8                 = 0x00;
const DEFAULT_REG_E: u8                 = 0xD8;
const DEFAULT_REG_H: u8                 = 0x01;
const DEFAULT_REG_L: u8                 = 0x4D;

const DEFAULT_SP: u16                   = 0xFFFE;
const DEFAULT_PC: u16                   = 0x0100;

macro_rules! fmt_registers {
    ($pc: expr, $sp: expr, $af: expr, $bc: expr, $de: expr, $hl: expr) => {
        format_args!("PC: 0x{:04X} | SP: 0x{:04X} | \
                      AF: 0x{:04X} ({}, {}, {}, {}) | BC: 0x{:04X} | \
                      DE: 0x{:04X} | HL: 0x{:04X}",
                     $pc, $sp, $af,
                     if ($af as u8 & FLAG_ZERO) == 0 { "-" } else { "Z" },
                     if ($af as u8 & FLAG_SUBSTRACT) == 0 { "-" } else { "N" },
                     if ($af as u8 & FLAG_HALF_CARRY) == 0 { "-" } else { "H" },
                     if ($af as u8 & FLAG_CARRY) == 0 { "-" } else { "C" },
                     $bc, $de, $hl)
    }
}

pub struct Cpu {
    // Registers
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    pc: u16,
    sp: u16,
    // CPU halted
    halted: bool,
    // CPU stopped until button is pressed
    stopped: bool,
    // Master Interrupt Enable
    master_ie: bool,
    enabling_ie: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            a: DEFAULT_REG_A,
            f: DEFAULT_REG_F,
            b: DEFAULT_REG_B,
            c: DEFAULT_REG_C,
            d: DEFAULT_REG_D,
            e: DEFAULT_REG_E,
            h: DEFAULT_REG_H,
            l: DEFAULT_REG_L,
            sp: DEFAULT_SP,
            pc: DEFAULT_PC,
            halted: false,
            stopped: false,
            master_ie: true,
            enabling_ie: false,
        }
    }

    fn af(&self) -> u16 {
        make_u16!(self.a, self.f)
    }

    fn bc(&self) -> u16 {
        make_u16!(self.b, self.c)
    }

    fn de(&self) -> u16 {
        make_u16!(self.d, self.e)
    }

    fn hl(&self) -> u16 {
        make_u16!(self.h, self.l)
    }

    fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = value as u8;
    }

    fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }

    fn inc_hl(&mut self) {
        let hl = self.hl();
        self.set_hl(hl.wrapping_add(1));
    }

    fn dec_hl(&mut self) {
        let hl = self.hl();
        self.set_hl(hl.wrapping_sub(1));
    }

    /// Set or reset flag in F register
    fn set_flag(&mut self, flag: u8, set: bool) {
        if set {
            self.f |= flag;
        } else {
            self.f &= !flag;
        }
    }

    /// Retrieve next byte
    fn fetch<T: Deref<Target=[u8]>>(&mut self, bus: &Bus<T>) -> u8 {
        let byte = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    /// Retrieve next 2 bytes as a u16
    fn fetch16<T: Deref<Target=[u8]>>(&mut self, bus: &Bus<T>) -> u16 {
        let l = self.fetch(bus);
        let h = self.fetch(bus);
        make_u16!(h, l)
    }

    /// Put SP + n into HL
    fn ld_hl_spn<T: Deref<Target=[u8]>>(&mut self, bus: &Bus<T>) {
        let n = self.fetch(bus);
        let res = (self.sp as i32).wrapping_add((n as i8) as i32) as u16;

        self.set_flag(FLAG_ZERO, false);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_CARRY, (res & 0xff) < (self.sp & 0xff));
        self.set_flag(FLAG_HALF_CARRY, (res & 0xf) < (self.sp & 0xf));
        self.set_hl(res);
    }

    /// PUSH element on top of the stack
    fn push<T: Deref<Target=[u8]>>(&mut self, bus: &mut Bus<T>, value: u16) {
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, (value >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        bus.write(self.sp, value as u8);
    }

    /// POP top element of the stack
    fn pop<T: Deref<Target=[u8]>>(&mut self, bus: &Bus<T>) -> u16 {
        let l = bus.read(self.sp);
        self.sp = self.sp.wrapping_add(1);
        let h = bus.read(self.sp);
        self.sp = self.sp.wrapping_add(1);

        ((h as u16) << 8) | l as u16
    }

    /// Add value to register A with provided carry
    fn addc(&mut self, value: u8, carry: u8) {
        let r = (self.a as u16) + (value as u16) + (carry as u16);
        self.set_flag(FLAG_CARRY, r > 0xff);
        let r = r as u8;
        self.set_flag(FLAG_ZERO, r == 0);
        self.set_flag(FLAG_HALF_CARRY, (self.a & 0xF) + (value & 0xF) + carry > 0xF);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.a = r;
    }

    /// Add value to register A, no carry
    fn add(&mut self, value: u8) {
        self.addc(value, 0u8)
    }

    /// Add value to register A using carry flag
    fn adc(&mut self, value: u8) {
        self.addc(value, (self.f & FLAG_CARRY == FLAG_CARRY) as u8)
    }

    /// Subtract value to register A with provided carry
    fn subc(&mut self, value: u8, carry: u8) -> u8 {
        let n = (self.a as u16)
            .wrapping_sub(value as u16)
            .wrapping_sub(carry as u16);

        self.set_flag(FLAG_CARRY, n > 0xff);
        let n = n as u8;
        self.set_flag(FLAG_ZERO, n == 0);
        let hc = ((self.a & 0xF) as i16 - (value & 0xF) as i16 - carry as i16) < 0x0;
        self.set_flag(FLAG_HALF_CARRY, hc);
        self.set_flag(FLAG_SUBSTRACT, true);
        n
    }

    /// Subtract value to register A no carry
    fn sub(&mut self, value: u8) {
        self.a = self.subc(value, 0u8)
    }

    /// Subtract value to register A using carry flag
    fn sbc(&mut self, value: u8) {
        self.a = self.subc(value, (self.f & FLAG_CARRY != 0) as u8)
    }

    /// Logical AND value with register A
    fn and(&mut self, value: u8) {
        self.a &= value;
        self.set_flag(FLAG_ZERO, self.a == 0);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_HALF_CARRY, true);
        self.set_flag(FLAG_CARRY, false);
    }

    /// Logical OR value with register A
    fn or(&mut self, value: u8) {
        self.a |= value;
        self.set_flag(FLAG_ZERO, self.a == 0);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_HALF_CARRY, false);
        self.set_flag(FLAG_CARRY, false);
    }

    /// XOR value with register A
    fn xor(&mut self, value: u8) {
        self.a ^= value;
        self.set_flag(FLAG_ZERO, self.a == 0);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_HALF_CARRY, false);
        self.set_flag(FLAG_CARRY, false);
    }

    /// Compare A with value
    fn cp(&mut self, value: u8) {
        self.subc(value, 0); // discard result
    }

    /// Increment a register value
    fn inc(&mut self, value: u8) -> u8 {
        let r = value.wrapping_add(1);
        self.set_flag(FLAG_ZERO, r == 0);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_HALF_CARRY, (r & 0xf) < (value & 0xf));
        r
    }

    /// Decrement a register value
    fn dec(&mut self, value: u8) -> u8 {
        let r = value.wrapping_sub(1);
        self.set_flag(FLAG_ZERO, r == 0);
        self.set_flag(FLAG_SUBSTRACT, true);
        self.set_flag(FLAG_HALF_CARRY, (r & 0xf) > (value & 0xf));
        r
    }

    /// Add value to HL register
    fn add16(&mut self, n: u16) {
        let hl = self.hl();
        let result = hl.wrapping_add(n);
        self.set_flag(FLAG_CARRY, hl > 0xFFFF - n);
        self.set_flag(FLAG_HALF_CARRY, (((hl & 0xFFF) + (n & 0xFFF)) & 0x1000) != 0);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_hl(result);
    }

    /// Swap upper & lower nibbles of value
    fn swap(&mut self, value: u8) -> u8 {
        let r = (value << 4) | (value >> 4);
        self.set_flag(FLAG_ZERO, r == 0);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_CARRY, false);
        self.set_flag(FLAG_HALF_CARRY, false);

        r
    }

    /// Decimal adjust register A
    /// Adjust A to get a BCD representation of A
    fn daa(&mut self) {
        let mut adj = 0x00;
        let is_sub = (self.f & FLAG_SUBSTRACT) == FLAG_SUBSTRACT;
        let mut is_carry = false;

        if (self.f & FLAG_HALF_CARRY) != 0 || (!is_sub && (self.a & 0x0F) > 0x09) {
            adj |= 0x06;
        }
        if (self.f & FLAG_CARRY) != 0 || (!is_sub && self.a > 0x99) {
            adj |= 0x60;
            is_carry = true;
        }

        if is_sub {
            self.a = self.a.wrapping_sub(adj);
        } else {
            self.a = self.a.wrapping_add(adj);
        }
        self.set_flag(FLAG_ZERO, self.a == 0);
        self.set_flag(FLAG_HALF_CARRY, false);
        self.set_flag(FLAG_CARRY, is_carry);
    }

    /// Complement
    /// Flip all bits of register A
    fn cpl(&mut self) {
        self.a = !self.a;
        self.set_flag(FLAG_SUBSTRACT, true);
        self.set_flag(FLAG_HALF_CARRY, true);
    }

    /// Complement carry flag
    fn ccf(&mut self) {
        self.f ^= FLAG_CARRY;
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_HALF_CARRY, false);
    }

    /// Set carry flag
    fn scf(&mut self) {
        self.set_flag(FLAG_CARRY, true);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_HALF_CARRY, false);
    }

    /// Rotate A left with old bit 7 to carry flag
    fn rl(&mut self, n: u8, with_carry: bool, set_zero: bool) -> u8 {
        let res = if with_carry {
            let carry_bit = ((self.f & FLAG_CARRY) == FLAG_CARRY) as u8;
            (n << 1) | carry_bit
        } else {
            n.rotate_left(1)
        };
        self.set_flag(FLAG_CARRY, (n >> 7) == 0x01);
        self.set_flag(FLAG_ZERO, set_zero && res == 0);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_HALF_CARRY, false);
        res
    }

    /// Rotate A right with old bit 7 to carry flag
    fn rr(&mut self, n: u8, with_carry: bool, set_zero: bool) -> u8 {
        let res = if with_carry {
            let carry_bit = ((self.f & FLAG_CARRY) == FLAG_CARRY) as u8;
            (carry_bit << 7) | (n >> 1)
        } else {
            n.rotate_right(1)
        };
        self.set_flag(FLAG_CARRY, (n & 0x01) == 0x01);
        self.set_flag(FLAG_ZERO, set_zero && res == 0);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_HALF_CARRY, false);
        res
    }

    /// Shift n left into carry
    fn sl(&mut self, n: u8) -> u8 {
        let res = n << 1;
        self.set_flag(FLAG_CARRY, (n >> 7) == 0x01);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_HALF_CARRY, false);
        self.set_flag(FLAG_ZERO, res == 0);
        res
    }

    /// Shift n right into carry
    fn sr(&mut self, n: u8, keep_msb: bool) -> u8 {
        let res = if keep_msb { (n & 0x80) | (n >> 1) } else { n >> 1 };
        self.set_flag(FLAG_CARRY, (n & 0x01) == 0x01);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_HALF_CARRY, false);
        self.set_flag(FLAG_ZERO, res == 0);
        res
    }

    /// Test if bit b is set in n
    fn bit(&mut self, n: u8, b: u8) {
        self.set_flag(FLAG_ZERO, (n & b) == 0);
        self.set_flag(FLAG_SUBSTRACT, false);
        self.set_flag(FLAG_HALF_CARRY, true);
    }

    /// Jump to address if flag is set / reset
    fn jump_if(&mut self, address: u16, condition: bool) -> u8 {
        if condition {
            self.pc = address;
            16
        } else {
            12
        }
    }

    /// Jump to pc + n if flag is set / reset
    fn jump_if_rel(&mut self, n: u8, condition: bool) -> u8 {
        if condition {
            self.pc = ((self.pc as i32) + ((n as i8) as i32)) as u16;
            12
        } else {
            8
        }
    }

    /// Save PC and jump to address
    fn call<T: Deref<Target=[u8]>>(&mut self, bus: &mut Bus<T>, address: u16) {
        self.push(bus, self.pc);
        self.pc = address;
    }

    /// Save PC and jump to address if condition is true
    fn call_if<T: Deref<Target=[u8]>>(&mut self, bus: &mut Bus<T>, nn: u16, condition: bool) -> u8 {
        if condition {
            self.call(bus, nn);
            24
        } else {
            12
        }
    }

    /// Return if condition is true
    fn ret_if<T: Deref<Target=[u8]>>(&mut self, bus: &Bus<T>, condition: bool) -> u8 {
        if condition {
            self.pc = self.pop(bus);
            20
        } else {
            8
        }
    }

    #[cfg(debug_assertions)]
    fn dump_instruction<T: Deref<Target=[u8]>>(&mut self, bus: &Bus<T>, op: u8) {
        macro_rules! trace_instruction {
            ($($arg:tt)*) => {
                trace!("{} | {}", fmt_registers!(self.pc.wrapping_sub(1), self.sp, self.af(),
                                                 self.bc(), self.de(), self.hl()),
                           format_args!($($arg)*))
            };
        }

        macro_rules! rel_address {
            ($n: expr) => {
                ((self.pc as i32 + 1) + (($n as i8) as i32)) as u16
            }
        }

        let next = bus.read(self.pc);
        let next16 = {
            let l = bus.read(self.pc);
            let h = bus.read(self.pc.wrapping_add(1));
            make_u16!(h, l)
        };

        match op {
            0x00 => { trace_instruction!("NOP") },
            0x27 => { trace_instruction!("DAA") },
            0x2F => { trace_instruction!("CPL") },
            0x37 => { trace_instruction!("SCF") },
            0x3F => { trace_instruction!("CCF") },
            0x76 => { trace_instruction!("HALT") },
            0x10 => { trace_instruction!("STOP") },
            0x01 => { trace_instruction!("LD BC, ${:04X}", next16) },
            0x11 => { trace_instruction!("LD DE, ${:04X}", next16) },
            0x21 => { trace_instruction!("LD HL, ${:04X}", next16) },
            0x31 => { trace_instruction!("LD SP, ${:04X}", next16) },
            0x06 => { trace_instruction!("LD B, ${:02X}", next) },
            0x0E => { trace_instruction!("LD C, ${:02X}", next) },
            0x16 => { trace_instruction!("LD D, ${:02X}", next) },
            0x1E => { trace_instruction!("LD E, ${:02X}", next) },
            0x26 => { trace_instruction!("LD H, ${:02X}", next) },
            0x2E => { trace_instruction!("LD L, ${:02X}", next) },
            0x3E => { trace_instruction!("LD A, ${:02X}", next) },
            0x40 => { trace_instruction!("LD B, B") },
            0x41 => { trace_instruction!("LD B, C") },
            0x42 => { trace_instruction!("LD B, D") },
            0x43 => { trace_instruction!("LD B, E") },
            0x44 => { trace_instruction!("LD B, H") },
            0x45 => { trace_instruction!("LD B, L") },
            0x46 => { trace_instruction!("LD B, (HL)") },
            0x47 => { trace_instruction!("LD B, A") },
            0x48 => { trace_instruction!("LD C, B") },
            0x49 => { trace_instruction!("LD C, C") },
            0x4A => { trace_instruction!("LD C, D") },
            0x4B => { trace_instruction!("LD C, E") },
            0x4C => { trace_instruction!("LD C, H") },
            0x4D => { trace_instruction!("LD C, L") },
            0x4E => { trace_instruction!("LD C, (HL)") },
            0x4F => { trace_instruction!("LD C, A") },
            0x50 => { trace_instruction!("LD D, B") },
            0x51 => { trace_instruction!("LD D, C") },
            0x52 => { trace_instruction!("LD D, D") },
            0x53 => { trace_instruction!("LD D, E") },
            0x54 => { trace_instruction!("LD D, H") },
            0x55 => { trace_instruction!("LD D, L") },
            0x56 => { trace_instruction!("LD D, (HL)") },
            0x57 => { trace_instruction!("LD D, A") },
            0x58 => { trace_instruction!("LD E, B") },
            0x59 => { trace_instruction!("LD E, C") },
            0x5A => { trace_instruction!("LD E, D") },
            0x5B => { trace_instruction!("LD E, E") },
            0x5C => { trace_instruction!("LD E, H") },
            0x5D => { trace_instruction!("LD E, L") },
            0x5E => { trace_instruction!("LD E, (HL)") },
            0x5F => { trace_instruction!("LD E, A") },
            0x60 => { trace_instruction!("LD H, B") },
            0x61 => { trace_instruction!("LD H, C") },
            0x62 => { trace_instruction!("LD H, D") },
            0x63 => { trace_instruction!("LD H, E") },
            0x64 => { trace_instruction!("LD H, H") },
            0x65 => { trace_instruction!("LD H, L") },
            0x66 => { trace_instruction!("LD B, (HL)") },
            0x67 => { trace_instruction!("LD H, A") },
            0x68 => { trace_instruction!("LD L, B") },
            0x69 => { trace_instruction!("LD L, C") },
            0x6A => { trace_instruction!("LD L, D") },
            0x6B => { trace_instruction!("LD L, E") },
            0x6C => { trace_instruction!("LD L, H") },
            0x6D => { trace_instruction!("LD L, L") },
            0x6E => { trace_instruction!("LD L, (HL)") },
            0x6F => { trace_instruction!("LD L, A") },
            0x78 => { trace_instruction!("LD A, B") },
            0x79 => { trace_instruction!("LD A, C") },
            0x7A => { trace_instruction!("LD A, D") },
            0x7B => { trace_instruction!("LD A, E") },
            0x7C => { trace_instruction!("LD A, H") },
            0x7D => { trace_instruction!("LD A, L") },
            0x7E => { trace_instruction!("LD A, (HL)") },
            0x7F => { trace_instruction!("LD A, A") },
            0x2A => { trace_instruction!("LD A, (HL+)") },
            0x3A => { trace_instruction!("LD A, (HL-)") },
            0x0A => { trace_instruction!("LD A, (BC)") },
            0x1A => { trace_instruction!("LD A, (DE)") },
            0xFA => { trace_instruction!("LD A, (${:04X})", next16) },
            0xEA => { trace_instruction!("LD (${:04X}), A", next16) },
            0x36 => { trace_instruction!("LD (HL), ${:02X}", next) },
            0x70 => { trace_instruction!("LD (HL), B") },
            0x71 => { trace_instruction!("LD (HL), C") },
            0x72 => { trace_instruction!("LD (HL), D") },
            0x73 => { trace_instruction!("LD (HL), E") },
            0x74 => { trace_instruction!("LD (HL), H") },
            0x75 => { trace_instruction!("LD (HL), L") },
            0x77 => { trace_instruction!("LD (HL), A") },
            0x02 => { trace_instruction!("LD (BC), A") },
            0x12 => { trace_instruction!("LD (DE), A") },
            0x22 => { trace_instruction!("LD (HL+), A") },
            0x32 => { trace_instruction!("LD (HL-), A") },
            0xE0 => { trace_instruction!("LD ($FF00 + ${:02X}), A", next) },
            0xF0 => { trace_instruction!("LD A, ($FF00 + ${:02X})", next) },
            0xE2 => { trace_instruction!("LD ($FF00 + C), A") },
            0xF2 => { trace_instruction!("LD A, ($FF00 + C)") },
            0xF8 => { trace_instruction!("LD HL, SP + ${:02X}", next) },
            0x08 => { trace_instruction!("LD (${:04X}), SP", next16) },
            0xF9 => { trace_instruction!("LD SP, HL") },
            0xF5 => { trace_instruction!("PUSH AF") },
            0xC5 => { trace_instruction!("PUSH BC") },
            0xD5 => { trace_instruction!("PUSH DE") },
            0xE5 => { trace_instruction!("PUSH HL") },
            0xF1 => { trace_instruction!("POP AF") },
            0xC1 => { trace_instruction!("POP BC") },
            0xD1 => { trace_instruction!("POP DE") },
            0xE1 => { trace_instruction!("POP HL") },
            0xC3 => { trace_instruction!("JP ${:04X}", next16) },
            0xC2 => { trace_instruction!("JP NZ, ${:04X}", next16) },
            0xCA => { trace_instruction!("JP Z, ${:04X}", next16) },
            0xD2 => { trace_instruction!("JP NC, ${:04X}", next16) },
            0xDA => { trace_instruction!("JP C, ${:04X}", next16) },
            0xE9 => { trace_instruction!("JP (HL)") },
            0x18 => { trace_instruction!("JR ${:2X}", rel_address!(next)) },
            0x20 => { trace_instruction!("JR NZ, ${:02X}", rel_address!(next)) },
            0x28 => { trace_instruction!("JR Z, ${:02X}", rel_address!(next)) },
            0x30 => { trace_instruction!("JR NC, ${:02X}", rel_address!(next)) },
            0x38 => { trace_instruction!("JR C, ${:02X}", rel_address!(next)) },
            0xCD => { trace_instruction!("CALL ${:04X}", next16) },
            0xC4 => { trace_instruction!("CALL NZ, ${:04X}", next16) },
            0xCC => { trace_instruction!("CALL Z, ${:04X}", next16) },
            0xD4 => { trace_instruction!("CALL NC, ${:04X}", next16) },
            0xDC => { trace_instruction!("CALL C, ${:04X}", next16) },
            0xC7 => { trace_instruction!("RST ${:04X}", 0x00u16) },
            0xCF => { trace_instruction!("RST ${:04X}", 0x08u16) },
            0xD7 => { trace_instruction!("RST ${:04X}", 0x10u16) },
            0xDF => { trace_instruction!("RST ${:04X}", 0x18u16) },
            0xE7 => { trace_instruction!("RST ${:04X}", 0x20u16) },
            0xEF => { trace_instruction!("RST ${:04X}", 0x28u16) },
            0xF7 => { trace_instruction!("RST ${:04X}", 0x30u16) },
            0xFF => { trace_instruction!("RST ${:04X}", 0x38u16) },
            0xC9 => { trace_instruction!("RET") },
            0xC0 => { trace_instruction!("RET NZ") },
            0xC8 => { trace_instruction!("RET Z") },
            0xD0 => { trace_instruction!("RET NC") },
            0xD8 => { trace_instruction!("RET C") },
            0xD9 => { trace_instruction!("RETI") }
            0x87 => { trace_instruction!("ADD A, A") },
            0x80 => { trace_instruction!("ADD A, B") },
            0x81 => { trace_instruction!("ADD A, C") },
            0x82 => { trace_instruction!("ADD A, D") },
            0x83 => { trace_instruction!("ADD A, E") },
            0x84 => { trace_instruction!("ADD A, H") },
            0x85 => { trace_instruction!("ADD A, L") },
            0x86 => { trace_instruction!("ADD A, (HL)") },
            0xC6 => { trace_instruction!("ADD A, ${:02X}", next) },
            0x8F => { trace_instruction!("ADC A, A") },
            0x88 => { trace_instruction!("ADC A, B") },
            0x89 => { trace_instruction!("ADC A, C") },
            0x8A => { trace_instruction!("ADC A, D") },
            0x8B => { trace_instruction!("ADC A, E") },
            0x8C => { trace_instruction!("ADC A, H") },
            0x8D => { trace_instruction!("ADC A, L") },
            0x8E => { trace_instruction!("ADC A, (HL)") },
            0xCE => { trace_instruction!("ADC A, ${:02X}", next) },
            0x97 => { trace_instruction!("SUB A, A") },
            0x90 => { trace_instruction!("SUB A, B") },
            0x91 => { trace_instruction!("SUB A, C") },
            0x92 => { trace_instruction!("SUB A, D") },
            0x93 => { trace_instruction!("SUB A, E") },
            0x94 => { trace_instruction!("SUB A, H") },
            0x95 => { trace_instruction!("SUB A, L") },
            0x96 => { trace_instruction!("SUB A, (HL)") },
            0xD6 => { trace_instruction!("SUB A, ${:02X}", next) },
            0x9F => { trace_instruction!("SBC A, A") },
            0x98 => { trace_instruction!("SBC A, B") },
            0x99 => { trace_instruction!("SBC A, C") },
            0x9A => { trace_instruction!("SBC A, D") },
            0x9B => { trace_instruction!("SBC A, E") },
            0x9C => { trace_instruction!("SBC A, H") },
            0x9D => { trace_instruction!("SBC A, L") },
            0x9E => { trace_instruction!("SBC A, (HL)") },
            0xDE => { trace_instruction!("SBC A, ${:02X}", next) },
            0xA7 => { trace_instruction!("AND A") },
            0xA0 => { trace_instruction!("AND B") },
            0xA1 => { trace_instruction!("AND C") },
            0xA2 => { trace_instruction!("AND D") },
            0xA3 => { trace_instruction!("AND E") },
            0xA4 => { trace_instruction!("AND H") },
            0xA5 => { trace_instruction!("AND L") },
            0xA6 => { trace_instruction!("AND (HL)") },
            0xE6 => { trace_instruction!("AND ${:02X}", next) },
            0xB7 => { trace_instruction!("OR A") },
            0xB0 => { trace_instruction!("OR B") },
            0xB1 => { trace_instruction!("OR C") },
            0xB2 => { trace_instruction!("OR D") },
            0xB3 => { trace_instruction!("OR E") },
            0xB4 => { trace_instruction!("OR H") },
            0xB5 => { trace_instruction!("OR L") },
            0xB6 => { trace_instruction!("OR (HL)") },
            0xF6 => { trace_instruction!("OR ${:02X}", next) },
            0xAF => { trace_instruction!("XOR A") },
            0xA8 => { trace_instruction!("XOR B") },
            0xA9 => { trace_instruction!("XOR C") },
            0xAA => { trace_instruction!("XOR D") },
            0xAB => { trace_instruction!("XOR E") },
            0xAC => { trace_instruction!("XOR H") },
            0xAD => { trace_instruction!("XOR L") },
            0xAE => { trace_instruction!("XOR (HL)") },
            0xEE => { trace_instruction!("XOR ${:02X}", next) },
            0xBF => { trace_instruction!("CP A") },
            0xB8 => { trace_instruction!("CP B") },
            0xB9 => { trace_instruction!("CP C") },
            0xBA => { trace_instruction!("CP D") },
            0xBB => { trace_instruction!("CP E") },
            0xBC => { trace_instruction!("CP H") },
            0xBD => { trace_instruction!("CP L") },
            0xBE => { trace_instruction!("CP (HL)") },
            0xFE => { trace_instruction!("CP ${:02X}", next) },
            0x3C => { trace_instruction!("INC A") },
            0x04 => { trace_instruction!("INC B") },
            0x0C => { trace_instruction!("INC C") },
            0x14 => { trace_instruction!("INC D") },
            0x1C => { trace_instruction!("INC E") },
            0x24 => { trace_instruction!("INC H") },
            0x2C => { trace_instruction!("INC L") },
            0x34 => { trace_instruction!("INC (HL)") },
            0x3D => { trace_instruction!("DEC A") },
            0x05 => { trace_instruction!("DEC B") },
            0x0D => { trace_instruction!("DEC C") },
            0x15 => { trace_instruction!("DEC D") },
            0x1D => { trace_instruction!("DEC E") },
            0x25 => { trace_instruction!("DEC H") },
            0x2D => { trace_instruction!("DEC L") },
            0x35 => { trace_instruction!("DEC (HL)") },
            0x09 => { trace_instruction!("ADD HL, BC") },
            0x19 => { trace_instruction!("ADD HL, DE") },
            0x29 => { trace_instruction!("ADD HL, HL") },
            0x39 => { trace_instruction!("ADD HL, SP") },
            0xE8 => { trace_instruction!("ADD SP, ${:02X}", next as i8) },
            0x03 => { trace_instruction!("INC BC") },
            0x13 => { trace_instruction!("INC DE") },
            0x23 => { trace_instruction!("INC HL") },
            0x33 => { trace_instruction!("INC SP") },
            0x0B => { trace_instruction!("DEC BC") },
            0x1B => { trace_instruction!("DEC DE") },
            0x2B => { trace_instruction!("DEC HL") },
            0x3B => { trace_instruction!("DEC SP") },
            0xF3 => { trace_instruction!("DI") },
            0xFB => { trace_instruction!("EI") },
            0x07 => { trace_instruction!("RLCA") },
            0x17 => { trace_instruction!("RLA") },
            0x0F => { trace_instruction!("RRCA") },
            0x1F => { trace_instruction!("RRA") },
            0xCB => {
                let op2 = next;

                match op2 {
                    0x37 => { trace_instruction!("SWAP A") },
                    0x30 => { trace_instruction!("SWAP B") },
                    0x31 => { trace_instruction!("SWAP C") },
                    0x32 => { trace_instruction!("SWAP D") },
                    0x33 => { trace_instruction!("SWAP E") },
                    0x34 => { trace_instruction!("SWAP H") },
                    0x35 => { trace_instruction!("SWAP L") },
                    0x36 => { trace_instruction!("SWAP (HL)") },
                    0x07 => { trace_instruction!("RLC A") },
                    0x00 => { trace_instruction!("RLC B") },
                    0x01 => { trace_instruction!("RLC C") },
                    0x02 => { trace_instruction!("RLC D") },
                    0x03 => { trace_instruction!("RLC E") },
                    0x04 => { trace_instruction!("RLC H") },
                    0x05 => { trace_instruction!("RLC L") },
                    0x06 => { trace_instruction!("RLC (HL)") },
                    0x17 => { trace_instruction!("RL A") },
                    0x10 => { trace_instruction!("RL B") },
                    0x11 => { trace_instruction!("RL C") },
                    0x12 => { trace_instruction!("RL D") },
                    0x13 => { trace_instruction!("RL E") },
                    0x14 => { trace_instruction!("RL H") },
                    0x15 => { trace_instruction!("RL L") },
                    0x16 => { trace_instruction!("RL (HL)") },
                    0x0F => { trace_instruction!("RRC A") },
                    0x08 => { trace_instruction!("RRC B") },
                    0x09 => { trace_instruction!("RRC C") },
                    0x0A => { trace_instruction!("RRC D") },
                    0x0B => { trace_instruction!("RRC E") },
                    0x0C => { trace_instruction!("RRC H") },
                    0x0D => { trace_instruction!("RRC L") },
                    0x0E => { trace_instruction!("RRC (HL)") },
                    0x1F => { trace_instruction!("RR A") },
                    0x18 => { trace_instruction!("RR B") },
                    0x19 => { trace_instruction!("RR C") },
                    0x1A => { trace_instruction!("RR D") },
                    0x1B => { trace_instruction!("RR E") },
                    0x1C => { trace_instruction!("RR H") },
                    0x1D => { trace_instruction!("RR L") },
                    0x1E => { trace_instruction!("RR (HL)") },
                    0x27 => { trace_instruction!("SLA A") },
                    0x20 => { trace_instruction!("SLA B") },
                    0x21 => { trace_instruction!("SLA C") },
                    0x22 => { trace_instruction!("SLA D") },
                    0x23 => { trace_instruction!("SLA E") },
                    0x24 => { trace_instruction!("SLA H") },
                    0x25 => { trace_instruction!("SLA L") },
                    0x26 => { trace_instruction!("SLA (HL)") },
                    0x2F => { trace_instruction!("SRA A") },
                    0x28 => { trace_instruction!("SRA B") },
                    0x29 => { trace_instruction!("SRA C") },
                    0x2A => { trace_instruction!("SRA D") },
                    0x2B => { trace_instruction!("SRA E") },
                    0x2C => { trace_instruction!("SRA H") },
                    0x2D => { trace_instruction!("SRA L") },
                    0x2E => { trace_instruction!("SRA (HL)") },
                    0x3F => { trace_instruction!("SRL A") },
                    0x38 => { trace_instruction!("SRL B") },
                    0x39 => { trace_instruction!("SRL C") },
                    0x3A => { trace_instruction!("SRL D") },
                    0x3B => { trace_instruction!("SRL E") },
                    0x3C => { trace_instruction!("SRL H") },
                    0x3D => { trace_instruction!("SRL L") },
                    0x3E => { trace_instruction!("SRL (HL)") },
                    0x47 => { trace_instruction!("BIT 0, A") },
                    0x40 => { trace_instruction!("BIT 0, B") },
                    0x41 => { trace_instruction!("BIT 0, C") },
                    0x42 => { trace_instruction!("BIT 0, D") },
                    0x43 => { trace_instruction!("BIT 0, E") },
                    0x44 => { trace_instruction!("BIT 0, H") },
                    0x45 => { trace_instruction!("BIT 0, L") },
                    0x46 => { trace_instruction!("BIT 0, (HL)") },
                    0x4F => { trace_instruction!("BIT 1, A") },
                    0x48 => { trace_instruction!("BIT 1, B") },
                    0x49 => { trace_instruction!("BIT 1, C") },
                    0x4A => { trace_instruction!("BIT 1, D") },
                    0x4B => { trace_instruction!("BIT 1, E") },
                    0x4C => { trace_instruction!("BIT 1, H") },
                    0x4D => { trace_instruction!("BIT 1, L") },
                    0x4E => { trace_instruction!("BIT 1, (HL)") },
                    0x57 => { trace_instruction!("BIT 2, A") },
                    0x50 => { trace_instruction!("BIT 2, B") },
                    0x51 => { trace_instruction!("BIT 2, C") },
                    0x52 => { trace_instruction!("BIT 2, D") },
                    0x53 => { trace_instruction!("BIT 2, E") },
                    0x54 => { trace_instruction!("BIT 2, H") },
                    0x55 => { trace_instruction!("BIT 2, L") },
                    0x56 => { trace_instruction!("BIT 2, (HL)") },
                    0x5F => { trace_instruction!("BIT 3, A") },
                    0x58 => { trace_instruction!("BIT 3, B") },
                    0x59 => { trace_instruction!("BIT 3, C") },
                    0x5A => { trace_instruction!("BIT 3, D") },
                    0x5B => { trace_instruction!("BIT 3, E") },
                    0x5C => { trace_instruction!("BIT 3, H") },
                    0x5D => { trace_instruction!("BIT 3, L") },
                    0x5E => { trace_instruction!("BIT 3, (HL)") },
                    0x67 => { trace_instruction!("BIT 4, A") },
                    0x60 => { trace_instruction!("BIT 4, B") },
                    0x61 => { trace_instruction!("BIT 4, C") },
                    0x62 => { trace_instruction!("BIT 4, D") },
                    0x63 => { trace_instruction!("BIT 4, E") },
                    0x64 => { trace_instruction!("BIT 4, H") },
                    0x65 => { trace_instruction!("BIT 4, L") },
                    0x66 => { trace_instruction!("BIT 4, (HL)") },
                    0x6F => { trace_instruction!("BIT 5, A") },
                    0x68 => { trace_instruction!("BIT 5, B") },
                    0x69 => { trace_instruction!("BIT 5, C") },
                    0x6A => { trace_instruction!("BIT 5, D") },
                    0x6B => { trace_instruction!("BIT 5, E") },
                    0x6C => { trace_instruction!("BIT 5, H") },
                    0x6D => { trace_instruction!("BIT 5, L") },
                    0x6E => { trace_instruction!("BIT 5, (HL)") },
                    0x77 => { trace_instruction!("BIT 6, A") },
                    0x70 => { trace_instruction!("BIT 6, B") },
                    0x71 => { trace_instruction!("BIT 6, C") },
                    0x72 => { trace_instruction!("BIT 6, D") },
                    0x73 => { trace_instruction!("BIT 6, E") },
                    0x74 => { trace_instruction!("BIT 6, H") },
                    0x75 => { trace_instruction!("BIT 6, L") },
                    0x76 => { trace_instruction!("BIT 6, (HL)") },
                    0x7F => { trace_instruction!("BIT 7, A") },
                    0x78 => { trace_instruction!("BIT 7, B") },
                    0x79 => { trace_instruction!("BIT 7, C") },
                    0x7A => { trace_instruction!("BIT 7, D") },
                    0x7B => { trace_instruction!("BIT 7, E") },
                    0x7C => { trace_instruction!("BIT 7, H") },
                    0x7D => { trace_instruction!("BIT 7, L") },
                    0x7E => { trace_instruction!("BIT 7, (HL)") },
                    0x87 => { trace_instruction!("RES 0, A") },
                    0x80 => { trace_instruction!("RES 0, B") },
                    0x81 => { trace_instruction!("RES 0, C") },
                    0x82 => { trace_instruction!("RES 0, D") },
                    0x83 => { trace_instruction!("RES 0, E") },
                    0x84 => { trace_instruction!("RES 0, H") },
                    0x85 => { trace_instruction!("RES 0, L") },
                    0x86 => { trace_instruction!("RES 0, (HL)") },
                    0x8F => { trace_instruction!("RES 1, A") },
                    0x88 => { trace_instruction!("RES 1, B") },
                    0x89 => { trace_instruction!("RES 1, C") },
                    0x8A => { trace_instruction!("RES 1, D") },
                    0x8B => { trace_instruction!("RES 1, E") },
                    0x8C => { trace_instruction!("RES 1, H") },
                    0x8D => { trace_instruction!("RES 1, L") },
                    0x8E => { trace_instruction!("RES 1, (HL)") },
                    0x97 => { trace_instruction!("RES 2, A") },
                    0x90 => { trace_instruction!("RES 2, B") },
                    0x91 => { trace_instruction!("RES 2, C") },
                    0x92 => { trace_instruction!("RES 2, D") },
                    0x93 => { trace_instruction!("RES 2, E") },
                    0x94 => { trace_instruction!("RES 2, H") },
                    0x95 => { trace_instruction!("RES 2, L") },
                    0x96 => { trace_instruction!("RES 2, (HL)") },
                    0x9F => { trace_instruction!("RES 3, A") },
                    0x98 => { trace_instruction!("RES 3, B") },
                    0x99 => { trace_instruction!("RES 3, C") },
                    0x9A => { trace_instruction!("RES 3, D") },
                    0x9B => { trace_instruction!("RES 3, E") },
                    0x9C => { trace_instruction!("RES 3, H") },
                    0x9D => { trace_instruction!("RES 3, L") },
                    0x9E => { trace_instruction!("RES 3, (HL)") },
                    0xA7 => { trace_instruction!("RES 4, A") },
                    0xA0 => { trace_instruction!("RES 4, B") },
                    0xA1 => { trace_instruction!("RES 4, C") },
                    0xA2 => { trace_instruction!("RES 4, D") },
                    0xA3 => { trace_instruction!("RES 4, E") },
                    0xA4 => { trace_instruction!("RES 4, H") },
                    0xA5 => { trace_instruction!("RES 4, L") },
                    0xA6 => { trace_instruction!("RES 4, (HL)") },
                    0xAF => { trace_instruction!("RES 5, A") },
                    0xA8 => { trace_instruction!("RES 5, B") },
                    0xA9 => { trace_instruction!("RES 5, C") },
                    0xAA => { trace_instruction!("RES 5, D") },
                    0xAB => { trace_instruction!("RES 5, E") },
                    0xAC => { trace_instruction!("RES 5, H") },
                    0xAD => { trace_instruction!("RES 5, L") },
                    0xAE => { trace_instruction!("RES 5, (HL)") },
                    0xB7 => { trace_instruction!("RES 6, A") },
                    0xB0 => { trace_instruction!("RES 6, B") },
                    0xB1 => { trace_instruction!("RES 6, C") },
                    0xB2 => { trace_instruction!("RES 6, D") },
                    0xB3 => { trace_instruction!("RES 6, E") },
                    0xB4 => { trace_instruction!("RES 6, H") },
                    0xB5 => { trace_instruction!("RES 6, L") },
                    0xB6 => { trace_instruction!("RES 6, (HL)") },
                    0xBF => { trace_instruction!("RES 7, A") },
                    0xB8 => { trace_instruction!("RES 7, B") },
                    0xB9 => { trace_instruction!("RES 7, C") },
                    0xBA => { trace_instruction!("RES 7, D") },
                    0xBB => { trace_instruction!("RES 7, E") },
                    0xBC => { trace_instruction!("RES 7, H") },
                    0xBD => { trace_instruction!("RES 7, L") },
                    0xBE => { trace_instruction!("RES 7, (HL)") },
                    0xC7 => { trace_instruction!("SET 0, A") },
                    0xC0 => { trace_instruction!("SET 0, B") },
                    0xC1 => { trace_instruction!("SET 0, C") },
                    0xC2 => { trace_instruction!("SET 0, D") },
                    0xC3 => { trace_instruction!("SET 0, E") },
                    0xC4 => { trace_instruction!("SET 0, H") },
                    0xC5 => { trace_instruction!("SET 0, L") },
                    0xC6 => { trace_instruction!("SET 0, (HL)") },
                    0xCF => { trace_instruction!("SET 1, A") },
                    0xC8 => { trace_instruction!("SET 1, B") },
                    0xC9 => { trace_instruction!("SET 1, C") },
                    0xCA => { trace_instruction!("SET 1, D") },
                    0xCB => { trace_instruction!("SET 1, E") },
                    0xCC => { trace_instruction!("SET 1, H") },
                    0xCD => { trace_instruction!("SET 1, L") },
                    0xCE => { trace_instruction!("SET 1, (HL)") },
                    0xD7 => { trace_instruction!("SET 2, A") },
                    0xD0 => { trace_instruction!("SET 2, B") },
                    0xD1 => { trace_instruction!("SET 2, C") },
                    0xD2 => { trace_instruction!("SET 2, D") },
                    0xD3 => { trace_instruction!("SET 2, E") },
                    0xD4 => { trace_instruction!("SET 2, H") },
                    0xD5 => { trace_instruction!("SET 2, L") },
                    0xD6 => { trace_instruction!("SET 2, (HL)") },
                    0xDF => { trace_instruction!("SET 3, A") },
                    0xD8 => { trace_instruction!("SET 3, B") },
                    0xD9 => { trace_instruction!("SET 3, C") },
                    0xDA => { trace_instruction!("SET 3, D") },
                    0xDB => { trace_instruction!("SET 3, E") },
                    0xDC => { trace_instruction!("SET 3, H") },
                    0xDD => { trace_instruction!("SET 3, L") },
                    0xDE => { trace_instruction!("SET 3, (HL)") },
                    0xE7 => { trace_instruction!("SET 4, A") },
                    0xE0 => { trace_instruction!("SET 4, B") },
                    0xE1 => { trace_instruction!("SET 4, C") },
                    0xE2 => { trace_instruction!("SET 4, D") },
                    0xE3 => { trace_instruction!("SET 4, E") },
                    0xE4 => { trace_instruction!("SET 4, H") },
                    0xE5 => { trace_instruction!("SET 4, L") },
                    0xE6 => { trace_instruction!("SET 4, (HL)") },
                    0xEF => { trace_instruction!("SET 5, A") },
                    0xE8 => { trace_instruction!("SET 5, B") },
                    0xE9 => { trace_instruction!("SET 5, C") },
                    0xEA => { trace_instruction!("SET 5, D") },
                    0xEB => { trace_instruction!("SET 5, E") },
                    0xEC => { trace_instruction!("SET 5, H") },
                    0xED => { trace_instruction!("SET 5, L") },
                    0xEE => { trace_instruction!("SET 5, (HL)") },
                    0xF7 => { trace_instruction!("SET 6, A") },
                    0xF0 => { trace_instruction!("SET 6, B") },
                    0xF1 => { trace_instruction!("SET 6, C") },
                    0xF2 => { trace_instruction!("SET 6, D") },
                    0xF3 => { trace_instruction!("SET 6, E") },
                    0xF4 => { trace_instruction!("SET 6, H") },
                    0xF5 => { trace_instruction!("SET 6, L") },
                    0xF6 => { trace_instruction!("SET 6, (HL)") },
                    0xFF => { trace_instruction!("SET 7, A") },
                    0xF8 => { trace_instruction!("SET 7, B") },
                    0xF9 => { trace_instruction!("SET 7, C") },
                    0xFA => { trace_instruction!("SET 7, D") },
                    0xFB => { trace_instruction!("SET 7, E") },
                    0xFC => { trace_instruction!("SET 7, H") },
                    0xFD => { trace_instruction!("SET 7, L") },
                    0xFE => { trace_instruction!("SET 7, (HL)") },
                }
            },
            _ => { error!("Unknown op code 0x{:02X}", op) },
        }
    }

    #[cfg(not(debug_assertions))]
    fn dump_instruction<T: Deref<Target=[u8]>>(&self, _bus: &Bus<T>, _op: u8) {
    }

    /// Decode the provided op code and execute the instruction
    fn decode_execute<T: Deref<Target=[u8]>>(&mut self, bus: &mut Bus<T>, op: u8) -> u8 {
        self.dump_instruction(bus, op);

        match op {
            // --- Misc
            // NOP
            0x00 => { 4 },
            // DAA
            0x27 => { self.daa(); 4 },
            // CPL
            0x2F => { self.cpl(); 4 },
            // SCF
            0x37 => { self.scf(); 4 },
            // CCF
            0x3F => { self.ccf(); 4 },
            // HALT
            0x76 => { self.halted = true; 4 },
            // STOP
            0x10 => { self.fetch(bus); self.stopped = true; 4 },
            // --- LD
            // LD BC, nn
            0x01 => { let nn = self.fetch16(bus); self.set_bc(nn); 12 },
            // LD DE, nn
            0x11 => { let nn = self.fetch16(bus); self.set_de(nn); 12 },
            // LD HL, nn
            0x21 => { let nn = self.fetch16(bus); self.set_hl(nn); 12 },
            // LD SP, nn
            0x31 => { let nn = self.fetch16(bus); self.sp = nn; 12 },
            // LD r, n
            0x06 => { self.b = self.fetch(bus); 8 },
            0x0E => { self.c = self.fetch(bus); 8 },
            0x16 => { self.d = self.fetch(bus); 8 },
            0x1E => { self.e = self.fetch(bus); 8 },
            0x26 => { self.h = self.fetch(bus); 8 },
            0x2E => { self.l = self.fetch(bus); 8 },
            0x3E => { self.a = self.fetch(bus); 8 },
            // LD B, r
            0x40 => { 4 },
            0x41 => { self.b = self.c; 4 },
            0x42 => { self.b = self.d; 4 },
            0x43 => { self.b = self.e; 4 },
            0x44 => { self.b = self.h; 4 },
            0x45 => { self.b = self.l; 4 },
            0x47 => { self.b = self.a; 4 },
            // LD C, r
            0x48 => { self.c = self.b; 4 },
            0x49 => { 4 },
            0x4A => { self.c = self.d; 4 },
            0x4B => { self.c = self.e; 4 },
            0x4C => { self.c = self.h; 4 },
            0x4D => { self.c = self.l; 4 },
            0x4F => { self.c = self.a; 4 },
            // LD D, r
            0x50 => { self.d = self.b; 4 },
            0x51 => { self.d = self.c; 4 },
            0x52 => { 4 },
            0x53 => { self.d = self.e; 4 },
            0x54 => { self.d = self.h; 4 },
            0x55 => { self.d = self.l; 4 },
            0x57 => { self.d = self.a; 4 },
            // LD E, r
            0x58 => { self.e = self.b; 4 },
            0x59 => { self.e = self.c; 4 },
            0x5A => { self.e = self.d; 4 },
            0x5B => { 4 },
            0x5C => { self.e = self.h; 4 },
            0x5D => { self.e = self.l; 4 },
            0x5F => { self.e = self.a; 4 },
            // LD H, r
            0x60 => { self.h = self.b; 4 },
            0x61 => { self.h = self.c; 4 },
            0x62 => { self.h = self.d; 4 },
            0x63 => { self.h = self.e; 4 },
            0x64 => { 4 },
            0x65 => { self.h = self.l; 4 },
            0x67 => { self.h = self.a; 4 },
            // LD L, r
            0x68 => { self.l = self.b; 4 },
            0x69 => { self.l = self.c; 4 },
            0x6A => { self.l = self.d; 4 },
            0x6B => { self.l = self.e; 4 },
            0x6C => { self.l = self.h; 4 },
            0x6D => { 4 },
            0x6F => { self.l = self.a; 4 },
            // LD A, r
            0x78 => { self.a = self.b; 4 },
            0x79 => { self.a = self.c; 4 },
            0x7A => { self.a = self.d; 4 },
            0x7B => { self.a = self.e; 4 },
            0x7C => { self.a = self.h; 4 },
            0x7D => { self.a = self.l; 4 },
            0x7F => { 4 },
            // LD A, (HL+)
            0x2A => { self.a = bus.read(self.hl()); self.inc_hl(); 8 },
            // LD A, (HL-)
            0x3A => { self.a = bus.read(self.hl()); self.dec_hl(); 8 },
            // LD A, (BC)
            0x0A => { self.a = bus.read(self.bc()); 8 },
            // LD A, (DE)
            0x1A => { self.a = bus.read(self.de()); 8 },
            // LD A, (nn)
            0xFA => { let nn = self.fetch16(bus); self.a = bus.read(nn); 16 },
            // LD (nn), A
            0xEA => { let nn = self.fetch16(bus); bus.write(nn, self.a); 16 },
            // LD r, (HL)
            0x46 => { self.b = bus.read(self.hl()); 8 },
            0x4E => { self.c = bus.read(self.hl()); 8 },
            0x56 => { self.d = bus.read(self.hl()); 8 },
            0x5E => { self.e = bus.read(self.hl()); 8 },
            0x66 => { self.h = bus.read(self.hl()); 8 },
            0x6E => { self.l = bus.read(self.hl()); 8 },
            0x7E => { self.a = bus.read(self.hl()); 8 },
            // LD (HL), n
            0x36 => { let n = self.fetch(bus); bus.write(self.hl(), n); 12 },
            // LD (HL), r
            0x70 => { bus.write(self.hl(), self.b); 8 },
            0x71 => { bus.write(self.hl(), self.c); 8 },
            0x72 => { bus.write(self.hl(), self.d); 8 },
            0x73 => { bus.write(self.hl(), self.e); 8 },
            0x74 => { bus.write(self.hl(), self.h); 8 },
            0x75 => { bus.write(self.hl(), self.l); 8 },
            0x77 => { bus.write(self.hl(), self.a); 8 },
            // LD (HL+), A
            0x22 => { bus.write(self.hl(), self.a); self.inc_hl(); 8 },
            // LD (HL-), A
            0x32 => { bus.write(self.hl(), self.a); self.dec_hl(); 8 },
            // LD (BC), A
            0x02 => { bus.write(self.bc(), self.a); 8 },
            // LD (DE), A
            0x12 => { bus.write(self.de(), self.a); 8 },
            // LD ($FF00 + C), A
            0xE2 => { bus.write(IO_REGION_START + self.c as u16, self.a); 8 },
            // LD A, ($FF00 + C)
            0xF2 => { self.a = bus.read(IO_REGION_START + self.c as u16); 8 },
            // LD ($FF00 + n), A
            0xE0 => { let n = self.fetch(bus); bus.write(IO_REGION_START + n as u16, self.a); 12 },
            // LD A, ($FF00 + n)
            0xF0 => { let n = self.fetch(bus); self.a = bus.read(IO_REGION_START + n as u16); 12 },
            // LD HL, SP+n
            0xF8 => { self.ld_hl_spn(bus); 12 },
            // LD (nn), SP
            0x08 => {
                let nn = self.fetch16(bus);
                bus.write(nn, self.sp as u8);
                bus.write(nn + 1, (self.sp >> 8) as u8);
                20
            },
            // LD SP, HL
            0xF9 => { self.sp = self.hl(); 8 },
            // ---
            // PUSH rr
            0xF5 => { self.push(bus, self.af()); 16 },
            0xC5 => { self.push(bus, self.bc()); 16 },
            0xD5 => { self.push(bus, self.de()); 16 },
            0xE5 => { self.push(bus, self.hl()); 16 },
            // POP rr
            0xF1 => { let rr = self.pop(bus); self.set_af(rr & 0xFFF0); 12 },
            0xC1 => { let rr = self.pop(bus); self.set_bc(rr); 12 },
            0xD1 => { let rr = self.pop(bus); self.set_de(rr); 12 },
            0xE1 => { let rr = self.pop(bus); self.set_hl(rr); 12 },
            // ---
            // JP nn
            0xC3 => { let nn = self.fetch16(bus); self.pc = nn; 12 },
            // JP cc, nn
            0xC2 => { let nn = self.fetch16(bus); self.jump_if(nn, (self.f & FLAG_ZERO) == 0) },
            0xCA => { let nn = self.fetch16(bus); self.jump_if(nn, (self.f & FLAG_ZERO) == FLAG_ZERO) },
            0xD2 => { let nn = self.fetch16(bus); self.jump_if(nn, (self.f & FLAG_CARRY) == 0) },
            0xDA => { let nn = self.fetch16(bus); self.jump_if(nn, (self.f & FLAG_CARRY) == FLAG_CARRY) },
            // JP HL
            0xE9 => { self.pc = self.hl(); 4 },
            // JR n
            0x18 => {
                let n = self.fetch(bus);
                self.pc = ((self.pc as i32) + ((n as i8) as i32)) as u16;
                8
            },
            // JR cc, n
            0x20 => { let n = self.fetch(bus); self.jump_if_rel(n, (self.f & FLAG_ZERO) == 0) },
            0x28 => { let n = self.fetch(bus); self.jump_if_rel(n, (self.f & FLAG_ZERO) == FLAG_ZERO) },
            0x30 => { let n = self.fetch(bus); self.jump_if_rel(n, (self.f & FLAG_CARRY) == 0) },
            0x38 => { let n = self.fetch(bus); self.jump_if_rel(n, (self.f & FLAG_CARRY) == FLAG_CARRY) },
            // CALL nn
            0xCD => { let nn = self.fetch16(bus); self.call(bus, nn); 24 },
            // CALL cc, nn
            0xC4 => { let nn = self.fetch16(bus); self.call_if(bus, nn, (self.f & FLAG_ZERO) == 0) },
            0xCC => { let nn = self.fetch16(bus); self.call_if(bus, nn, (self.f & FLAG_ZERO) == FLAG_ZERO) },
            0xD4 => { let nn = self.fetch16(bus); self.call_if(bus, nn, (self.f & FLAG_CARRY) == 0) },
            0xDC => { let nn = self.fetch16(bus); self.call_if(bus, nn, (self.f & FLAG_CARRY) == FLAG_CARRY) },
            // RST n
            0xC7 => { self.call(bus, 0x00u16); 16 },
            0xCF => { self.call(bus, 0x08u16); 16 },
            0xD7 => { self.call(bus, 0x10u16); 16 },
            0xDF => { self.call(bus, 0x18u16); 16 },
            0xE7 => { self.call(bus, 0x20u16); 16 },
            0xEF => { self.call(bus, 0x28u16); 16 },
            0xF7 => { self.call(bus, 0x30u16); 16 },
            0xFF => { self.call(bus, 0x38u16); 16 },
            // RET
            0xC9 => { self.pc = self.pop(bus); 16 },
            // RET cc
            0xC0 => { self.ret_if(bus, (self.f & FLAG_ZERO) == 0) },
            0xC8 => { self.ret_if(bus, (self.f & FLAG_ZERO) == FLAG_ZERO) },
            0xD0 => { self.ret_if(bus, (self.f & FLAG_CARRY) == 0) },
            0xD8 => { self.ret_if(bus, (self.f & FLAG_CARRY) == FLAG_CARRY) },
            // RETI
            0xD9 => { self.pc = self.pop(bus); self.master_ie = true; 8 }
            // --- 8-bit arithmetic
            // ADD A, n
            0x87 => { self.add(self.a); 4 },
            0x80 => { self.add(self.b); 4 },
            0x81 => { self.add(self.c); 4 },
            0x82 => { self.add(self.d); 4 },
            0x83 => { self.add(self.e); 4 },
            0x84 => { self.add(self.h); 4 },
            0x85 => { self.add(self.l); 4 },
            0x86 => { let n = bus.read(self.hl()); self.add(n); 8 },
            0xC6 => { let n = self.fetch(bus); self.add(n); 8 },
            // ADC A, n
            0x8F => { self.adc(self.a); 4 },
            0x88 => { self.adc(self.b); 4 },
            0x89 => { self.adc(self.c); 4 },
            0x8A => { self.adc(self.d); 4 },
            0x8B => { self.adc(self.e); 4 },
            0x8C => { self.adc(self.h); 4 },
            0x8D => { self.adc(self.l); 4 },
            0x8E => { let n = bus.read(self.hl()); self.adc(n); 8 },
            0xCE => { let n = self.fetch(bus); self.adc(n); 8 },
            // SUB A, n
            0x97 => { self.sub(self.a); 4 },
            0x90 => { self.sub(self.b); 4 },
            0x91 => { self.sub(self.c); 4 },
            0x92 => { self.sub(self.d); 4 },
            0x93 => { self.sub(self.e); 4 },
            0x94 => { self.sub(self.h); 4 },
            0x95 => { self.sub(self.l); 4 },
            0x96 => { let n = bus.read(self.hl()); self.sub(n); 8 },
            0xD6 => { let n = self.fetch(bus); self.sub(n); 8 },
            // SBC A, n
            0x9F => { self.sbc(self.a); 4 },
            0x98 => { self.sbc(self.b); 4 },
            0x99 => { self.sbc(self.c); 4 },
            0x9A => { self.sbc(self.d); 4 },
            0x9B => { self.sbc(self.e); 4 },
            0x9C => { self.sbc(self.h); 4 },
            0x9D => { self.sbc(self.l); 4 },
            0x9E => { let n = bus.read(self.hl()); self.sbc(n); 8 },
            0xDE => { let n = self.fetch(bus); self.sbc(n); 8 },
            // AND n
            0xA7 => { self.and(self.a); 4 },
            0xA0 => { self.and(self.b); 4 },
            0xA1 => { self.and(self.c); 4 },
            0xA2 => { self.and(self.d); 4 },
            0xA3 => { self.and(self.e); 4 },
            0xA4 => { self.and(self.h); 4 },
            0xA5 => { self.and(self.l); 4 },
            0xA6 => { let n = bus.read(self.hl()); self.and(n); 8 },
            0xE6 => { let n = self.fetch(bus); self.and(n); 8 },
            // OR n
            0xB7 => { self.or(self.a); 4 },
            0xB0 => { self.or(self.b); 4 },
            0xB1 => { self.or(self.c); 4 },
            0xB2 => { self.or(self.d); 4 },
            0xB3 => { self.or(self.e); 4 },
            0xB4 => { self.or(self.h); 4 },
            0xB5 => { self.or(self.l); 4 },
            0xB6 => { let n = bus.read(self.hl()); self.or(n); 8 },
            0xF6 => { let n = self.fetch(bus); self.or(n); 8 },
            // XOR n
            0xAF => { self.xor(self.a); 4 },
            0xA8 => { self.xor(self.b); 4 },
            0xA9 => { self.xor(self.c); 4 },
            0xAA => { self.xor(self.d); 4 },
            0xAB => { self.xor(self.e); 4 },
            0xAC => { self.xor(self.h); 4 },
            0xAD => { self.xor(self.l); 4 },
            0xAE => { let n = bus.read(self.hl()); self.xor(n); 8 },
            0xEE => { let n = self.fetch(bus); self.xor(n); 8 },
            // CP n
            0xBF => { self.cp(self.a); 4 },
            0xB8 => { self.cp(self.b); 4 },
            0xB9 => { self.cp(self.c); 4 },
            0xBA => { self.cp(self.d); 4 },
            0xBB => { self.cp(self.e); 4 },
            0xBC => { self.cp(self.h); 4 },
            0xBD => { self.cp(self.l); 4 },
            0xBE => { let n = bus.read(self.hl()); self.cp(n); 8 },
            0xFE => { let n = self.fetch(bus); self.cp(n); 8 },
            // INC n
            0x3C => { self.a = self.inc(self.a); 4 },
            0x04 => { self.b = self.inc(self.b); 4 },
            0x0C => { self.c = self.inc(self.c); 4 },
            0x14 => { self.d = self.inc(self.d); 4 },
            0x1C => { self.e = self.inc(self.e); 4 },
            0x24 => { self.h = self.inc(self.h); 4 },
            0x2C => { self.l = self.inc(self.l); 4 },
            0x34 => {
                let hl = self.hl();
                let n = bus.read(hl);
                let r = self.inc(n);
                bus.write(hl, r);
                12
            },
            // DEC n
            0x3D => { self.a = self.dec(self.a); 4 },
            0x05 => { self.b = self.dec(self.b); 4 },
            0x0D => { self.c = self.dec(self.c); 4 },
            0x15 => { self.d = self.dec(self.d); 4 },
            0x1D => { self.e = self.dec(self.e); 4 },
            0x25 => { self.h = self.dec(self.h); 4 },
            0x2D => { self.l = self.dec(self.l); 4 },
            0x35 => {
                let hl = self.hl();
                let n = bus.read(hl);
                let r = self.dec(n);
                bus.write(hl, r);
                12
            },
            // --- 16-bit arithmetic
            // ADD HL, r
            0x09 => { self.add16(self.bc()); 8 },
            0x19 => { self.add16(self.de()); 8 },
            0x29 => { self.add16(self.hl()); 8 },
            0x39 => { self.add16(self.sp); 8 },
            // ADD SP, n
            0xE8 => {
                let n = self.fetch(bus);
                let r = (self.sp as i32).wrapping_add((n as i8) as i32) as u16;
                self.set_flag(FLAG_ZERO, false);
                self.set_flag(FLAG_SUBSTRACT, false);
                self.set_flag(FLAG_CARRY, (r & 0xFF) < (self.sp & 0xFF));
                self.set_flag(FLAG_HALF_CARRY, (r & 0xF) < (self.sp & 0xF));
                self.sp = r as u16;
                16
            },
            // INC rr
            0x03 => { let rr = self.bc().wrapping_add(1); self.set_bc(rr); 8 },
            0x13 => { let rr = self.de().wrapping_add(1); self.set_de(rr); 8 },
            0x23 => { let rr = self.hl().wrapping_add(1); self.set_hl(rr); 8 },
            0x33 => { self.sp = self.sp.wrapping_add(1); 8 },
            // DEC rr
            0x0B => { let rr = self.bc().wrapping_sub(1); self.set_bc(rr); 8 },
            0x1B => { let rr = self.de().wrapping_sub(1); self.set_de(rr); 8 },
            0x2B => { let rr = self.hl().wrapping_sub(1); self.set_hl(rr); 8 },
            0x3B => { self.sp = self.sp.wrapping_sub(1); 8 },
            // DI
            0xF3 => {
                self.enabling_ie = false;
                self.master_ie = false;
                4
            },
            // EI
            0xFB => { self.enabling_ie = true; 4 },
            // Rotates
            0x07 => { self.a = self.rl(self.a, false, false); 4 },
            0x17 => { self.a = self.rl(self.a, true, false); 4 },
            0x0F => { self.a = self.rr(self.a, false, false); 4 },
            0x1F => { self.a = self.rr(self.a, true, false); 4 },
            // --- CB prefixed commands
            0xCB => {
                let op2 = self.fetch(bus);

                match op2 {
                    // SWAP n
                    0x37 => { self.a = self.swap(self.a); 8 },
                    0x30 => { self.b = self.swap(self.b); 8 },
                    0x31 => { self.c = self.swap(self.c); 8 },
                    0x32 => { self.d = self.swap(self.d); 8 },
                    0x33 => { self.e = self.swap(self.e); 8 },
                    0x34 => { self.h = self.swap(self.h); 8 },
                    0x35 => { self.l = self.swap(self.l); 8 },
                    0x36 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        let r = self.swap(n);
                        bus.write(hl, r);
                        16
                    },
                    // RLC n
                    0x07 => { self.a = self.rl(self.a, false, true); 8 },
                    0x00 => { self.b = self.rl(self.b, false, true); 8 },
                    0x01 => { self.c = self.rl(self.c, false, true); 8 },
                    0x02 => { self.d = self.rl(self.d, false, true); 8 },
                    0x03 => { self.e = self.rl(self.e, false, true); 8 },
                    0x04 => { self.h = self.rl(self.h, false, true); 8 },
                    0x05 => { self.l = self.rl(self.l, false, true); 8 },
                    0x06 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        let res = self.rl(n, false, true);
                        bus.write(hl, res);
                        16
                    },
                    // RL n
                    0x17 => { self.a = self.rl(self.a, true, true); 8 },
                    0x10 => { self.b = self.rl(self.b, true, true); 8 },
                    0x11 => { self.c = self.rl(self.c, true, true); 8 },
                    0x12 => { self.d = self.rl(self.d, true, true); 8 },
                    0x13 => { self.e = self.rl(self.e, true, true); 8 },
                    0x14 => { self.h = self.rl(self.h, true, true); 8 },
                    0x15 => { self.l = self.rl(self.l, true, true); 8 },
                    0x16 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        let res = self.rl(n, true, true);
                        bus.write(hl, res);
                        16
                    },
                    // RRC n
                    0x0F => { self.a = self.rr(self.a, false, true); 8 },
                    0x08 => { self.b = self.rr(self.b, false, true); 8 },
                    0x09 => { self.c = self.rr(self.c, false, true); 8 },
                    0x0A => { self.d = self.rr(self.d, false, true); 8 },
                    0x0B => { self.e = self.rr(self.e, false, true); 8 },
                    0x0C => { self.h = self.rr(self.h, false, true); 8 },
                    0x0D => { self.l = self.rr(self.l, false, true); 8 },
                    0x0E => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        let res = self.rr(n, false, true);
                        bus.write(hl, res);
                        16
                    },
                    // RRC n
                    0x1F => { self.a = self.rr(self.a, true, true); 8 },
                    0x18 => { self.b = self.rr(self.b, true, true); 8 },
                    0x19 => { self.c = self.rr(self.c, true, true); 8 },
                    0x1A => { self.d = self.rr(self.d, true, true); 8 },
                    0x1B => { self.e = self.rr(self.e, true, true); 8 },
                    0x1C => { self.h = self.rr(self.h, true, true); 8 },
                    0x1D => { self.l = self.rr(self.l, true, true); 8 },
                    0x1E => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        let res = self.rr(n, true, true);
                        bus.write(hl, res);
                        16
                    },
                    // SLA n
                    0x27 => { self.a = self.sl(self.a); 8 },
                    0x20 => { self.b = self.sl(self.b); 8 },
                    0x21 => { self.c = self.sl(self.c); 8 },
                    0x22 => { self.d = self.sl(self.d); 8 },
                    0x23 => { self.e = self.sl(self.e); 8 },
                    0x24 => { self.h = self.sl(self.h); 8 },
                    0x25 => { self.l = self.sl(self.l); 8 },
                    0x26 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        let res = self.sl(n);
                        bus.write(hl, res);
                        16
                    },
                    // SRA n
                    0x2F => { self.a = self.sr(self.a, true); 8 },
                    0x28 => { self.b = self.sr(self.b, true); 8 },
                    0x29 => { self.c = self.sr(self.c, true); 8 },
                    0x2A => { self.d = self.sr(self.d, true); 8 },
                    0x2B => { self.e = self.sr(self.e, true); 8 },
                    0x2C => { self.h = self.sr(self.h, true); 8 },
                    0x2D => { self.l = self.sr(self.l, true); 8 },
                    0x2E => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        let res = self.sr(n, true);
                        bus.write(hl, res);
                        16
                    },
                    // SRL n
                    0x3F => { self.a = self.sr(self.a, false); 8 },
                    0x38 => { self.b = self.sr(self.b, false); 8 },
                    0x39 => { self.c = self.sr(self.c, false); 8 },
                    0x3A => { self.d = self.sr(self.d, false); 8 },
                    0x3B => { self.e = self.sr(self.e, false); 8 },
                    0x3C => { self.h = self.sr(self.h, false); 8 },
                    0x3D => { self.l = self.sr(self.l, false); 8 },
                    0x3E => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        let res = self.sr(n, false);
                        bus.write(hl, res);
                        16
                    },
                    // BIT 0, r
                    0x47 => { self.bit(self.a, 0x01); 8 },
                    0x40 => { self.bit(self.b, 0x01); 8 },
                    0x41 => { self.bit(self.c, 0x01); 8 },
                    0x42 => { self.bit(self.d, 0x01); 8 },
                    0x43 => { self.bit(self.e, 0x01); 8 },
                    0x44 => { self.bit(self.h, 0x01); 8 },
                    0x45 => { self.bit(self.l, 0x01); 8 },
                    0x46 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        self.bit(n, 0x01);
                        16
                    },
                    // BIT 1, r
                    0x4F => { self.bit(self.a, 0x01 << 1); 8 },
                    0x48 => { self.bit(self.b, 0x01 << 1); 8 },
                    0x49 => { self.bit(self.c, 0x01 << 1); 8 },
                    0x4A => { self.bit(self.d, 0x01 << 1); 8 },
                    0x4B => { self.bit(self.e, 0x01 << 1); 8 },
                    0x4C => { self.bit(self.h, 0x01 << 1); 8 },
                    0x4D => { self.bit(self.l, 0x01 << 1); 8 },
                    0x4E => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        self.bit(n, 0x01 << 1);
                        16
                    },
                    // BIT 2, r
                    0x57 => { self.bit(self.a, 0x01 << 2); 8 },
                    0x50 => { self.bit(self.b, 0x01 << 2); 8 },
                    0x51 => { self.bit(self.c, 0x01 << 2); 8 },
                    0x52 => { self.bit(self.d, 0x01 << 2); 8 },
                    0x53 => { self.bit(self.e, 0x01 << 2); 8 },
                    0x54 => { self.bit(self.h, 0x01 << 2); 8 },
                    0x55 => { self.bit(self.l, 0x01 << 2); 8 },
                    0x56 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        self.bit(n, 0x01 << 2);
                        16
                    },
                    // BIT 3, r
                    0x5F => { self.bit(self.a, 0x01 << 3); 8 },
                    0x58 => { self.bit(self.b, 0x01 << 3); 8 },
                    0x59 => { self.bit(self.c, 0x01 << 3); 8 },
                    0x5A => { self.bit(self.d, 0x01 << 3); 8 },
                    0x5B => { self.bit(self.e, 0x01 << 3); 8 },
                    0x5C => { self.bit(self.h, 0x01 << 3); 8 },
                    0x5D => { self.bit(self.l, 0x01 << 3); 8 },
                    0x5E => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        self.bit(n, 0x01 << 3);
                        16
                    },
                    // BIT 4, r
                    0x67 => { self.bit(self.a, 0x01 << 4); 8 },
                    0x60 => { self.bit(self.b, 0x01 << 4); 8 },
                    0x61 => { self.bit(self.c, 0x01 << 4); 8 },
                    0x62 => { self.bit(self.d, 0x01 << 4); 8 },
                    0x63 => { self.bit(self.e, 0x01 << 4); 8 },
                    0x64 => { self.bit(self.h, 0x01 << 4); 8 },
                    0x65 => { self.bit(self.l, 0x01 << 4); 8 },
                    0x66 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        self.bit(n, 0x01 << 4);
                        16
                    },
                    // BIT 5, r
                    0x6F => { self.bit(self.a, 0x01 << 5); 8 },
                    0x68 => { self.bit(self.b, 0x01 << 5); 8 },
                    0x69 => { self.bit(self.c, 0x01 << 5); 8 },
                    0x6A => { self.bit(self.d, 0x01 << 5); 8 },
                    0x6B => { self.bit(self.e, 0x01 << 5); 8 },
                    0x6C => { self.bit(self.h, 0x01 << 5); 8 },
                    0x6D => { self.bit(self.l, 0x01 << 5); 8 },
                    0x6E => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        self.bit(n, 0x01 << 5);
                        16
                    },
                    // BIT 6, r
                    0x77 => { self.bit(self.a, 0x01 << 6); 8 },
                    0x70 => { self.bit(self.b, 0x01 << 6); 8 },
                    0x71 => { self.bit(self.c, 0x01 << 6); 8 },
                    0x72 => { self.bit(self.d, 0x01 << 6); 8 },
                    0x73 => { self.bit(self.e, 0x01 << 6); 8 },
                    0x74 => { self.bit(self.h, 0x01 << 6); 8 },
                    0x75 => { self.bit(self.l, 0x01 << 6); 8 },
                    0x76 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        self.bit(n, 0x01 << 6);
                        16
                    },
                    // BIT 7, r
                    0x7F => { self.bit(self.a, 0x01 << 7); 8 },
                    0x78 => { self.bit(self.b, 0x01 << 7); 8 },
                    0x79 => { self.bit(self.c, 0x01 << 7); 8 },
                    0x7A => { self.bit(self.d, 0x01 << 7); 8 },
                    0x7B => { self.bit(self.e, 0x01 << 7); 8 },
                    0x7C => { self.bit(self.h, 0x01 << 7); 8 },
                    0x7D => { self.bit(self.l, 0x01 << 7); 8 },
                    0x7E => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        self.bit(n, 0x01 << 7);
                        16
                    },
                    // RES 0, r
                    0x87 => { self.a &= !0x01; 8 },
                    0x80 => { self.b &= !0x01; 8 },
                    0x81 => { self.c &= !0x01; 8 },
                    0x82 => { self.d &= !0x01; 8 },
                    0x83 => { self.e &= !0x01; 8 },
                    0x84 => { self.h &= !0x01; 8 },
                    0x85 => { self.l &= !0x01; 8 },
                    0x86 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n & !0x01);
                        16
                    },
                    // RES 1, r
                    0x8F => { self.a &= !(0x01 << 1); 8 },
                    0x88 => { self.b &= !(0x01 << 1); 8 },
                    0x89 => { self.c &= !(0x01 << 1); 8 },
                    0x8A => { self.d &= !(0x01 << 1); 8 },
                    0x8B => { self.e &= !(0x01 << 1); 8 },
                    0x8C => { self.h &= !(0x01 << 1); 8 },
                    0x8D => { self.l &= !(0x01 << 1); 8 },
                    0x8E => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n & !(0x01 << 1));
                        16
                    },
                    // RES 2, r
                    0x97 => { self.a &= !(0x01 << 2); 8 },
                    0x90 => { self.b &= !(0x01 << 2); 8 },
                    0x91 => { self.c &= !(0x01 << 2); 8 },
                    0x92 => { self.d &= !(0x01 << 2); 8 },
                    0x93 => { self.e &= !(0x01 << 2); 8 },
                    0x94 => { self.h &= !(0x01 << 2); 8 },
                    0x95 => { self.l &= !(0x01 << 2); 8 },
                    0x96 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n & !(0x01 << 2));
                        16
                    },
                    // RES 3, r
                    0x9F => { self.a &= !(0x01 << 3); 8 },
                    0x98 => { self.b &= !(0x01 << 3); 8 },
                    0x99 => { self.c &= !(0x01 << 3); 8 },
                    0x9A => { self.d &= !(0x01 << 3); 8 },
                    0x9B => { self.e &= !(0x01 << 3); 8 },
                    0x9C => { self.h &= !(0x01 << 3); 8 },
                    0x9D => { self.l &= !(0x01 << 3); 8 },
                    0x9E => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n & !(0x01 << 3));
                        16
                    },
                    // RES 4, r
                    0xA7 => { self.a &= !(0x01 << 4); 8 },
                    0xA0 => { self.b &= !(0x01 << 4); 8 },
                    0xA1 => { self.c &= !(0x01 << 4); 8 },
                    0xA2 => { self.d &= !(0x01 << 4); 8 },
                    0xA3 => { self.e &= !(0x01 << 4); 8 },
                    0xA4 => { self.h &= !(0x01 << 4); 8 },
                    0xA5 => { self.l &= !(0x01 << 4); 8 },
                    0xA6 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n & !(0x01 << 4));
                        16
                    },
                    // RES 5, r
                    0xAF => { self.a &= !(0x01 << 5); 8 },
                    0xA8 => { self.b &= !(0x01 << 5); 8 },
                    0xA9 => { self.c &= !(0x01 << 5); 8 },
                    0xAA => { self.d &= !(0x01 << 5); 8 },
                    0xAB => { self.e &= !(0x01 << 5); 8 },
                    0xAC => { self.h &= !(0x01 << 5); 8 },
                    0xAD => { self.l &= !(0x01 << 5); 8 },
                    0xAE => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n & !(0x01 << 5));
                        16
                    },
                    // RES 6, r
                    0xB7 => { self.a &= !(0x01 << 6); 8 },
                    0xB0 => { self.b &= !(0x01 << 6); 8 },
                    0xB1 => { self.c &= !(0x01 << 6); 8 },
                    0xB2 => { self.d &= !(0x01 << 6); 8 },
                    0xB3 => { self.e &= !(0x01 << 6); 8 },
                    0xB4 => { self.h &= !(0x01 << 6); 8 },
                    0xB5 => { self.l &= !(0x01 << 6); 8 },
                    0xB6 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n & !(0x01 << 6));
                        16
                    },
                    // RES 7, r
                    0xBF => { self.a &= !(0x01 << 7); 8 },
                    0xB8 => { self.b &= !(0x01 << 7); 8 },
                    0xB9 => { self.c &= !(0x01 << 7); 8 },
                    0xBA => { self.d &= !(0x01 << 7); 8 },
                    0xBB => { self.e &= !(0x01 << 7); 8 },
                    0xBC => { self.h &= !(0x01 << 7); 8 },
                    0xBD => { self.l &= !(0x01 << 7); 8 },
                    0xBE => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n & !(0x01 << 7));
                        16
                    },
                    // SET 0, r
                    0xC7 => { self.a |= 0x01; 8 },
                    0xC0 => { self.b |= 0x01; 8 },
                    0xC1 => { self.c |= 0x01; 8 },
                    0xC2 => { self.d |= 0x01; 8 },
                    0xC3 => { self.e |= 0x01; 8 },
                    0xC4 => { self.h |= 0x01; 8 },
                    0xC5 => { self.l |= 0x01; 8 },
                    0xC6 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n | 0x01);
                        16
                    },
                    // SET 1, r
                    0xCF => { self.a |= 0x01 << 1; 8 },
                    0xC8 => { self.b |= 0x01 << 1; 8 },
                    0xC9 => { self.c |= 0x01 << 1; 8 },
                    0xCA => { self.d |= 0x01 << 1; 8 },
                    0xCB => { self.e |= 0x01 << 1; 8 },
                    0xCC => { self.h |= 0x01 << 1; 8 },
                    0xCD => { self.l |= 0x01 << 1; 8 },
                    0xCE => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n | (0x01 << 1));
                        16
                    },
                    // SET 2, r
                    0xD7 => { self.a |= 0x01 << 2; 8 },
                    0xD0 => { self.b |= 0x01 << 2; 8 },
                    0xD1 => { self.c |= 0x01 << 2; 8 },
                    0xD2 => { self.d |= 0x01 << 2; 8 },
                    0xD3 => { self.e |= 0x01 << 2; 8 },
                    0xD4 => { self.h |= 0x01 << 2; 8 },
                    0xD5 => { self.l |= 0x01 << 2; 8 },
                    0xD6 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n | (0x01 << 2));
                        16
                    },
                    // SET 3, r
                    0xDF => { self.a |= 0x01 << 3; 8 },
                    0xD8 => { self.b |= 0x01 << 3; 8 },
                    0xD9 => { self.c |= 0x01 << 3; 8 },
                    0xDA => { self.d |= 0x01 << 3; 8 },
                    0xDB => { self.e |= 0x01 << 3; 8 },
                    0xDC => { self.h |= 0x01 << 3; 8 },
                    0xDD => { self.l |= 0x01 << 3; 8 },
                    0xDE => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n | (0x01 << 3));
                        16
                    },
                    // SET 4, r
                    0xE7 => { self.a |= 0x01 << 4; 8 },
                    0xE0 => { self.b |= 0x01 << 4; 8 },
                    0xE1 => { self.c |= 0x01 << 4; 8 },
                    0xE2 => { self.d |= 0x01 << 4; 8 },
                    0xE3 => { self.e |= 0x01 << 4; 8 },
                    0xE4 => { self.h |= 0x01 << 4; 8 },
                    0xE5 => { self.l |= 0x01 << 4; 8 },
                    0xE6 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n | (0x01 << 4));
                        16
                    },
                    // SET 5, r
                    0xEF => { self.a |= 0x01 << 5; 8 },
                    0xE8 => { self.b |= 0x01 << 5; 8 },
                    0xE9 => { self.c |= 0x01 << 5; 8 },
                    0xEA => { self.d |= 0x01 << 5; 8 },
                    0xEB => { self.e |= 0x01 << 5; 8 },
                    0xEC => { self.h |= 0x01 << 5; 8 },
                    0xED => { self.l |= 0x01 << 5; 8 },
                    0xEE => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n | (0x01 << 5));
                        16
                    },
                    // SET 6, r
                    0xF7 => { self.a |= 0x01 << 6; 8 },
                    0xF0 => { self.b |= 0x01 << 6; 8 },
                    0xF1 => { self.c |= 0x01 << 6; 8 },
                    0xF2 => { self.d |= 0x01 << 6; 8 },
                    0xF3 => { self.e |= 0x01 << 6; 8 },
                    0xF4 => { self.h |= 0x01 << 6; 8 },
                    0xF5 => { self.l |= 0x01 << 6; 8 },
                    0xF6 => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n | (0x01 << 6));
                        16
                    },
                    // SET 7, r
                    0xFF => { self.a |= 0x01 << 7; 8 },
                    0xF8 => { self.b |= 0x01 << 7; 8 },
                    0xF9 => { self.c |= 0x01 << 7; 8 },
                    0xFA => { self.d |= 0x01 << 7; 8 },
                    0xFB => { self.e |= 0x01 << 7; 8 },
                    0xFC => { self.h |= 0x01 << 7; 8 },
                    0xFD => { self.l |= 0x01 << 7; 8 },
                    0xFE => {
                        let hl = self.hl();
                        let n = bus.read(hl);
                        bus.write(hl, n | (0x01 << 7));
                        16
                    },
                }
            }
            // Unknown op code
            _ => {
                error!("Unknown op code 0x{:02X}", op);
                error!("{}", fmt_registers!(self.pc.wrapping_sub(1), self.sp,
                                            self.af(), self.bc(), self.de(), self.hl()));
                4
            }
        }
    }

    /// Reset all registers & state
    pub fn reset(&mut self) {
        self.a = DEFAULT_REG_A;
        self.f = DEFAULT_REG_F;
        self.b = DEFAULT_REG_B;
        self.c = DEFAULT_REG_C;
        self.d = DEFAULT_REG_D;
        self.e = DEFAULT_REG_E;
        self.h = DEFAULT_REG_H;
        self.l = DEFAULT_REG_L;
        self.sp = DEFAULT_SP;
        self.pc = DEFAULT_PC;
        self.halted = false;
        self.stopped = false;
        self.master_ie = true;
        self.enabling_ie = false;
    }

    /// Fetch, decode and execute next instruction
    /// Returns the number of ticks
    pub fn step<T: Deref<Target=[u8]>>(&mut self, bus: &mut Bus<T>) -> u8 {
        let ticks = if !self.halted {
            // Fetch instruction
            let op = self.fetch(bus);
            // Decode & execute
            self.decode_execute(bus, op)
        } else {
            let pending_it = bus.read(REG_IF_ADDR);
            if pending_it != 0 {
                self.halted = false;
            }
            // If CPU is halted, we assume 4 cycles and return
            4
        };

        // Check for interrupts
        if self.master_ie {
            let int_enable = bus.read(REG_IE_ADDR);
            let int_flag = bus.read(REG_IF_ADDR);

            macro_rules! handle_interrupt {
                ($f:expr, $addr:expr) => {
                    if (int_enable & ($f as u8)) != 0 && (int_flag & ($f as u8)) != 0 {
                        self.call(bus, $addr);
                        bus.it.clear($f);
                        self.halted = false;
                        self.master_ie = false;
                        true
                    } else {
                        false
                    }
                }
            }

            let _ = handle_interrupt!(InterruptFlag::Vblank, IR_VBLANK_ADDR)
                || handle_interrupt!(InterruptFlag::Lcdc, IR_LCDC_STATUS_ADDR)
                || handle_interrupt!(InterruptFlag::TimerOverflow, IR_TIMER_OVERFLOW_ADDR)
                || handle_interrupt!(InterruptFlag::Serial, IR_SERIAL_TRANSFER_ADDR)
                || handle_interrupt!(InterruptFlag::Joypad, IR_JOYPAD_PRESS_ADDR);

        }

        // Enable / Disable interrupt if requested, after 1 instruction
        if self.enabling_ie {
            self.master_ie = true;
        }

        ticks
    }
}
