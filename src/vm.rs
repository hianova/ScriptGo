use crate::instruction::Instruction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmError {
    DivideByZero { pc: usize },
    StackOverflow { pc: usize },
    StackUnderflow { pc: usize },
    InvalidOpcode { pc: usize, opcode: u8 },
}

pub struct ScriptVm {
    pub registers: [u32; 256],
    pub pc: usize,
    pub call_stack: [usize; 64],
    pub sp: usize,
    pub print_handler: Option<fn(u32)>,
    pub neural_handler: Option<fn(usize, usize, usize)>,
    pub ui_handler: Option<alloc::sync::Arc<dyn Fn(usize, usize, usize) + Send + Sync>>,
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
            print_handler: None,
            neural_handler: None,
            ui_handler: None,
        }
    }

    /// Run the VM execution loop.
    /// Returns the number of instructions executed on success.
    #[inline(never)]
    pub fn run(&mut self, code: &[Instruction]) -> Result<u32, VmError> {
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
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[inst.a()] = self.registers[inst.b()] / divisor;
                }
                0x14 => {
                    let divisor = self.registers[inst.c()];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[inst.a()] = self.registers[inst.b()] % divisor;
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
                        return Err(VmError::StackOverflow { pc: self.pc - 1 });
                    }
                }
                0x41 => { // Ret
                    if self.sp > 0 {
                        self.sp -= 1;
                        self.pc = self.call_stack[self.sp];
                    } else {
                        return Err(VmError::StackUnderflow { pc: self.pc - 1 });
                    }
                }
                
                0x50 => { // PrintReg
                    if let Some(handler) = self.print_handler {
                        handler(self.registers[inst.a()]);
                    }
                }
                
                0xFF => { // NeuralCall
                    if let Some(handler) = self.neural_handler {
                        handler(inst.a(), inst.b(), inst.c());
                    }
                }
                
                0xFE => { // UiCall
                    if let Some(ref handler) = self.ui_handler {
                        handler(inst.a(), inst.b(), inst.c());
                    }
                }
                
                op => return Err(VmError::InvalidOpcode { pc: self.pc - 1, opcode: op }),
            }
        }
        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::OpCode;

    #[test]
    fn test_div_by_zero() {
        let mut vm = ScriptVm::new();
        // LOADIMM 1 10
        // LOADIMM 2 0
        // DIV 3 1 2
        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, 10, 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::Div as u8, 3, 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::DivideByZero { pc: 2 }));
    }

    #[test]
    fn test_stack_overflow() {
        let mut vm = ScriptVm::new();
        // CALL 0 (recursive call to itself)
        let code = [
            Instruction::new(OpCode::Call as u8, 0, 0, 0),
        ];
        
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackOverflow { pc: 0 }));
    }

    #[test]
    fn test_stack_underflow() {
        let mut vm = ScriptVm::new();
        // RET (no call pushed)
        let code = [
            Instruction::new(OpCode::Ret as u8, 0, 0, 0),
        ];
        
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackUnderflow { pc: 0 }));
    }

    #[test]
    fn test_invalid_opcode() {
        let mut vm = ScriptVm::new();
        let code = [
            Instruction::new(0x99, 0, 0, 0), // 0x99 is undefined
        ];
        
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::InvalidOpcode { pc: 0, opcode: 0x99 }));
    }
}

