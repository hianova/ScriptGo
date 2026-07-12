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
    FAdd = 0x15,     // R[A] = R[B] + R[C] as f32
    FSub = 0x16,     // R[A] = R[B] - R[C] as f32
    FMul = 0x17,     // R[A] = R[B] * R[C] as f32
    FDiv = 0x18,     // R[A] = R[B] / R[C] as f32
    
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
    JmpIfFLt = 0x35, // If R[A] < R[B] as f32, PC = C
    JmpIfFGt = 0x36, // If R[A] > R[B] as f32, PC = C

    Call = 0x40,     // Push PC to stack, PC = imm16
    Ret = 0x41,      // Pop PC from stack

    PrintReg = 0x50, // System call: Print R[A]

    Load = 0x60,     // R[A] = Memory[R[B] + R[C]]
    Store = 0x61,    // Memory[R[B] + R[C]] = R[A]

    Exp = 0x70,      // R[A] = exp_approx_q16(R[B])
    Rsqrt = 0x71,    // R[A] = rsqrt_approx_i32(R[B])
    Silu = 0x72,     // R[A] = silu_approx_i8(R[B])
    
    UiCall = 0xFE,   // UI System Call: Command=A, Arg1=B, Arg2=C
    NeuralCall = 0xFF,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
