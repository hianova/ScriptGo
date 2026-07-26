#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use alloc::boxed::Box;
use crate::sgl::instruction::Instruction;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde :: Serialize, serde :: Deserialize)]
#[repr(C, align(64))]
pub struct TraceStep {
    pub pc: u32,
    pub inst: u32,
    pub reg_change: Option<(u8, u32)>,
    pub mem_change: Option<(u16, u32)>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmResult {
    Halted(u32),
    Yielded(u32),
    Spawn(u32, u16, u8),
    Awaiting(u32, u32, u8),
    MmapRequest(u32, u32),
}
#[inline(always)]
fn likely(b: bool) -> bool {
    b
}
#[inline(always)]
fn unlikely(b: bool) -> bool {
    b
}
pub type UiHandler = fn(usize, usize, usize);
#[repr(align(64))]
pub struct ScriptVm {
    pub registers: [u32; 256],
    pub pc: usize,
    pub call_stack: [usize; 64],
    pub sp: usize,
    pub print_handler: Option<fn(u32)>,
    pub neural_handler: Option<fn(&mut ScriptVm, usize, usize, usize)>,
    pub syscall_handler: Option<fn(u32, u32, u32)>,
    pub syscall_handlers: alloc::vec::Vec<fn(&mut ScriptVm, usize, usize, usize)>,
    pub ui_handler: Option<UiHandler>,
    pub hardware_handler: Option<fn(&mut ScriptVm, usize, usize, usize)>,
    pub hardware_handlers: alloc::vec::Vec<fn(&mut ScriptVm, usize, usize, usize)>,
    pub host_context: Option<crate::sgl::host_handlers::HostContext>,
    pub ui_dispatcher: Option<crate::sgl::ui_engine::UiDispatcher>,
    pub abort_flag: Option<fn() -> bool>,
    pub debug_hook: Option<fn(&ScriptVm, Instruction)>,
    pub memory: [u8; 1024],
    pub mmap_ptr: usize,
    pub mmap_len: usize,
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
    #[inline(always)]
    pub fn get_ptr(&self, addr: usize, len: usize) -> Result<*const u8, VmError> {
        if addr < covopt_param!("M_76_18", 1024) {
            if addr.checked_add(len).is_some_and(|end| end <= covopt_param!("M_77_62", 1024)) {
                Ok(self.memory.as_ptr().wrapping_add(addr))
            } else {
                Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
            }
        } else if addr >= covopt_param!("M_82_26", 2147483648) {
            let offset = addr - covopt_param!("M_83_32", 2147483648);
            if offset.checked_add(len).is_some_and(|end| end <= self.mmap_len) {
                Ok((self.mmap_ptr + offset) as *const u8)
            } else {
                Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
            }
        } else {
            Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
        }
    }

    #[inline(always)]
    pub fn get_mut_ptr(&mut self, addr: usize, len: usize) -> Result<*mut u8, VmError> {
        if addr < covopt_param!("M_96_18", 1024) {
            if addr.checked_add(len).is_some_and(|end| end <= covopt_param!("M_97_62", 1024)) {
                Ok(self.memory.as_mut_ptr().wrapping_add(addr))
            } else {
                Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
            }
        } else if addr >= covopt_param!("M_102_26", 2147483648) {
            let offset = addr - covopt_param!("M_103_32", 2147483648);
            if offset.checked_add(len).is_some_and(|end| end <= self.mmap_len) {
                Ok((self.mmap_ptr + offset) as *mut u8)
            } else {
                Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
            }
        } else {
            Err(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr })
        }
    }
    
    #[inline(always)]
    pub fn check_watchdog_timeout(&self, steps: u32) -> Result<(), VmError> {
        if let Some(limit) = self.max_steps
            && steps >= limit
        {
            return Err(VmError::OutOfFuel { pc: self.pc });
        }
        Ok(())
    }

    pub fn register_host_context(&mut self, context: crate::sgl::host_handlers::HostContext) {
        self.host_context = Some(context);
    }

    pub fn register_syscall_handler(&mut self, handler: fn(u32, u32, u32)) {
        self.syscall_handler = Some(handler);
    }

    pub fn register_syscall_handler_ext(&mut self, handler: fn(&mut ScriptVm, usize, usize, usize)) {
        if !self.syscall_handlers.contains(&handler) {
            self.syscall_handlers.push(handler);
        }
    }

    pub fn register_hardware_handler(&mut self, handler: fn(&mut ScriptVm, usize, usize, usize)) {
        if !self.hardware_handlers.contains(&handler) {
            self.hardware_handlers.push(handler);
        }
        self.hardware_handler = Some(handler);
    }

    pub fn register_print_handler(&mut self, handler: fn(u32)) {
        self.print_handler = Some(handler);
    }

    pub fn get_host_context(&self) -> Option<&crate::sgl::host_handlers::HostContext> {
        self.host_context.as_ref()
    }

    pub fn get_host_context_mut(&mut self) -> Option<&mut crate::sgl::host_handlers::HostContext> {
        self.host_context.as_mut()
    }

    pub fn register_ui_dispatcher(&mut self, dispatcher: crate::sgl::ui_engine::UiDispatcher) {
        self.ui_dispatcher = Some(dispatcher);
    }

    pub fn get_ui_dispatcher(&self) -> Option<&crate::sgl::ui_engine::UiDispatcher> {
        self.ui_dispatcher.as_ref()
    }

    pub fn get_ui_dispatcher_mut(&mut self) -> Option<&mut crate::sgl::ui_engine::UiDispatcher> {
        self.ui_dispatcher.as_mut()
    }

    pub fn read_bytes(&self, addr: usize, len: usize, until_null: bool) -> Result<alloc::vec::Vec<u8>, VmError> {
        if until_null {
            let mut result = alloc::vec::Vec::new();
            let mut cur = addr;
            loop {
                let ptr = self.get_ptr(cur, 1)?;
                let byte = unsafe { *ptr };
                if byte == 0 {
                    break;
                }
                result.push(byte);
                cur += 1;
            }
            Ok(result)
        } else {
            let ptr = self.get_ptr(addr, len)?;
            let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
            Ok(slice.to_vec())
        }
    }

    pub fn read_string(&self, addr: usize, max_len: Option<usize>) -> Result<alloc::string::String, VmError> {
        let limit = max_len.unwrap_or(covopt_param!("M_191_38", 1024));
        self.get_ptr(addr, 1)?;
        let mut bytes = alloc::vec::Vec::new();
        for cur in addr..addr.saturating_add(limit) {
            let ptr = match self.get_ptr(cur, 1) {
                Ok(p) => p,
                Err(_) => break,
            };
            let byte = unsafe { *ptr };
            if byte == 0 {
                break;
            }
            bytes.push(byte);
        }
        alloc::string::String::from_utf8(bytes)
            .map_err(|_| VmError::MemoryAccessOutOfBounds { pc: self.pc, addr })
    }

    pub fn write_bytes(&mut self, addr: usize, data: &[u8]) -> Result<(), VmError> {
        let ptr = self.get_mut_ptr(addr, data.len())?;
        unsafe {
            core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        }
        Ok(())
    }

    pub fn write_string(&mut self, addr: usize, text: &str, null_terminate: bool) -> Result<usize, VmError> {
        let bytes = text.as_bytes();
        let total_len = if null_terminate { bytes.len() + 1 } else { bytes.len() };
        let ptr = self.get_mut_ptr(addr, total_len)?;
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            if null_terminate {
                *ptr.add(bytes.len()) = 0;
            }
        }
        Ok(total_len)
    }
    pub fn new() -> Self {
        Self {
            registers: [0; 256],
            pc: 0,
            call_stack: [0; 64],
            sp: 0,
            print_handler: None,
            neural_handler: None,
            syscall_handler: None,
            syscall_handlers: alloc::vec::Vec::new(),
            ui_handler: None,
            hardware_handler: None,
            hardware_handlers: alloc::vec::Vec::new(),
            host_context: None,
            ui_dispatcher: None,
            abort_flag: None,
            debug_hook: None,
            memory: [0; 1024],
            mmap_ptr: 0,
            mmap_len: 0,
            max_steps: Some(covopt_param!("M_249_28", 10000)),
            tracing_enabled: false,
            trace_buffer: [TraceStep {
                pc: 0,
                inst: 0,
                reg_change: None,
                mem_change: None,
            }; 1024],
            trace_head: 0,
            trace_count: 0,
            _tracker: no_std_tool::debug::ScopedResource::new(),
        }
    }

    pub fn new_with_max_steps(max_steps: u32) -> Self {
        let mut vm = Self::new();
        vm.max_steps = Some(max_steps);
        vm
    }

    pub fn new_boxed() -> Box<Self> {
        unsafe {
            let layout = core::alloc::Layout::new::<Self>();
            let ptr = alloc::alloc::alloc_zeroed(layout) as *mut Self;
            if ptr.is_null() {
                alloc::alloc::handle_alloc_error(layout);
            }
            core::ptr::addr_of_mut!((*ptr).max_steps).write(Some(covopt_param!("M_276_65", 10000)));
            core::ptr::addr_of_mut!((*ptr)._tracker).write(no_std_tool::debug::ScopedResource::new());
            Box::from_raw(ptr)
        }
    }

    pub fn new_boxed_with_max_steps(max_steps: u32) -> Box<Self> {
        let mut vm = Self::new_boxed();
        vm.max_steps = Some(max_steps);
        vm
    }
    #[doc = " Reset ephemeral execution context (PC, SP, call stack, R[0..16]) while preserving"]
    #[doc = " memory and persistent registers R[16..256] across code reloads (similar to React Fast Refresh)."]
    pub fn hot_reload(&mut self) {
        self.pc = 0;
        self.sp = 0;
        self.call_stack = [0; 64];
        for i in 0..covopt_param!("M_293_20", 16) {
            self.registers[i] = 0;
        }
    }
    #[doc = " Log a trace step to the circular trace buffer."]
    #[inline(always)]
    fn log_trace(
        &mut self,
        pc: u32,
        inst: u32,
        reg_change: Option<(u8, u32)>,
        mem_change: Option<(u16, u32)>,
    ) {
        if self.tracing_enabled {
            let step = TraceStep {
                pc,
                inst,
                reg_change,
                mem_change,
            };
            self.trace_buffer[self.trace_head] = step;
            self.trace_head = (self.trace_head + 1) % covopt_param!("M_314_54", 1024);
            if self.trace_count < covopt_param!("M_315_34", 1024) {
                self.trace_count += 1;
            }
        }
    }
    #[doc = " Run the VM execution loop."]
    #[doc = " Returns the number of instructions executed on success."]
    #[inline(always)]
    pub fn run(&mut self, code: &[Instruction]) -> Result<VmResult, VmError> {
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
    pub fn run_fast(&mut self, code: &[Instruction]) -> Result<VmResult, VmError> {
        // self.pc is intentionally NOT reset to 0 here to support resuming from Yield
        if self.pc == 0 {
            self.sp = 0;
        }
        let mut steps = 0;
        let max_steps = self.max_steps.unwrap_or(u32::MAX);
        let poll_mask = covopt_param!("M_342_24", 255);
        loop {
            if (steps & poll_mask) == 0 {
                if unlikely(steps >= max_steps) {
                    return Err(VmError::OutOfFuel { pc: self.pc });
                }
                if unlikely(self.check_watchdog_timeout(steps).is_err()) {
                    return Err(VmError::OutOfFuel { pc: self.pc });
                }
            }
            if unlikely(self.pc >= code.len()) {
                break;
            }
            let inst = unsafe { *code.get_unchecked(self.pc) };
            self.pc += 1;
            steps += 1;
            let opcode = crate::opcode!(inst);
            match opcode {
                0 => break,
                1 => {
                    let a = crate::inst_a!(inst);
                    unsafe { *self.registers.get_unchecked_mut(a) = crate::inst_b!(inst) as u32; }
                }
                2 => {
                    let a = crate::inst_a!(inst);
                    unsafe { *self.registers.get_unchecked_mut(a) = crate::inst_imm16!(inst) as u32; }
                }
                3 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)]);
                }
                4 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_sub(self.registers[crate::inst_c!(inst)]);
                }
                5 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_mul(self.registers[crate::inst_c!(inst)]);
                }
                6 => {
                    let a = crate::inst_a!(inst);
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[crate::inst_b!(inst)] / divisor;
                }
                7 => {
                    let a = crate::inst_a!(inst);
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[crate::inst_b!(inst)] % divisor;
                }
                8 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val + c_val).to_bits();
                }
                9 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val - c_val).to_bits();
                }
                10 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val * c_val).to_bits();
                }
                11 => {
                    let divisor = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    if divisor == 0.0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val / divisor).to_bits();
                }
                12 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] & self.registers[crate::inst_c!(inst)];
                }
                13 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] | self.registers[crate::inst_c!(inst)];
                }
                14 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] ^ self.registers[crate::inst_c!(inst)];
                }
                15 => {
                    self.registers[crate::inst_a!(inst)] = self.registers[crate::inst_b!(inst)]
                        << self.registers[crate::inst_c!(inst)];
                }
                16 => {
                    self.registers[crate::inst_a!(inst)] = self.registers[crate::inst_b!(inst)]
                        >> self.registers[crate::inst_c!(inst)];
                }
                17 => {
                    self.registers[crate::inst_a!(inst)] = if self.registers[crate::inst_b!(inst)]
                        == self.registers[crate::inst_c!(inst)]
                    {
                        1
                    } else {
                        0
                    };
                }
                18 => {
                    self.registers[crate::inst_a!(inst)] = if self.registers[crate::inst_b!(inst)]
                        < self.registers[crate::inst_c!(inst)]
                    {
                        1
                    } else {
                        0
                    };
                }
                19 => {
                    self.pc = crate::inst_imm16!(inst) as usize;
                }
                20 => {
                    if self.registers[crate::inst_a!(inst)] == 0 {
                        self.pc = crate::inst_b!(inst);
                    }
                }
                21 => {
                    if self.registers[crate::inst_a!(inst)] == self.registers[crate::inst_b!(inst)]
                    {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                22 => {
                    if self.registers[crate::inst_a!(inst)] < self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                23 => {
                    if self.registers[crate::inst_a!(inst)] > self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                24 => {
                    let a_val = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a_val < b_val {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                25 => {
                    let a_val = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a_val > b_val {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                26 => {
                    if self.sp < covopt_param!("M_500_33", 64) {
                        self.call_stack[self.sp] = self.pc;
                        self.sp += 1;
                        self.pc = crate::inst_imm16!(inst) as usize;
                    } else {
                        return Err(VmError::StackOverflow { pc: self.pc - 1 });
                    }
                }
                27 => {
                    if self.sp > 0 {
                        self.sp -= 1;
                        self.pc = self.call_stack[self.sp];
                    } else {
                        return Err(VmError::StackUnderflow { pc: self.pc - 1 });
                    }
                }
                28 => {
                    if let Some(handler) = self.print_handler {
                        handler(self.registers[crate::inst_a!(inst)]);
                    }
                }
                29 => {
                    let a = crate::inst_a!(inst);
                    let b = crate::inst_b!(inst);
                    let c = crate::inst_c!(inst);
                    let dest_idx = a % covopt_param!("M_525_39", 256);
                    let init_val = self.registers[dest_idx];

                    if self.host_context.is_some() {
                        let mut context = self.host_context.take().unwrap();
                        context.dispatch_syscall(self, a, b, c);
                        self.host_context = Some(context);
                    }
                    if self.registers[dest_idx] == init_val {
                        for handler in self.syscall_handlers.clone() {
                            handler(self, a, b, c);
                            if self.registers[dest_idx] != init_val {
                                break;
                            }
                        }
                    }
                    if let Some(handler) = self.syscall_handler {
                        handler(
                            self.registers[a],
                            self.registers[b],
                            self.registers[c],
                        );
                    }
                }
                30 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_ptr(addr, covopt_param!("M_553_49", 4))?;
                    let mut val = 0u32;
                    unsafe {
                        for i in 0..covopt_param!("M_556_36", 4) {
                            val |= (*ptr.add(i) as u32) << (i * covopt_param!("M_557_64", 8));
                        }
                    }
                    self.registers[crate::inst_a!(inst)] = val;
                }
                31 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_mut_ptr(addr, covopt_param!("M_566_53", 4))?;
                    let val = self.registers[crate::inst_a!(inst)];
                    unsafe {
                        for i in 0..covopt_param!("M_569_36", 4) {
                            *ptr.add(i) = ((val >> (i * covopt_param!("M_570_56", 8))) & covopt_param!("M_570_62", 255)) as u8;
                        }
                    }
                }
                32 => {
                    let val = self.registers[crate::inst_b!(inst)] as i32;
                    if let Some(res) = no_std_tool::math::exp_approx_q16(val) {
                        self.registers[crate::inst_a!(inst)] = res as u32;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                33 => {
                    let val = self.registers[crate::inst_b!(inst)];
                    if let Some(res) = no_std_tool::math::rsqrt_approx_i32(val) {
                        self.registers[crate::inst_a!(inst)] = res;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                34 => {
                    let val = (self.registers[crate::inst_b!(inst)] & covopt_param!("M_591_70", 255)) as i8;
                    if let Some(res) = no_std_tool::math::silu_approx_i8(val) {
                        self.registers[crate::inst_a!(inst)] = (res as u32) & covopt_param!("M_593_78", 255);
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                35 => {
                    let a = crate::inst_a!(inst);
                    let b = crate::inst_b!(inst);
                    let c = crate::inst_c!(inst);
                    let dest_idx = a % covopt_param!("M_602_39", 256);
                    let init_val = self.registers[dest_idx];
                    if self.host_context.is_some() {
                        let mut context = self.host_context.take().unwrap();
                        context.dispatch_hardware_call(self, a, b, c);
                        self.host_context = Some(context);
                    }
                    if self.registers[dest_idx] == init_val {
                        for handler in self.hardware_handlers.clone() {
                            handler(self, a, b, c);
                            if self.registers[dest_idx] != init_val {
                                break;
                            }
                        }
                        if self.registers[dest_idx] == init_val
                            && let Some(handler) = self.hardware_handler
                        {
                            handler(self, a, b, c);
                        }
                    }
                }
                36 => {
                    let a = crate::inst_a!(inst);
                    let b = crate::inst_b!(inst);
                    let c = crate::inst_c!(inst);
                    if self.ui_dispatcher.is_some() {
                        let mut dispatcher = self.ui_dispatcher.take().unwrap();
                        let _ = dispatcher.dispatch(self, a, b, c);
                        self.ui_dispatcher = Some(dispatcher);
                    }
                    if let Some(handler) = self.ui_handler {
                        handler(a, b, c);
                    }
                }
                37 => {
                    let handler = self.neural_handler;
                    if let Some(h) = handler {
                        h(
                            self,
                            crate::inst_a!(inst),
                            crate::inst_b!(inst),
                            crate::inst_c!(inst),
                        );
                    }
                }
                38 => {
                    return Ok(VmResult::Yielded(steps));
                }
                39 => { // VecAdd
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let byte_len = len.checked_mul(covopt_param!("M_655_51", 4)).ok_or(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr: dest })?;
                    let dest_ptr = self.get_mut_ptr(dest, byte_len)?;
                    let src1_ptr = self.get_ptr(src1, byte_len)?;
                    let src2_ptr = self.get_ptr(src2, byte_len)?;
                    unsafe {
                        crate::sgl::simd_ops::simd_vec_add(len, src1_ptr, src2_ptr, dest_ptr);
                    }
                }
                40 => { // VecMul
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let byte_len = len.checked_mul(covopt_param!("M_668_51", 4)).ok_or(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr: dest })?;
                    let dest_ptr = self.get_mut_ptr(dest, byte_len)?;
                    let src1_ptr = self.get_ptr(src1, byte_len)?;
                    let src2_ptr = self.get_ptr(src2, byte_len)?;
                    unsafe {
                        crate::sgl::simd_ops::simd_vec_mul(len, src1_ptr, src2_ptr, dest_ptr);
                    }
                }
                41 => { // VecDot
                    let len = self.registers[0] as usize;
                    let dest_reg = crate::inst_a!(inst);
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let byte_len = len.checked_mul(covopt_param!("M_681_51", 4)).ok_or(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr: src1 })?;
                    let src1_ptr = self.get_ptr(src1, byte_len)?;
                    let src2_ptr = self.get_ptr(src2, byte_len)?;
                    unsafe {
                        let sum = crate::sgl::simd_ops::simd_vec_dot(len, src1_ptr, src2_ptr);
                        self.registers[dest_reg] = sum.to_bits();
                    }
                }
                42 => { // Spawn
                    let target_pc = crate::inst_imm16!(inst) as usize;
                    return Ok(VmResult::Spawn(steps, target_pc as u16, crate::inst_a!(inst) as u8));
                }
                43 => { // Await
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::Awaiting(steps, resource_id, crate::inst_a!(inst) as u8));
                }
                44 => { // Mmap
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::MmapRequest(steps, resource_id));
                }
                45 => {
                    let dst = crate::inst_a!(inst);
                    let src = self.registers[crate::inst_b!(inst)];
                    let state = crate::inst_c!(inst);
                    let last = self.registers[state];
                    let delta = (src as i32).wrapping_sub(last as i32);
                    self.registers[state] = src;
                    self.registers[dst] = no_std_tool::compress::zigzag_encode_i32(delta);
                }
                46 => {
                    let dst = crate::inst_a!(inst);
                    let src = self.registers[crate::inst_b!(inst)];
                    let state = crate::inst_c!(inst);
                    let last = self.registers[state];
                    let delta = no_std_tool::compress::zigzag_decode_u32(src);
                    let current = (last as i32).wrapping_add(delta);
                    self.registers[state] = current as u32;
                    self.registers[dst] = current as u32;
                }
                _ => {
                    return Err(VmError::InvalidOpcode {
                        pc: self.pc - 1,
                        opcode,
                    });
                }
            }
        }
        Ok(VmResult::Halted(steps))
    }
    #[inline(always)]
    pub fn run_fast_with<N>(&mut self, code: &[Instruction], mut neural_handler: N) -> Result<VmResult, VmError>
    where
        N: FnMut(&mut ScriptVm, usize, usize, usize),
    {
        // self.pc is NOT reset to 0 to support Yield
        if self.pc == 0 {
            self.sp = 0;
        }
        let mut steps = 0;
        let max_steps = self.max_steps.unwrap_or(u32::MAX);
        let poll_mask = covopt_param!("M_741_24", 255);
        loop {
            if (steps & poll_mask) == 0 {
                if unlikely(steps >= max_steps) {
                    return Err(VmError::OutOfFuel { pc: self.pc });
                }
                if unlikely(self.check_watchdog_timeout(steps).is_err()) {
                    return Err(VmError::OutOfFuel { pc: self.pc });
                }
            }
            if unlikely(self.pc >= code.len()) {
                break;
            }
            let inst = unsafe { *code.get_unchecked(self.pc) };
            self.pc += 1;
            steps += 1;
            let opcode = crate::opcode!(inst);
            match opcode {
                0 => break,
                1 => {
                    let a = crate::inst_a!(inst);
                    unsafe { *self.registers.get_unchecked_mut(a) = crate::inst_b!(inst) as u32; }
                }
                2 => {
                    let a = crate::inst_a!(inst);
                    unsafe { *self.registers.get_unchecked_mut(a) = crate::inst_imm16!(inst) as u32; }
                }
                3 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)]);
                }
                4 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_sub(self.registers[crate::inst_c!(inst)]);
                }
                5 => {
                    let a = crate::inst_a!(inst);
                    self.registers[a] = self.registers[crate::inst_b!(inst)]
                        .wrapping_mul(self.registers[crate::inst_c!(inst)]);
                }
                6 => {
                    let a = crate::inst_a!(inst);
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[crate::inst_b!(inst)] / divisor;
                }
                7 => {
                    let a = crate::inst_a!(inst);
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    self.registers[a] = self.registers[crate::inst_b!(inst)] % divisor;
                }
                8 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val + c_val).to_bits();
                }
                9 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val - c_val).to_bits();
                }
                10 => {
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c_val = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val * c_val).to_bits();
                }
                11 => {
                    let divisor = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    if divisor == 0.0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    self.registers[crate::inst_a!(inst)] = (b_val / divisor).to_bits();
                }
                12 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] & self.registers[crate::inst_c!(inst)];
                }
                13 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] | self.registers[crate::inst_c!(inst)];
                }
                14 => {
                    self.registers[crate::inst_a!(inst)] =
                        self.registers[crate::inst_b!(inst)] ^ self.registers[crate::inst_c!(inst)];
                }
                15 => {
                    self.registers[crate::inst_a!(inst)] = self.registers[crate::inst_b!(inst)]
                        << self.registers[crate::inst_c!(inst)];
                }
                16 => {
                    self.registers[crate::inst_a!(inst)] = self.registers[crate::inst_b!(inst)]
                        >> self.registers[crate::inst_c!(inst)];
                }
                17 => {
                    self.registers[crate::inst_a!(inst)] = if self.registers[crate::inst_b!(inst)]
                        == self.registers[crate::inst_c!(inst)]
                    {
                        1
                    } else {
                        0
                    };
                }
                18 => {
                    self.registers[crate::inst_a!(inst)] = if self.registers[crate::inst_b!(inst)]
                        < self.registers[crate::inst_c!(inst)]
                    {
                        1
                    } else {
                        0
                    };
                }
                19 => {
                    self.pc = crate::inst_imm16!(inst) as usize;
                }
                20 => {
                    if self.registers[crate::inst_a!(inst)] == 0 {
                        self.pc = crate::inst_b!(inst);
                    }
                }
                21 => {
                    if self.registers[crate::inst_a!(inst)] == self.registers[crate::inst_b!(inst)]
                    {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                22 => {
                    if self.registers[crate::inst_a!(inst)] < self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                23 => {
                    if self.registers[crate::inst_a!(inst)] > self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                24 => {
                    let a_val = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a_val < b_val {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                25 => {
                    let a_val = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b_val = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a_val > b_val {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                26 => {
                    if self.sp < covopt_param!("M_899_33", 64) {
                        self.call_stack[self.sp] = self.pc;
                        self.sp += 1;
                        self.pc = crate::inst_imm16!(inst) as usize;
                    } else {
                        return Err(VmError::StackOverflow { pc: self.pc - 1 });
                    }
                }
                27 => {
                    if self.sp > 0 {
                        self.sp -= 1;
                        self.pc = self.call_stack[self.sp];
                    } else {
                        return Err(VmError::StackUnderflow { pc: self.pc - 1 });
                    }
                }
                28 => {
                    if let Some(handler) = self.print_handler {
                        handler(self.registers[crate::inst_a!(inst)]);
                    }
                }
                29 => {
                    let a = crate::inst_a!(inst);
                    let b = crate::inst_b!(inst);
                    let c = crate::inst_c!(inst);
                    let dest_idx = a % covopt_param!("M_924_39", 256);
                    let init_val = self.registers[dest_idx];

                    if self.host_context.is_some() {
                        let mut context = self.host_context.take().unwrap();
                        context.dispatch_syscall(self, a, b, c);
                        self.host_context = Some(context);
                    }
                    if self.registers[dest_idx] == init_val {
                        for handler in self.syscall_handlers.clone() {
                            handler(self, a, b, c);
                            if self.registers[dest_idx] != init_val {
                                break;
                            }
                        }
                    }
                    if let Some(handler) = self.syscall_handler {
                        handler(
                            self.registers[a],
                            self.registers[b],
                            self.registers[c],
                        );
                    }
                }
                30 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_ptr(addr, covopt_param!("M_952_49", 4))?;
                    let mut val = 0u32;
                    unsafe {
                        for i in 0..covopt_param!("M_955_36", 4) {
                            val |= (*ptr.add(i) as u32) << (i * covopt_param!("M_956_64", 8));
                        }
                    }
                    self.registers[crate::inst_a!(inst)] = val;
                }
                31 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_mut_ptr(addr, covopt_param!("M_965_53", 4))?;
                    let val = self.registers[crate::inst_a!(inst)];
                    unsafe {
                        for i in 0..covopt_param!("M_968_36", 4) {
                            *ptr.add(i) = ((val >> (i * covopt_param!("M_969_56", 8))) & covopt_param!("M_969_62", 255)) as u8;
                        }
                    }
                }
                32 => {
                    let val = self.registers[crate::inst_b!(inst)] as i32;
                    if let Some(res) = no_std_tool::math::exp_approx_q16(val) {
                        self.registers[crate::inst_a!(inst)] = res as u32;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                33 => {
                    let val = self.registers[crate::inst_b!(inst)];
                    if let Some(res) = no_std_tool::math::rsqrt_approx_i32(val) {
                        self.registers[crate::inst_a!(inst)] = res;
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                34 => {
                    let val = (self.registers[crate::inst_b!(inst)] & covopt_param!("M_990_70", 255)) as i8;
                    if let Some(res) = no_std_tool::math::silu_approx_i8(val) {
                        self.registers[crate::inst_a!(inst)] = (res as u32) & covopt_param!("M_992_78", 255);
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                35 => {
                    let a = crate::inst_a!(inst);
                    let b = crate::inst_b!(inst);
                    let c = crate::inst_c!(inst);
                    let dest_idx = a % covopt_param!("M_1001_39", 256);
                    let init_val = self.registers[dest_idx];

                    if self.host_context.is_some() {
                        let mut context = self.host_context.take().unwrap();
                        context.dispatch_hardware_call(self, a, b, c);
                        self.host_context = Some(context);
                    }
                    if self.registers[dest_idx] == init_val {
                        for handler in self.hardware_handlers.clone() {
                            handler(self, a, b, c);
                            if self.registers[dest_idx] != init_val {
                                break;
                            }
                        }
                        if self.registers[dest_idx] == init_val
                            && let Some(handler) = self.hardware_handler
                        {
                            handler(self, a, b, c);
                        }
                    }
                }
                36 => {
                    let a = crate::inst_a!(inst);
                    let b = crate::inst_b!(inst);
                    let c = crate::inst_c!(inst);
                    if self.ui_dispatcher.is_some() {
                        let mut dispatcher = self.ui_dispatcher.take().unwrap();
                        let _ = dispatcher.dispatch(self, a, b, c);
                        self.ui_dispatcher = Some(dispatcher);
                    }
                    if let Some(handler) = self.ui_handler {
                        handler(a, b, c);
                    }
                }
                37 => {
                    neural_handler(
                        self,
                        crate::inst_a!(inst),
                        crate::inst_b!(inst),
                        crate::inst_c!(inst),
                    );
                }
                38 => {
                    return Ok(VmResult::Yielded(steps));
                }
                39 => { // VecAdd
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let byte_len = len.checked_mul(covopt_param!("M_1052_51", 4)).ok_or(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr: dest })?;
                    let dest_ptr = self.get_mut_ptr(dest, byte_len)?;
                    let src1_ptr = self.get_ptr(src1, byte_len)?;
                    let src2_ptr = self.get_ptr(src2, byte_len)?;
                    unsafe {
                        crate::sgl::simd_ops::simd_vec_add(len, src1_ptr, src2_ptr, dest_ptr);
                    }
                }
                40 => { // VecMul
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let byte_len = len.checked_mul(covopt_param!("M_1065_51", 4)).ok_or(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr: dest })?;
                    let dest_ptr = self.get_mut_ptr(dest, byte_len)?;
                    let src1_ptr = self.get_ptr(src1, byte_len)?;
                    let src2_ptr = self.get_ptr(src2, byte_len)?;
                    unsafe {
                        crate::sgl::simd_ops::simd_vec_mul(len, src1_ptr, src2_ptr, dest_ptr);
                    }
                }
                41 => { // VecDot
                    let len = self.registers[0] as usize;
                    let dest_reg = crate::inst_a!(inst);
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let byte_len = len.checked_mul(covopt_param!("M_1078_51", 4)).ok_or(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr: src1 })?;
                    let src1_ptr = self.get_ptr(src1, byte_len)?;
                    let src2_ptr = self.get_ptr(src2, byte_len)?;
                    unsafe {
                        let sum = crate::sgl::simd_ops::simd_vec_dot(len, src1_ptr, src2_ptr);
                        self.registers[dest_reg] = sum.to_bits();
                    }
                }
                42 => { // Spawn
                    let target_pc = crate::inst_imm16!(inst) as usize;
                    return Ok(VmResult::Spawn(steps, target_pc as u16, crate::inst_a!(inst) as u8));
                }
                43 => { // Await
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::Awaiting(steps, resource_id, crate::inst_a!(inst) as u8));
                }
                44 => { // Mmap
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::MmapRequest(steps, resource_id));
                }
                45 => {
                    let dst = crate::inst_a!(inst);
                    let src = self.registers[crate::inst_b!(inst)];
                    let state = crate::inst_c!(inst);
                    let last = self.registers[state];
                    let delta = (src as i32).wrapping_sub(last as i32);
                    self.registers[state] = src;
                    self.registers[dst] = no_std_tool::compress::zigzag_encode_i32(delta);
                }
                46 => {
                    let dst = crate::inst_a!(inst);
                    let src = self.registers[crate::inst_b!(inst)];
                    let state = crate::inst_c!(inst);
                    let last = self.registers[state];
                    let delta = no_std_tool::compress::zigzag_decode_u32(src);
                    let current = (last as i32).wrapping_add(delta);
                    self.registers[state] = current as u32;
                    self.registers[dst] = current as u32;
                }
                _ => {
                    return Err(VmError::InvalidOpcode {
                        pc: self.pc - 1,
                        opcode,
                    });
                }
            }
        }
        Ok(VmResult::Halted(steps))
    }

    #[inline(never)]
    fn run_slow(&mut self, code: &[Instruction]) -> Result<VmResult, VmError> {
        // self.pc is NOT reset to 0 to support Yield
        if self.pc == 0 {
            self.sp = 0;
        }
        let mut steps = 0;
        while likely(self.pc < code.len()) {
            if let Some(abort) = self.abort_flag
                && unlikely(abort())
            {
                break;
            }
            if unlikely(self.max_steps.is_some() && steps >= self.max_steps.unwrap_or(u32::MAX)) {
                return Err(VmError::OutOfFuel { pc: self.pc });
            }
            if unlikely(self.check_watchdog_timeout(steps).is_err()) {
                return Err(VmError::OutOfFuel { pc: self.pc });
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
            match crate::opcode!(inst) {
                0 => break,
                1 => {
                    let val = crate::inst_b!(inst) as u32;
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                2 => {
                    let val = crate::inst_imm16!(inst) as u32;
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                3 => {
                    let val = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                4 => {
                    let val = self.registers[crate::inst_b!(inst)]
                        .wrapping_sub(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                5 => {
                    let val = self.registers[crate::inst_b!(inst)]
                        .wrapping_mul(self.registers[crate::inst_c!(inst)]);
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                6 => {
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let val = self.registers[crate::inst_b!(inst)] / divisor;
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                7 => {
                    let divisor = self.registers[crate::inst_c!(inst)];
                    if divisor == 0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let val = self.registers[crate::inst_b!(inst)] % divisor;
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                8 => {
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    let val = (b + c).to_bits();
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                9 => {
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    let val = (b - c).to_bits();
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                10 => {
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let c = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    let val = (b * c).to_bits();
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                11 => {
                    let divisor = f32::from_bits(self.registers[crate::inst_c!(inst)]);
                    if divisor == 0.0 {
                        return Err(VmError::DivideByZero { pc: self.pc - 1 });
                    }
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    let val = (b / divisor).to_bits();
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                12 => {
                    let val =
                        self.registers[crate::inst_b!(inst)] & self.registers[crate::inst_c!(inst)];
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                13 => {
                    let val =
                        self.registers[crate::inst_b!(inst)] | self.registers[crate::inst_c!(inst)];
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                14 => {
                    let val =
                        self.registers[crate::inst_b!(inst)] ^ self.registers[crate::inst_c!(inst)];
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                15 => {
                    let val = self.registers[crate::inst_b!(inst)]
                        << self.registers[crate::inst_c!(inst)];
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                16 => {
                    let val = self.registers[crate::inst_b!(inst)]
                        >> self.registers[crate::inst_c!(inst)];
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                19 => self.pc = crate::inst_imm16!(inst) as usize,
                20 => {
                    if self.registers[crate::inst_a!(inst)] == 0 {
                        self.pc = crate::inst_imm16!(inst) as usize;
                    }
                }
                21 => {
                    if self.registers[crate::inst_a!(inst)] == self.registers[crate::inst_b!(inst)]
                    {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                22 => {
                    if self.registers[crate::inst_a!(inst)] < self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                23 => {
                    if self.registers[crate::inst_a!(inst)] > self.registers[crate::inst_b!(inst)] {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                24 => {
                    let a = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a < b {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                25 => {
                    let a = f32::from_bits(self.registers[crate::inst_a!(inst)]);
                    let b = f32::from_bits(self.registers[crate::inst_b!(inst)]);
                    if a > b {
                        self.pc = crate::inst_c!(inst);
                    }
                }
                26 => {
                    if self.sp < covopt_param!("M_1302_33", 64) {
                        self.call_stack[self.sp] = self.pc;
                        self.sp += 1;
                        self.pc = crate::inst_imm16!(inst) as usize;
                    } else {
                        return Err(VmError::StackOverflow { pc: self.pc - 1 });
                    }
                }
                27 => {
                    if self.sp > 0 {
                        self.sp -= 1;
                        self.pc = self.call_stack[self.sp];
                    } else {
                        return Err(VmError::StackUnderflow { pc: self.pc - 1 });
                    }
                }
                28 => {
                    if let Some(handler) = self.print_handler {
                        handler(self.registers[crate::inst_a!(inst)]);
                    }
                }
                29 => {
                    let a = crate::inst_a!(inst);
                    let b = crate::inst_b!(inst);
                    let c = crate::inst_c!(inst);
                    let dest_idx = a % covopt_param!("M_1327_39", 256);
                    let init_val = self.registers[dest_idx];

                    if self.host_context.is_some() {
                        let mut context = self.host_context.take().unwrap();
                        context.dispatch_syscall(self, a, b, c);
                        self.host_context = Some(context);
                    }
                    if self.registers[dest_idx] == init_val {
                        for handler in self.syscall_handlers.clone() {
                            handler(self, a, b, c);
                            if self.registers[dest_idx] != init_val {
                                break;
                            }
                        }
                    }
                    if let Some(handler) = self.syscall_handler {
                        handler(
                            self.registers[a],
                            self.registers[b],
                            self.registers[c],
                        );
                    }
                    reg_change = Some((a as u8, self.registers[a]));
                }
                30 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_ptr(addr, covopt_param!("M_1356_49", 4))?;
                    let mut val = 0u32;
                    unsafe {
                        for i in 0..covopt_param!("M_1359_36", 4) {
                            val |= (*ptr.add(i) as u32) << (i * covopt_param!("M_1360_64", 8));
                        }
                    }
                    self.registers[crate::inst_a!(inst)] = val;
                    reg_change = Some((crate::inst_a!(inst) as u8, val));
                }
                31 => {
                    let addr = self.registers[crate::inst_b!(inst)]
                        .wrapping_add(self.registers[crate::inst_c!(inst)])
                        as usize;
                    let ptr = self.get_mut_ptr(addr, covopt_param!("M_1370_53", 4))?;
                    let val = self.registers[crate::inst_a!(inst)];
                    unsafe {
                        for i in 0..covopt_param!("M_1373_36", 4) {
                            *ptr.add(i) = ((val >> (i * covopt_param!("M_1374_56", 8))) & covopt_param!("M_1374_62", 255)) as u8;
                        }
                    }
                    mem_change = Some((addr as u16, val));
                }
                32 => {
                    let val = self.registers[crate::inst_b!(inst)] as i32;
                    if let Some(res) = no_std_tool::math::exp_approx_q16(val) {
                        let val_u32 = res as u32;
                        self.registers[crate::inst_a!(inst)] = val_u32;
                        reg_change = Some((crate::inst_a!(inst) as u8, val_u32));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                33 => {
                    let val = self.registers[crate::inst_b!(inst)];
                    if let Some(res) = no_std_tool::math::rsqrt_approx_i32(val) {
                        self.registers[crate::inst_a!(inst)] = res;
                        reg_change = Some((crate::inst_a!(inst) as u8, res));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                34 => {
                    let val = (self.registers[crate::inst_b!(inst)] & covopt_param!("M_1399_70", 255)) as i8;
                    if let Some(res) = no_std_tool::math::silu_approx_i8(val) {
                        let val_u32 = (res as u32) & covopt_param!("M_1401_53", 255);
                        self.registers[crate::inst_a!(inst)] = val_u32;
                        reg_change = Some((crate::inst_a!(inst) as u8, val_u32));
                    } else {
                        return Err(VmError::MathError { pc: self.pc - 1 });
                    }
                }
                37 => {
                    let handler = self.neural_handler;
                    if let Some(h) = handler {
                        h(
                            self,
                            crate::inst_a!(inst),
                            crate::inst_b!(inst),
                            crate::inst_c!(inst),
                        );
                    }
                }
                35 => {
                    let a = crate::inst_a!(inst);
                    let b = crate::inst_b!(inst);
                    let c = crate::inst_c!(inst);
                    let dest_idx = a % covopt_param!("M_1423_39", 256);
                    let init_val = self.registers[dest_idx];
                    if self.host_context.is_some() {
                        let mut context = self.host_context.take().unwrap();
                        context.dispatch_hardware_call(self, a, b, c);
                        self.host_context = Some(context);
                    }
                    if self.registers[dest_idx] == init_val {
                        for handler in self.hardware_handlers.clone() {
                            handler(self, a, b, c);
                            if self.registers[dest_idx] != init_val {
                                break;
                            }
                        }
                        if self.registers[dest_idx] == init_val
                            && let Some(handler) = self.hardware_handler
                        {
                            handler(self, a, b, c);
                        }
                    }
                    reg_change = Some((a as u8, self.registers[a]));
                }
                36 => {
                    let a = crate::inst_a!(inst);
                    let b = crate::inst_b!(inst);
                    let c = crate::inst_c!(inst);
                    if self.ui_dispatcher.is_some() {
                        let mut dispatcher = self.ui_dispatcher.take().unwrap();
                        let _ = dispatcher.dispatch(self, a, b, c);
                        self.ui_dispatcher = Some(dispatcher);
                    }
                    if let Some(handler) = self.ui_handler {
                        handler(a, b, c);
                    }
                }
                38 => {
                    self.log_trace(current_pc, inst.0, reg_change, mem_change);
                    return Ok(VmResult::Yielded(steps));
                }
                39 => { // VecAdd
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let byte_len = len.checked_mul(covopt_param!("M_1467_51", 4)).ok_or(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr: dest })?;
                    let dest_ptr = self.get_mut_ptr(dest, byte_len)?;
                    let src1_ptr = self.get_ptr(src1, byte_len)?;
                    let src2_ptr = self.get_ptr(src2, byte_len)?;
                    unsafe {
                        crate::sgl::simd_ops::simd_vec_add(len, src1_ptr, src2_ptr, dest_ptr);
                    }
                    mem_change = Some((dest as u16, len as u32));
                }
                40 => { // VecMul
                    let len = self.registers[0] as usize;
                    let dest = self.registers[crate::inst_a!(inst)] as usize;
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let byte_len = len.checked_mul(covopt_param!("M_1481_51", 4)).ok_or(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr: dest })?;
                    let dest_ptr = self.get_mut_ptr(dest, byte_len)?;
                    let src1_ptr = self.get_ptr(src1, byte_len)?;
                    let src2_ptr = self.get_ptr(src2, byte_len)?;
                    unsafe {
                        crate::sgl::simd_ops::simd_vec_mul(len, src1_ptr, src2_ptr, dest_ptr);
                    }
                    mem_change = Some((dest as u16, len as u32));
                }
                41 => { // VecDot
                    let len = self.registers[0] as usize;
                    let dest_reg = crate::inst_a!(inst);
                    let src1 = self.registers[crate::inst_b!(inst)] as usize;
                    let src2 = self.registers[crate::inst_c!(inst)] as usize;
                    let byte_len = len.checked_mul(covopt_param!("M_1495_51", 4)).ok_or(VmError::MemoryAccessOutOfBounds { pc: self.pc.wrapping_sub(1), addr: src1 })?;
                    let src1_ptr = self.get_ptr(src1, byte_len)?;
                    let src2_ptr = self.get_ptr(src2, byte_len)?;
                    unsafe {
                        let mut sum = 0.0f32;
                        for i in 0..len {
                            let val1 = f32::from_le_bytes(core::ptr::read_unaligned(src1_ptr.add(i * covopt_param!("M_1501_101", 4)) as *const [u8; 4]));
                            let val2 = f32::from_le_bytes(core::ptr::read_unaligned(src2_ptr.add(i * covopt_param!("M_1502_101", 4)) as *const [u8; 4]));
                            sum += val1 * val2;
                        }
                        self.registers[dest_reg] = sum.to_bits();
                        reg_change = Some((dest_reg as u8, sum.to_bits()));
                    }
                }
                42 => { // Spawn
                    let target_pc = crate::inst_imm16!(inst);
                    return Ok(VmResult::Spawn(steps, target_pc, crate::inst_a!(inst) as u8));
                }
                43 => { // Await
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::Awaiting(steps, resource_id, crate::inst_a!(inst) as u8));
                }
                44 => { // Mmap
                    let resource_id = self.registers[crate::inst_b!(inst)];
                    return Ok(VmResult::MmapRequest(steps, resource_id));
                }
                45 => {
                    let dst = crate::inst_a!(inst);
                    let src = self.registers[crate::inst_b!(inst)];
                    let state = crate::inst_c!(inst);
                    let last = self.registers[state];
                    let delta = (src as i32).wrapping_sub(last as i32);
                    self.registers[state] = src;
                    self.registers[dst] = no_std_tool::compress::zigzag_encode_i32(delta);
                }
                46 => {
                    let dst = crate::inst_a!(inst);
                    let src = self.registers[crate::inst_b!(inst)];
                    let state = crate::inst_c!(inst);
                    let last = self.registers[state];
                    let delta = no_std_tool::compress::zigzag_decode_u32(src);
                    let current = (last as i32).wrapping_add(delta);
                    self.registers[state] = current as u32;
                    self.registers[dst] = current as u32;
                }
                op => {
                    return Err(VmError::InvalidOpcode {
                        pc: self.pc - 1,
                        opcode: op,
                    });
                }
            }
            self.log_trace(current_pc, inst.0, reg_change, mem_change);
        }
        Ok(VmResult::Halted(steps))
    }
}
#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::sgl::instruction::OpCode;
    #[test]
    fn test_div_by_zero() {
        let mut vm = ScriptVm::new();
        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_1561_55", 10), 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::Div as u8, covopt_param!("M_1563_48", 3), 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::DivideByZero { pc: 2 }));
    }
    #[test]
    fn test_stack_overflow() {
        let mut vm = ScriptVm::new();
        let code = [Instruction::new(OpCode::Call as u8, 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackOverflow { pc: 0 }));
    }
    #[test]
    fn test_stack_underflow() {
        let mut vm = ScriptVm::new();
        let code = [Instruction::new(OpCode::Ret as u8, 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::StackUnderflow { pc: 0 }));
    }
    #[test]
    fn test_invalid_opcode() {
        let mut vm = ScriptVm::new();
        let code = [Instruction::new(covopt_param!("M_1586_37", 153), 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(
            result,
            Err(VmError::InvalidOpcode {
                pc: 0,
                opcode: 0x99
            })
        );
    }
    #[test]
    fn test_floats() {
        let n = std::env::var("COVOPT_N")
            .unwrap_or(std::string::String::from("1"))
            .parse::<usize>()
            .unwrap();
        let mut vm = ScriptVm::new();
        let val1 = (covopt_param!("M_1603_19", 3.5) as f32).to_bits();
        let val2 = (covopt_param!("M_1604_19", 1.5) as f32).to_bits();
        vm.registers[1] = val1;
        vm.registers[2] = val2;
        let code = [
            Instruction::new(OpCode::FAdd as u8, covopt_param!("M_1608_49", 3), 1, 2),
            Instruction::new(OpCode::FSub as u8, covopt_param!("M_1609_49", 4), 1, 2),
            Instruction::new(OpCode::FMul as u8, covopt_param!("M_1610_49", 5), 1, 2),
            Instruction::new(OpCode::FDiv as u8, covopt_param!("M_1611_49", 6), 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        for _ in 0..n {
            vm.run(&code).unwrap();
        }
        assert_eq!(f32::from_bits(vm.registers[3]), 5.0f32);
        assert_eq!(f32::from_bits(vm.registers[4]), 2.0f32);
        assert_eq!(f32::from_bits(vm.registers[5]), 5.25f32);
        assert_eq!(f32::from_bits(vm.registers[6]), 3.5 / 1.5);
    }
    #[test]
    fn test_memory_load_store() {
        let mut vm = ScriptVm::new();
        vm.registers[1] = covopt_param!("M_1625_26", 42);
        vm.registers[2] = covopt_param!("M_1626_26", 10);
        vm.registers[covopt_param!("M_1627_21", 3)] = covopt_param!("M_1627_26", 4);
        let code = [
            Instruction::new(OpCode::Store as u8, 1, 2, covopt_param!("M_1629_56", 3)),
            Instruction::new(OpCode::Load as u8, covopt_param!("M_1630_49", 4), 2, covopt_param!("M_1630_55", 3)),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        vm.run(&code).unwrap();
        assert_eq!(vm.registers[4], 42);
        assert_eq!(vm.memory[14], 42);
        assert_eq!(vm.memory[15], 0);
        assert_eq!(vm.memory[16], 0);
        assert_eq!(vm.memory[17], 0);
    }
    #[test]
    fn test_math_approximations() {
        let mut vm = ScriptVm::new();
        vm.registers[1] = 0;
        vm.registers[2] = covopt_param!("M_1644_26", 4);
        vm.registers[covopt_param!("M_1645_21", 3)] = 2;
        let code = [
            Instruction::new(OpCode::Exp as u8, covopt_param!("M_1647_48", 4), 1, 0),
            Instruction::new(OpCode::Rsqrt as u8, covopt_param!("M_1648_50", 5), 2, 0),
            Instruction::new(OpCode::Silu as u8, covopt_param!("M_1649_49", 6), covopt_param!("M_1649_52", 3), 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        vm.run(&code).unwrap();
        assert_eq!(vm.registers[4], 65536);
        assert_eq!(vm.registers[5], 32768);
        assert!(vm.registers[6] > 0);
    }
    #[test]
    fn test_abort_flag() {
        let mut vm = ScriptVm::new();
        vm.max_steps = None;
        static ABORT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(true);
        fn abort_checker() -> bool {
            ABORT.load(core::sync::atomic::Ordering::Relaxed)
        }
        vm.abort_flag = Some(abort_checker);
        let code = [Instruction::new(OpCode::Jmp as u8, 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(result.unwrap(), crate::sgl::vm::VmResult::Halted(0));
    }
    #[test]
    fn test_out_of_fuel() {
        let mut vm = ScriptVm::new();
        vm.max_steps = Some(covopt_param!("M_1673_28", 50));
        let code = [Instruction::new(OpCode::Jmp as u8, 0, 0, 0)];
        let result = vm.run(&code);
        assert_eq!(result, Err(VmError::OutOfFuel { pc: 0 }));
    }
    #[test]
    fn test_trace_logging() {
        let mut vm = ScriptVm::new();
        vm.tracing_enabled = true;
        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_1683_55", 42), 0),
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
            if crate::opcode!(inst) == OpCode::LoadImm as u8 {
                assert_eq!(crate::inst_a!(inst), 1);
            }
        });
        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_1711_55", 10), 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        vm.run(&code).unwrap();
        assert_eq!(EXEC_COUNT.load(Ordering::Relaxed), 2);
    }
    #[test]
    fn test_run_slow_syscall_and_hardware_handlers() {
        let mut vm = ScriptVm::new();
        vm.tracing_enabled = true; // Forces run_slow execution path

        fn mock_syscall(vm: &mut ScriptVm, a: usize, _b: usize, _c: usize) {
            vm.registers[a] = covopt_param!("M_1723_30", 100);
        }

        fn mock_hardware(vm: &mut ScriptVm, a: usize, _b: usize, _c: usize) {
            vm.registers[a] = covopt_param!("M_1727_30", 200);
        }

        vm.register_syscall_handler_ext(mock_syscall);
        vm.register_hardware_handler(mock_hardware);

        // Syscall: OpCode 29, R[1], R[0], R[0]
        let syscall_inst = Instruction::new(OpCode::SysCall as u8, 1, 0, 0);
        // HardwareCall: OpCode 35, R[2], R[0], R[0]
        let hardware_inst = Instruction::new(OpCode::HardwareCall as u8, 2, 0, 0);
        let code = [syscall_inst, hardware_inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

        let res = vm.run(&code);
        assert!(res.is_ok());
        assert_eq!(vm.registers[1], 100);
        assert_eq!(vm.registers[2], 200);
    }
    #[test]
    fn test_panic_recovery() {
        let mut vm = ScriptVm::new();
        vm.print_handler = Some(|_| {
            panic!("Mock handler panic!");
        });
        let code = [Instruction::new(OpCode::PrintReg as u8, 0, 0, 0)];
        let vm_ref = &mut vm;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _ = vm_ref.run(&code);
        }));
        assert!(result.is_err());
    }
    #[test]
    fn test_hot_reload_state_preservation() {
        let mut vm = ScriptVm::new();
        vm.pc = covopt_param!("M_1760_16", 42);
        vm.sp = covopt_param!("M_1761_16", 5);
        vm.call_stack[0] = covopt_param!("M_1762_27", 99);
        vm.registers[covopt_param!("M_1763_21", 3)] = covopt_param!("M_1763_26", 77);
        vm.registers[covopt_param!("M_1764_21", 20)] = covopt_param!("M_1764_27", 88);
        vm.memory[covopt_param!("M_1765_18", 10)] = covopt_param!("M_1765_24", 55);
        vm.hot_reload();
        assert_eq!(vm.pc, 0);
        assert_eq!(vm.sp, 0);
        assert_eq!(vm.call_stack[0], 0);
        assert_eq!(vm.registers[3], 0);
        assert_eq!(vm.registers[20], 88);
        assert_eq!(vm.memory[10], 55);
    }
    #[test]
    fn test_audit() {
        let n = std::env::var("COVOPT_N")
            .unwrap_or(std::string::String::from("1000"))
            .parse::<usize>()
            .unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut handles = std::vec::Vec::new();
        for _ in 0..covopt_param!("M_1782_20", 4) {
            let tx_clone = tx.clone();
            let handle = std::thread::spawn(move || {
                let n = n;
                let mut vm = ScriptVm::new();
                vm.print_handler = Some(|_| {});
                vm.registers[1] = 2;
                vm.registers[2] = 1;
                let code = [
                    Instruction::new(OpCode::LoadImm as u8, 1, 2, 0),
                    Instruction::new(OpCode::LoadImm as u8, 2, 1, 0),
                    Instruction::new(OpCode::LoadImm as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfZero as u8, 1, 0, 0),
                    Instruction::new(OpCode::JmpIfZero as u8, 0, covopt_param!("M_1795_65", 6), 0),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfEq as u8, 1, 2, 0),
                    Instruction::new(OpCode::JmpIfEq as u8, 1, 1, covopt_param!("M_1798_66", 9)),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfLt as u8, 1, 2, 0),
                    Instruction::new(OpCode::JmpIfLt as u8, 2, 1, covopt_param!("M_1801_66", 12)),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfGt as u8, 2, 1, 0),
                    Instruction::new(OpCode::JmpIfGt as u8, 1, 2, covopt_param!("M_1804_66", 15)),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfFLt as u8, 1, 2, 0),
                    Instruction::new(OpCode::JmpIfFLt as u8, 2, 1, covopt_param!("M_1807_67", 18)),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::JmpIfFGt as u8, 2, 1, 0),
                    Instruction::new(OpCode::JmpIfFGt as u8, 1, 2, covopt_param!("M_1810_67", 21)),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                    Instruction::new(OpCode::LoadImm16 as u8, covopt_param!("M_1812_62", 4), 0, covopt_param!("M_1812_68", 5)),
                    Instruction::new(OpCode::Add as u8, covopt_param!("M_1813_56", 5), 1, 2),
                    Instruction::new(OpCode::Sub as u8, covopt_param!("M_1814_56", 5), 1, 2),
                    Instruction::new(OpCode::Mul as u8, covopt_param!("M_1815_56", 5), 1, 2),
                    Instruction::new(OpCode::Div as u8, covopt_param!("M_1816_56", 5), 1, 2),
                    Instruction::new(OpCode::Mod as u8, covopt_param!("M_1817_56", 5), 1, 2),
                    Instruction::new(OpCode::And as u8, covopt_param!("M_1818_56", 5), 1, 2),
                    Instruction::new(OpCode::Or as u8, covopt_param!("M_1819_55", 5), 1, 2),
                    Instruction::new(OpCode::Xor as u8, covopt_param!("M_1820_56", 5), 1, 2),
                    Instruction::new(OpCode::Shl as u8, covopt_param!("M_1821_56", 5), 1, 2),
                    Instruction::new(OpCode::Shr as u8, covopt_param!("M_1822_56", 5), 1, 2),
                    Instruction::new(OpCode::CmpEq as u8, covopt_param!("M_1823_58", 5), 1, 2),
                    Instruction::new(OpCode::CmpLt as u8, covopt_param!("M_1824_58", 5), 1, 2),
                    Instruction::new(OpCode::FAdd as u8, covopt_param!("M_1825_57", 5), 1, 2),
                    Instruction::new(OpCode::FSub as u8, covopt_param!("M_1826_57", 5), 1, 2),
                    Instruction::new(OpCode::FMul as u8, covopt_param!("M_1827_57", 5), 1, 2),
                    Instruction::new(OpCode::FDiv as u8, covopt_param!("M_1828_57", 5), 1, 2),
                    Instruction::new(OpCode::Store as u8, 1, 0, 2),
                    Instruction::new(OpCode::Load as u8, covopt_param!("M_1830_57", 5), 0, 2),
                    Instruction::new(OpCode::PrintReg as u8, covopt_param!("M_1831_61", 5), 0, 0),
                    Instruction::new(OpCode::SysCall as u8, covopt_param!("M_1832_60", 5), 0, 0),
                    Instruction::new(OpCode::Call as u8, 0, covopt_param!("M_1833_60", 44), 0),
                    Instruction::new(OpCode::Jmp as u8, 0, covopt_param!("M_1834_59", 45), 0),
                    Instruction::new(OpCode::Ret as u8, 0, 0, 0),
                    Instruction::new(OpCode::Halt as u8, 0, 0, 0),
                ];
                for _ in 0..n {
                    std::hint::black_box(vm.run(&code).unwrap());
                }
                tx_clone.send(()).unwrap();
            });
            handles.push(handle);
        }
        for _ in 0..covopt_param!("M_1845_20", 4) {
            rx.recv_timeout(std::time::Duration::from_secs(covopt_param!("M_1846_59", 5)))
                .expect("Watchdog timeout");
        }
        for handle in handles {
            handle.join().unwrap();
        }
        let mut vm_err = ScriptVm::new();
        let code_div0 = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_1854_55", 10), 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::Div as u8, covopt_param!("M_1856_48", 3), 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_div0);
        let code_fdiv0 = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_1861_55", 10), 0),
            Instruction::new(OpCode::LoadImm as u8, 2, 0, 0),
            Instruction::new(OpCode::FDiv as u8, covopt_param!("M_1863_49", 3), 1, 2),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_fdiv0);
        let code_mem = [
            Instruction::new(
                OpCode::LoadImm16 as u8,
                1,
                (covopt_param!("M_1871_17", 10000) & covopt_param!("M_1871_25", 255)) as u8,
                (covopt_param!("M_1872_17", 10000) >> covopt_param!("M_1872_26", 8)) as u8,
            ),
            Instruction::new(OpCode::Load as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_mem);
        let code_so = [Instruction::new(OpCode::Call as u8, 0, 0, 0); 257];
        let mut vm_so = ScriptVm::new();
        let _ = vm_so.run_fast(&code_so);
        let code_su = [
            Instruction::new(OpCode::Ret as u8, 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_su);
        let code_inv = [
            Instruction::new(covopt_param!("M_1887_29", 255), 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_inv);
        let mut vm_handlers = ScriptVm::new();
        vm_handlers.print_handler = Some(|_| {});
        vm_handlers.syscall_handler = Some(|_, _, _| {});
        vm_handlers.hardware_handler = Some(|_, _, _, _| {});
        vm_handlers.ui_handler = Some(|_, _, _| {});
        vm_handlers.neural_handler = Some(|_, _, _, _| {});
        let code_handlers = [
            Instruction::new(OpCode::PrintReg as u8, 0, 0, 0),
            Instruction::new(OpCode::SysCall as u8, 0, 0, 0),
            Instruction::new(OpCode::HardwareCall as u8, 0, 0, 0),
            Instruction::new(OpCode::UiCall as u8, 1, 1, 0),
            Instruction::new(OpCode::UiCall as u8, 0, covopt_param!("M_1902_54", 5), 0),
            Instruction::new(OpCode::NeuralCall as u8, 0, 0, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_handlers.run_fast(&code_handlers);
        let mut vm_jumps = ScriptVm::new();
        vm_jumps.registers[1] = 0;
        vm_jumps.registers[2] = 1;
        let code_jumps = [
            Instruction::new(OpCode::JmpIfZero as u8, 1, 1, 0),
            Instruction::new(OpCode::JmpIfZero as u8, 2, 2, 0),
            Instruction::new(OpCode::JmpIfEq as u8, 1, 1, covopt_param!("M_1913_58", 3)),
            Instruction::new(OpCode::JmpIfEq as u8, 1, 2, covopt_param!("M_1914_58", 3)),
            Instruction::new(OpCode::JmpIfLt as u8, 1, 2, covopt_param!("M_1915_58", 5)),
            Instruction::new(OpCode::JmpIfLt as u8, 2, 1, covopt_param!("M_1916_58", 5)),
            Instruction::new(OpCode::JmpIfGt as u8, 2, 1, covopt_param!("M_1917_58", 7)),
            Instruction::new(OpCode::JmpIfGt as u8, 1, 2, covopt_param!("M_1918_58", 7)),
            Instruction::new(OpCode::JmpIfFLt as u8, 1, 2, covopt_param!("M_1919_59", 9)),
            Instruction::new(OpCode::JmpIfFLt as u8, 2, 1, covopt_param!("M_1920_59", 9)),
            Instruction::new(OpCode::JmpIfFGt as u8, 2, 1, covopt_param!("M_1921_59", 11)),
            Instruction::new(OpCode::JmpIfFGt as u8, 1, 2, covopt_param!("M_1922_59", 11)),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_jumps.run_fast(&code_jumps);
        let code_store = [
            Instruction::new(
                OpCode::LoadImm16 as u8,
                1,
                (covopt_param!("M_1930_17", 10000) & covopt_param!("M_1930_25", 255)) as u8,
                (covopt_param!("M_1931_17", 10000) >> covopt_param!("M_1931_26", 8)) as u8,
            ),
            Instruction::new(OpCode::Store as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_store);
        let code_math_exp = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_1938_55", 11), 0),
            Instruction::new(OpCode::LoadImm as u8, 2, covopt_param!("M_1939_55", 16), 0),
            Instruction::new(OpCode::Shl as u8, 1, 1, 2),
            Instruction::new(OpCode::Exp as u8, covopt_param!("M_1941_48", 3), 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_exp);
        let code_math_rsqrt = [
            Instruction::new(OpCode::LoadImm as u8, 1, 0, 0),
            Instruction::new(OpCode::Rsqrt as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_rsqrt);
        let code_math_silu = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_1952_55", 128), 0),
            Instruction::new(OpCode::Silu as u8, 2, 1, 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let _ = vm_err.run_fast(&code_math_silu);
    }

    #[test]
    fn test_yield_inside_subroutine() {
        let mut vm = ScriptVm::new();
        // Program structure:
        // PC 0: CALL subroutine at PC 3 (imm16 = 3)
        // PC 1: LOADIMM R1 = 99
        // PC 2: HALT
        // PC 3 (subroutine start): YIELD
        // PC 4: RET
        let code = [
            Instruction::new(OpCode::Call as u8, 0, covopt_param!("M_1969_52", 3), 0),
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_1970_55", 99), 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
            Instruction::new(OpCode::Yield as u8, 0, 0, 0),
            Instruction::new(OpCode::Ret as u8, 0, 0, 0),
        ];

        let res1 = vm.run(&code);
        assert_eq!(res1, Ok(VmResult::Yielded(2)));
        assert_eq!(vm.pc, 4);
        assert_eq!(vm.sp, 1);

        let res2 = vm.run(&code);
        assert!(matches!(res2, Ok(VmResult::Halted(_))));
        assert_eq!(vm.registers[1], 99);
        assert_eq!(vm.sp, 0);
    }
}
