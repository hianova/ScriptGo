#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    Halt = 0x00,
    LoadImm = 0x01,  // R[A] = B
    LoadImm16 = 0x02,// R[A] = imm16(B, C)
    
    Add = 0x10,      // R[A] = R[B] + R[C]
    Sub = 0x11,      // R[A] = R[B] - R[C]
    Mul = 0x12,      // R[A] = R[B] * R[C]
    Div = 0x13,      // R[A] = R[B] / R[C]
    Mod = 0x14,      // R[A] = R[B] % R[C]
    
    And = 0x20,      // R[A] = R[B] & R[C]
    Or = 0x21,       // R[A] = R[B] | R[C]
    Xor = 0x22,      // R[A] = R[B] ^ R[C]
    Shl = 0x23,      // R[A] = R[B] << R[C]
    Shr = 0x24,      // R[A] = R[B] >> R[C]

    Jmp = 0x30,      // PC = imm16
    JmpIfZero = 0x31,// If R[A] == 0, PC = B
    JmpIfEq = 0x32,  // If R[A] == R[B], PC = C
    JmpIfLt = 0x33,  // If R[A] < R[B], PC = C
    JmpIfGt = 0x34,  // If R[A] > R[B], PC = C

    Call = 0x40,     // Push PC to stack, PC = imm16
    Ret = 0x41,      // Pop PC from stack

    PrintReg = 0x50, // System call: Print R[A]
    
    NeuralCall = 0xFF,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Instruction(pub u32);

impl Instruction {
    pub const fn new(opcode: u8, a: u8, b: u8, c: u8) -> Self {
        Self((opcode as u32) | ((a as u32) << 8) | ((b as u32) << 16) | ((c as u32) << 24))
    }

    #[inline(always)]
    pub fn opcode(&self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    #[inline(always)]
    pub fn a(&self) -> usize {
        ((self.0 >> 8) & 0xFF) as usize
    }

    #[inline(always)]
    pub fn b(&self) -> usize {
        ((self.0 >> 16) & 0xFF) as usize
    }

    #[inline(always)]
    pub fn c(&self) -> usize {
        ((self.0 >> 24) & 0xFF) as usize
    }

    #[inline(always)]
    pub fn imm16(&self) -> u16 {
        ((self.0 >> 16) & 0xFFFF) as u16
    }
}
