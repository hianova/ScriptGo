use crate::instruction::Instruction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TraceStep {
    pub pc: u32,
    pub inst: u32,
    pub reg_change: Option<(u8, u32)>,
    pub mem_change: Option<(u16, u32)>,
}

impl Default for TraceStep {
    fn default() -> Self {
        Self {
            pc: 0,
            inst: 0,
            reg_change: None,
            mem_change: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmError {
    DivideByZero { pc: usize },
    StackOverflow { pc: usize },
    StackUnderflow { pc: usize },
    InvalidOpcode { pc: usize, opcode: u8 },
    MemoryAccessOutOfBounds { pc: usize, addr: usize },
    MathError { pc: usize },
    OutOfFuel { pc: usize },
}

pub struct ScriptVm {
    pub registers: [u32; 256],
    pub pc: usize,
    pub call_stack: [usize; 64],
    pub sp: usize,
    pub print_handler: Option<fn(u32)>,
    pub neural_handler: Option<fn(usize, usize, usize)>,
    pub ui_handler: Option<alloc::sync::Arc<dyn Fn(usize, usize, usize) + Send + Sync>>,
    pub abort_flag: Option<alloc::sync::Arc<core::sync::atomic::AtomicBool>>,
    pub debug_hook: Option<fn(&ScriptVm, Instruction)>,
    pub memory: [u8; 1024],
    pub max_steps: Option<u32>,
    pub tracing_enabled: bool,
    pub trace_buffer: [TraceStep; 1024],
    pub trace_head: usize,
    pub trace_count: usize,
    _tracker: no_std_tool::debug::ScopedResource,
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
            abort_flag: None,
            debug_hook: None,
            memory: [0; 1024],
            max_steps: Some(10000),
            tracing_enabled: false,
            trace_buffer: [TraceStep { pc: 0, inst: 0, reg_change: None, mem_change: None }; 1024],
            trace_head: 0,
            trace_count: 0,
            _tracker: no_std_tool::debug::ScopedResource::new(),
        }
    }

    /// Reset ephemeral execution context (PC, SP, call stack, R[0..16]) while preserving
    /// memory and persistent registers R[16..256] across code reloads (similar to React Fast Refresh).
    pub fn hot_reload(&mut self) {
        self.pc = 0;
        self.sp = 0;
        self.call_stack = [0; 64];
        for i in 0..16 {
            self.registers[i] = 0;
        }
    }

    /// Log a trace step to the circular trace buffer.
    #[inline(always)]
    fn log_trace(&mut self, pc: u32, inst: u32, reg_change: Option<(u8, u32)>, mem_change: Option<(u16, u32)>) {
        if self.tracing_enabled {
            let step = TraceStep {
                pc,
                inst,
                reg_change,
                mem_change,
            };
            self.trace_buffer[self.trace_head] = step;
            self.trace_head = (self.trace_head + 1) % 1024;
            if self.trace_count < 1024 {
                self.trace_count += 1;
            }
        }
    }

    /// Run the VM execution loop.
    /// Returns the number of instructions executed on success.
    #[inline(never)]
    pub fn run(&mut self, code: &[Instruction]) -> Result<u32, VmError> {
        if self.debug_hook.is_none() && self.abort_flag.is_none() && !self.tracing_enabled {
            self.run_fast(code)
        } else {
            self.run_slow(code)
        }
    }

    #[inline(never)]
    pub fn step_count_helper(&self) {
        let _ = self.pc;
    }

    #[inline(always)]
    fn run_fast(&mut self, code: &[Instruction]) -> Result<u32, VmError> {
        self.pc = 0;
        self.sp = 0;
        let mut steps = 0;
        let max_steps = self.max_steps.unwrap_or(u32::MAX);


        while self.pc < code.len() {
            if steps >= max_steps {
                return Err(VmError::OutOfFuel { pc: self.pc });
            }
            
            let inst = unsafe { *code.get_unchecked(self.pc) };
            self.pc += 1;
            steps += 1;
            self.step_count_helper();

            let opcode = inst.opcode();
            let a = inst.a();
            let b = inst.b();
            let c = inst.c();

            match opcode {
                0x00 => break, // Halt
                0x01 => {
                    self.registers[a] = b as u32;
                }
                0x02 => {
                    self.registers[a] = inst.imm16() as u32;
                }
                
                0x10 => {
                    self.registers[a] = self.registers[b].wrapping_add(self.registers[c]);
                }
                0x11 => {
                    self.registers[a] = self.registers[b].wrapping_sub(self.registers[c]);
                }
                0x12 => {
                    self.registers[a] = self.registers[b].wrapping_mul(self.registers[c]);
                }
                0x13 => {
                    let divisor = self.registers[c];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[b] / divisor;
                }
                0x14 => {
                    let divisor = self.registers[c];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[b] % divisor;
                }
                0x15 => { // FAdd
                    let b_val = f32::from_bits(self.registers[b]);
                    let c_val = f32::from_bits(self.registers[c]);
                    self.registers[a] = (b_val + c_val).to_bits();
                }
                0x16 => { // FSub
                    let b_val = f32::from_bits(self.registers[b]);
                    let c_val = f32::from_bits(self.registers[c]);
                    self.registers[a] = (b_val - c_val).to_bits();
                }
                0x17 => { // FMul
                    let b_val = f32::from_bits(self.registers[b]);
                    let c_val = f32::from_bits(self.registers[c]);
                    self.registers[a] = (b_val * c_val).to_bits();
                }
                0x18 => { // FDiv
                    let divisor = f32::from_bits(self.registers[c]);
                    if divisor == 0.0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let b_val = f32::from_bits(self.registers[b]);
                    self.registers[a] = (b_val / divisor).to_bits();
                }
                
                0x20 => {
                    self.registers[a] = self.registers[b] & self.registers[c];
                }
                0x21 => {
                    self.registers[a] = self.registers[b] | self.registers[c];
                }
                0x22 => {
                    self.registers[a] = self.registers[b] ^ self.registers[c];
                }
                0x23 => {
                    self.registers[a] = self.registers[b] << self.registers[c];
                }
                0x24 => {
                    self.registers[a] = self.registers[b] >> self.registers[c];
                }

                0x30 => self.pc = inst.imm16() as usize,
                0x31 => {
                    if self.registers[a] == 0 {
                        self.pc = inst.imm16() as usize;
                    }
                }
                0x32 => {
                    if self.registers[a] == self.registers[b] {
                        self.pc = c;
                    }
                }
                0x33 => {
                    if self.registers[a] < self.registers[b] {
                        self.pc = c;
                    }
                }
                0x34 => {
                    if self.registers[a] > self.registers[b] {
                        self.pc = c;
                    }
                }
                0x35 => { // JmpIfFLt
                    let a_val = f32::from_bits(self.registers[a]);
                    let b_val = f32::from_bits(self.registers[b]);
                    if a_val < b_val {
                        self.pc = c;
                    }
                }
                0x36 => { // JmpIfFGt
                    let a_val = f32::from_bits(self.registers[a]);
                    let b_val = f32::from_bits(self.registers[b]);
                    if a_val > b_val {
                        self.pc = c;
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
                        handler(self.registers[a]);
                    }
                }

                0x60 => { // Load
                    let addr = self.registers[b].wrapping_add(self.registers[c]) as usize;
                    if addr + 4 <= 1024 {
                        let mut val = 0u32;
                        for i in 0..4 {
                            val |= (self.memory[addr + i] as u32) << (i * 8);
                        }
                        self.registers[a] = val;
                    } else {
                        return Err(VmError::MemoryAccessOutOfBounds { pc: self.pc - 1, addr });
                    }
                }
                0x61 => { // Store
                    let addr = self.registers[b].wrapping_add(self.registers[c]) as usize;
                    if addr + 4 <= 1024 {
                        let val = self.registers[a];
                        for i in 0..4 {
                            self.memory[addr + i] = ((val >> (i * 8)) & 0xFF) as u8;
                        }
                    } else {
                        return Err(VmError::MemoryAccessOutOfBounds { pc: self.pc - 1, addr });
                    }
                }

                0x70 => { // Exp
                    let val = self.registers[b] as i32;
                    if let Some(res) = no_std_tool::math::exp_approx_q16(val) {
                        self.registers[a] = res as u32;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                0x71 => { // Rsqrt
                    let val = self.registers[b];
                    if let Some(res) = no_std_tool::math::rsqrt_approx_i32(val) {
                        self.registers[a] = res;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                0x72 => { // Silu
                    let val = (self.registers[b] & 0xFF) as i8;
                    if let Some(res) = no_std_tool::math::silu_approx_i8(val) {
                        self.registers[a] = (res as u32) & 0xFF;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                
                0xFF => { // NeuralCall
                    if let Some(handler) = self.neural_handler {
                        handler(a, b, c);
                    }
                }
                
                0xFE => { // UiCall
                    let cmd = b;
                    if a == 0 || cmd < 1 || cmd > 4 {
                        // Drop invalid payload silently
                    } else if let Some(ref handler) = self.ui_handler {
                        handler(a, b, c);
                    }
                }
                
                op => return Err(VmError::InvalidOpcode { pc: self.pc - 1, opcode: op }),
            }
        }
        Ok(steps)
    }

    #[inline(never)]
    fn run_slow(&mut self, code: &[Instruction]) -> Result<u32, VmError> {

        self.pc = 0;
        self.sp = 0;
        let mut steps = 0;
        
        while self.pc < code.len() {
            if let Some(ref abort) = self.abort_flag {
                if abort.load(core::sync::atomic::Ordering::Relaxed) {
                    break;
                }
            }

            if let Some(max) = self.max_steps {
                if steps >= max {
                    return Err(VmError::OutOfFuel { pc: self.pc });
                }
            }
            
            let current_pc = self.pc as u32;
            let inst = code[self.pc];
            if let Some(hook) = self.debug_hook {
                hook(self, inst);
            }

            self.pc += 1;
            steps += 1;

            let mut reg_change = None;
            let mut mem_change = None;

            match inst.opcode() {
                0x00 => break, // Halt
                0x01 => {
                    let val = inst.b() as u32;
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x02 => {
                    let val = inst.imm16() as u32;
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                
                0x10 => {
                    let val = self.registers[inst.b()].wrapping_add(self.registers[inst.c()]);
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x11 => {
                    let val = self.registers[inst.b()].wrapping_sub(self.registers[inst.c()]);
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x12 => {
                    let val = self.registers[inst.b()].wrapping_mul(self.registers[inst.c()]);
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x13 => {
                    let divisor = self.registers[inst.c()];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let val = self.registers[inst.b()] / divisor;
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x14 => {
                    let divisor = self.registers[inst.c()];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let val = self.registers[inst.b()] % divisor;
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x15 => { // FAdd
                    let b = f32::from_bits(self.registers[inst.b()]);
                    let c = f32::from_bits(self.registers[inst.c()]);
                    let val = (b + c).to_bits();
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x16 => { // FSub
                    let b = f32::from_bits(self.registers[inst.b()]);
                    let c = f32::from_bits(self.registers[inst.c()]);
                    let val = (b - c).to_bits();
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x17 => { // FMul
                    let b = f32::from_bits(self.registers[inst.b()]);
                    let c = f32::from_bits(self.registers[inst.c()]);
                    let val = (b * c).to_bits();
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x18 => { // FDiv
                    let divisor = f32::from_bits(self.registers[inst.c()]);
                    if divisor == 0.0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let b = f32::from_bits(self.registers[inst.b()]);
                    let val = (b / divisor).to_bits();
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                
                0x20 => {
                    let val = self.registers[inst.b()] & self.registers[inst.c()];
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x21 => {
                    let val = self.registers[inst.b()] | self.registers[inst.c()];
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x22 => {
                    let val = self.registers[inst.b()] ^ self.registers[inst.c()];
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x23 => {
                    let val = self.registers[inst.b()] << self.registers[inst.c()];
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }
                0x24 => {
                    let val = self.registers[inst.b()] >> self.registers[inst.c()];
                    self.registers[inst.a()] = val;
                    reg_change = Some((inst.a() as u8, val));
                }

                0x30 => self.pc = inst.imm16() as usize,
                0x31 => {
                    if self.registers[inst.a()] == 0 {
                        self.pc = inst.imm16() as usize;
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
                0x35 => { // JmpIfFLt
                    let a = f32::from_bits(self.registers[inst.a()]);
                    let b = f32::from_bits(self.registers[inst.b()]);
                    if a < b {
                        self.pc = inst.c();
                    }
                }
                0x36 => { // JmpIfFGt
                    let a = f32::from_bits(self.registers[inst.a()]);
                    let b = f32::from_bits(self.registers[inst.b()]);
                    if a > b {
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

                0x60 => { // Load
                    let addr = self.registers[inst.b()].wrapping_add(self.registers[inst.c()]) as usize;
                    if addr + 4 <= 1024 {
                        let mut val = 0u32;
                        for i in 0..4 {
                            val |= (self.memory[addr + i] as u32) << (i * 8);
                        }
                        self.registers[inst.a()] = val;
                        reg_change = Some((inst.a() as u8, val));
                    } else {
                        return Err(VmError::MemoryAccessOutOfBounds { pc: self.pc - 1, addr });
                    }
                }
                0x61 => { // Store
                    let addr = self.registers[inst.b()].wrapping_add(self.registers[inst.c()]) as usize;
                    if addr + 4 <= 1024 {
                        let val = self.registers[inst.a()];
                        for i in 0..4 {
                            self.memory[addr + i] = ((val >> (i * 8)) & 0xFF) as u8;
                        }
                        mem_change = Some((addr as u16, val));
                    } else {
                        return Err(VmError::MemoryAccessOutOfBounds { pc: self.pc - 1, addr });
                    }
                }

                0x70 => { // Exp
                    let val = self.registers[inst.b()] as i32;
                    if let Some(res) = no_std_tool::math::exp_approx_q16(val) {
                        let val_u32 = res as u32;
                        self.registers[inst.a()] = val_u32;
                        reg_change = Some((inst.a() as u8, val_u32));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                0x71 => { // Rsqrt
                    let val = self.registers[inst.b()];
                    if let Some(res) = no_std_tool::math::rsqrt_approx_i32(val) {
                        self.registers[inst.a()] = res;
                        reg_change = Some((inst.a() as u8, res));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                0x72 => { // Silu
                    let val = (self.registers[inst.b()] & 0xFF) as i8;
                    if let Some(res) = no_std_tool::math::silu_approx_i8(val) {
                        let val_u32 = (res as u32) & 0xFF;
                        self.registers[inst.a()] = val_u32;
                        reg_change = Some((inst.a() as u8, val_u32));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                
                0xFF => { // NeuralCall
                    if let Some(handler) = self.neural_handler {
                        handler(inst.a(), inst.b(), inst.c());
                    }
                }
                
                0xFE => { // UiCall
                    // FFI border verification: ID must be non-zero, and Command type must be within 1..=4.
                    let cmd = inst.b();
                    if inst.a() == 0 || cmd < 1 || cmd > 4 {
                        // Drop invalid payload silently on FFI boundary check error.
                    } else if let Some(ref handler) = self.ui_handler {
                        handler(inst.a(), inst.b(), inst.c());
                    }
                }
                
                op => return Err(VmError::InvalidOpcode { pc: self.pc - 1, opcode: op }),
            }

            self.log_trace(current_pc, inst.0, reg_change, mem_change);
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

    #[test]
    fn test_floats() {
        let mut vm = ScriptVm::new();
        // Load f32 values represented as raw bits
        let val1 = 3.5f32.to_bits();
        let val2 = 1.5f32.to_bits();
        
        // R[1] = val1
        // R[2] = val2
        // R[3] = R[1] + R[2] (FAdd)
        // R[4] = R[1] - R[2] (FSub)
        // R[5] = R[1] * R[2] (FMul)
        // R[6] = R[1] / R[2] (FDiv)
        vm.registers[1] = val1;
        vm.registers[2] = val2;
        
        let code = [
            Instruction::new(OpCode::FAdd as u8, 3, 1, 2),
            Instruction::new(OpCode::FSub as u8, 4, 1, 2),
            Instruction::new(OpCode::FMul as u8, 5, 1, 2),
            Instruction::new(OpCode::FDiv as u8, 6, 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        
        vm.run(&code).unwrap();
        
        assert_eq!(f32::from_bits(vm.registers[3]), 5.0f32);
        assert_eq!(f32::from_bits(vm.registers[4]), 2.0f32);
        assert_eq!(f32::from_bits(vm.registers[5]), 5.25f32);
        assert_eq!(f32::from_bits(vm.registers[6]), 3.5 / 1.5);
    }

    #[test]
    fn test_memory_load_store() {
        let mut vm = ScriptVm::new();
        // R[1] = 42 (value to store)
        // R[2] = 10 (base address)
        // R[3] = 4 (offset)
        // Store R[1] to Memory[R[2] + R[3]]
        // R[4] = Load from Memory[R[2] + R[3]]
        vm.registers[1] = 42;
        vm.registers[2] = 10;
        vm.registers[3] = 4;
        
        let code = [
            Instruction::new(OpCode::Store as u8, 1, 2, 3),
            Instruction::new(OpCode::Load as u8, 4, 2, 3),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        
        vm.run(&code).unwrap();
        
        assert_eq!(vm.registers[4], 42);
        // Verify bytes in memory (little endian)
        assert_eq!(vm.memory[14], 42);
        assert_eq!(vm.memory[15], 0);
        assert_eq!(vm.memory[16], 0);
        assert_eq!(vm.memory[17], 0);
    }

    #[test]
    fn test_math_approximations() {
        let mut vm = ScriptVm::new();
        // EXP: exp_approx_q16
        // R[1] = 0 (Q16.16)
        // RSQRT: rsqrt_approx_i32
        // R[2] = 4
        // SILU: silu_approx_i8
        // R[3] = 2
        vm.registers[1] = 0;
        vm.registers[2] = 4;
        vm.registers[3] = 2;
        
        let code = [
            Instruction::new(OpCode::Exp as u8, 4, 1, 0),
            Instruction::new(OpCode::Rsqrt as u8, 5, 2, 0),
            Instruction::new(OpCode::Silu as u8, 6, 3, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        
        vm.run(&code).unwrap();
        
        // exp(0) = 1.0 (Q16.16 -> 65536)
        assert_eq!(vm.registers[4], 65536);
        // rsqrt(4) = 1/sqrt(4) = 0.5 (Q16.16 -> 32768)
        assert_eq!(vm.registers[5], 32768);
        // silu(2) ≈ 2 * (1 / (1 + exp(-2)))
        // Silu approx of 2 is non-zero
        assert!(vm.registers[6] > 0);
    }

    #[test]
    fn test_abort_flag() {
        let mut vm = ScriptVm::new();
        vm.max_steps = None;
        let abort = alloc::sync::Arc::new(core::sync::atomic::AtomicBool::new(false));
        vm.abort_flag = Some(abort.clone());
        
        // Endless loop:
        // 0: JMP 0
        let code = [
            Instruction::new(OpCode::Jmp as u8, 0, 0, 0),
        ];
        
        let abort_clone = abort.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            abort_clone.store(true, core::sync::atomic::Ordering::Relaxed);
        });
        
        let result = vm.run(&code);
        assert!(result.is_ok());
        handle.join().unwrap();
    }

    #[test]
    fn test_out_of_fuel() {
        let mut vm = ScriptVm::new();
        vm.max_steps = Some(50);
        let code = [
            Instruction::new(OpCode::Jmp as u8, 0, 0, 0),
        ];
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::OutOfFuel { pc: 0 }));
    }

    #[test]
    fn test_trace_logging() {
        let mut vm = ScriptVm::new();
        vm.tracing_enabled = true;
        
        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, 42, 0),
            Instruction::new(OpCode::Store as u8, 1, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        
        vm.run(&code).unwrap();
        
        assert_eq!(vm.trace_count, 2);
        let trace1 = vm.trace_buffer[0];
        assert_eq!(trace1.pc, 0);
        assert_eq!(trace1.reg_change, Some((1, 42)));
        assert_eq!(trace1.mem_change, None);
        
        let trace2 = vm.trace_buffer[1];
        assert_eq!(trace2.pc, 1);
        assert_eq!(trace2.reg_change, None);
        assert_eq!(trace2.mem_change, Some((0, 42)));
    }

    #[test]
    fn test_debug_hook() {
        let mut vm = ScriptVm::new();
        use core::sync::atomic::{AtomicUsize, Ordering};
        static EXEC_COUNT: AtomicUsize = AtomicUsize::new(0);
        EXEC_COUNT.store(0, Ordering::Relaxed);
        
        vm.debug_hook = Some(|_vm, inst| {
            EXEC_COUNT.fetch_add(1, Ordering::Relaxed);
            if inst.opcode() == OpCode::LoadImm as u8 {
                assert_eq!(inst.a(), 1);
            }
        });
        
        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, 10, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        
        vm.run(&code).unwrap();
        assert_eq!(EXEC_COUNT.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_panic_recovery() {
        let mut vm = ScriptVm::new();
        vm.print_handler = Some(|_| {
            panic!("Mock handler panic!");
        });
        
        let code = [
            Instruction::new(OpCode::PrintReg as u8, 0, 0, 0),
        ];
        
        let vm_ref = &mut vm;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _ = vm_ref.run(&code);
        }));
        
        assert!(result.is_err());
    }

    #[test]
    fn test_hot_reload_state_preservation() {
        let mut vm = ScriptVm::new();
        // Set some ephemeral state
        vm.pc = 42;
        vm.sp = 5;
        vm.call_stack[0] = 99;
        vm.registers[3] = 77; // Ephemeral register
        
        // Set some persistent state
        vm.registers[20] = 88; // Persistent register
        vm.memory[10] = 55; // RAM
        
        vm.hot_reload();
        
        // Ephemeral state must be reset
        assert_eq!(vm.pc, 0);
        assert_eq!(vm.sp, 0);
        assert_eq!(vm.call_stack[0], 0);
        assert_eq!(vm.registers[3], 0);
        
        // Persistent state must be preserved
        assert_eq!(vm.registers[20], 88);
        assert_eq!(vm.memory[10], 55);
    }
}

