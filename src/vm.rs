use crate::instruction::Instruction;

pub struct ScriptVm {
    pub registers: [u32; 256],
    pub pc: usize,
    pub call_stack: [usize; 64],
    pub sp: usize,
}

impl Default for ScriptVm {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptVm {
    pub fn new() -> Self {
        Self {
            registers: [0; 256],
            pc: 0,
            call_stack: [0; 64],
            sp: 0,
        }
    }

    /// Run the VM execution loop.
    /// Returns the number of instructions executed.
    #[inline(never)]
    pub fn run(&mut self, code: &[Instruction]) -> u32 {
        self.pc = 0;
        self.sp = 0;
        let mut steps = 0;
        
        while self.pc < code.len() {
            let inst = code[self.pc];
            self.pc += 1;
            steps += 1;

            match inst.opcode() {
                0x00 => break, // Halt
                0x01 => self.registers[inst.a()] = inst.b() as u32,
                0x02 => self.registers[inst.a()] = inst.imm16() as u32,
                
                0x10 => self.registers[inst.a()] = self.registers[inst.b()].wrapping_add(self.registers[inst.c()]),
                0x11 => self.registers[inst.a()] = self.registers[inst.b()].wrapping_sub(self.registers[inst.c()]),
                0x12 => self.registers[inst.a()] = self.registers[inst.b()].wrapping_mul(self.registers[inst.c()]),
                0x13 => {
                    let divisor = self.registers[inst.c()];
                    if divisor != 0 {
                        self.registers[inst.a()] = self.registers[inst.b()] / divisor;
                    }
                }
                0x14 => {
                    let divisor = self.registers[inst.c()];
                    if divisor != 0 {
                        self.registers[inst.a()] = self.registers[inst.b()] % divisor;
                    }
                }
                
                0x20 => self.registers[inst.a()] = self.registers[inst.b()] & self.registers[inst.c()],
                0x21 => self.registers[inst.a()] = self.registers[inst.b()] | self.registers[inst.c()],
                0x22 => self.registers[inst.a()] = self.registers[inst.b()] ^ self.registers[inst.c()],
                0x23 => self.registers[inst.a()] = self.registers[inst.b()] << self.registers[inst.c()],
                0x24 => self.registers[inst.a()] = self.registers[inst.b()] >> self.registers[inst.c()],

                0x30 => self.pc = inst.imm16() as usize,
                0x31 => {
                    if self.registers[inst.a()] == 0 {
                        self.pc = inst.b();
                    }
                }
                0x32 => {
                    if self.registers[inst.a()] == self.registers[inst.b()] {
                        self.pc = inst.c();
                    }
                }
                0x33 => {
                    if self.registers[inst.a()] < self.registers[inst.b()] {
                        self.pc = inst.c();
                    }
                }
                0x34 => {
                    if self.registers[inst.a()] > self.registers[inst.b()] {
                        self.pc = inst.c();
                    }
                }
                
                0x40 => { // Call
                    if self.sp < 64 {
                        self.call_stack[self.sp] = self.pc;
                        self.sp += 1;
                        self.pc = inst.imm16() as usize;
                    } else {
                        break; // Stack Overflow
                    }
                }
                0x41 => { // Ret
                    if self.sp > 0 {
                        self.sp -= 1;
                        self.pc = self.call_stack[self.sp];
                    } else {
                        break; // Stack Underflow
                    }
                }
                
                _ => {} // Ignore unknown opcodes
            }
        }
        steps
    }
}
