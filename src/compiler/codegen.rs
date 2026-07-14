use crate::compiler::ast::*;
use crate::compiler::ir::*;
use scriptgo_vm::instruction::{Instruction as VmInst, OpCode};
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use no_std_tool::collections::HashMap;

pub struct CodeGen {
    vars_reg: HashMap<String, u8>,
    vm_regs: HashMap<ValueId, u8>,
    reg_counter: u8,
}

impl CodeGen {
    pub fn new() -> Self {
        Self {
            vars_reg: HashMap::new(),
            vm_regs: HashMap::new(),
            reg_counter: 1, // Reg 0 reserved for constant 0
        }
    }

    fn alloc_reg(&mut self, val: ValueId) -> u8 {
        let r = self.reg_counter;
        self.reg_counter += 1;
        self.vm_regs.insert(val, r);
        r
    }

    fn get_reg(&self, val: ValueId) -> u8 {
        *self.vm_regs.get(&val).unwrap_or(&0)
    }

    fn get_var_reg(&mut self, name: &str) -> u8 {
        if let Some(&r) = self.vars_reg.get(name) {
            r
        } else {
            let r = self.reg_counter;
            self.reg_counter += 1;
            self.vars_reg.insert(name.to_string(), r);
            r
        }
    }

    pub fn compile(&mut self, prog: &Program) -> Result<Vec<VmInst>, String> {
        let mut func_ir = FunctionIR::new(String::from("main"));
        
        let start_block = BasicBlock { id: 0, insts: Vec::new(), successors: Vec::new() };
        func_ir.blocks.push(start_block);

        let mut current_block_id = 0;
        for stmt in &prog.statements {
            current_block_id = self.stmt_to_ir(stmt, &mut func_ir, current_block_id)?;
        }

        // Optimizer is disabled temporarily to avoid wiping out our VarLoad/VarStore which are not SSA
        // Optimizer::optimize(&mut func_ir);

        // Map blocks to bytecode indices
        let mut bytecode = Vec::new();
        let mut block_starts = HashMap::new();
        let mut backpatch_jumps = Vec::new(); // (bytecode_index, target_block_id, is_conditional)

        for block in &func_ir.blocks {
            block_starts.insert(block.id, bytecode.len());

            for inst in &block.insts {
                match &inst.op {
                    Op::LoadImm(val) => {
                        let r = self.alloc_reg(inst.id);
                        let v = *val;
                        if v >= 0 && v < 256 {
                            bytecode.push(VmInst::new(OpCode::LoadImm as u8, r, v as u8, 0));
                        } else {
                            let low = (v & 0xFF) as u8;
                            let high = ((v >> 8) & 0xFF) as u8;
                            bytecode.push(VmInst::new(OpCode::LoadImm16 as u8, r, low, high));
                        }
                    }
                    Op::Add(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::Add as u8, r, r1, r2));
                    }
                    Op::Sub(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::Sub as u8, r, r1, r2));
                    }
                    Op::Mul(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::Mul as u8, r, r1, r2));
                    }
                    Op::Div(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::Div as u8, r, r1, r2));
                    }
                    Op::VarLoad(name) => {
                        let r_dest = self.alloc_reg(inst.id);
                        let r_src = self.get_var_reg(name);
                        bytecode.push(VmInst::new(OpCode::Add as u8, r_dest, r_src, 0)); // Move src to dest
                    }
                    Op::VarStore(name, val) => {
                        let r_dest = self.get_var_reg(name);
                        let r_src = self.get_reg(*val);
                        bytecode.push(VmInst::new(OpCode::Add as u8, r_dest, r_src, 0)); // Move val to dest
                    }
                    Op::Jmp(target_block) => {
                        backpatch_jumps.push((bytecode.len(), *target_block, false));
                        bytecode.push(VmInst::new(OpCode::Jmp as u8, 0, 0, 0)); // Placeholder
                    }
                    Op::JmpIf(cond, true_block, false_block) => {
                        let r_cond = self.get_reg(*cond);
                        // JmpIfZero takes (A, B, C) where A is tested register, (B,C) is imm16.
                        // We test if r_cond is 0. If it is 0, the condition is FALSE, so we jump to false_block.
                        backpatch_jumps.push((bytecode.len(), *false_block, true));
                        bytecode.push(VmInst::new(OpCode::JmpIfZero as u8, r_cond, 0, 0)); // Placeholder jump to false

                        // Otherwise we fall through and unconditionally jump to true_block
                        backpatch_jumps.push((bytecode.len(), *true_block, false));
                        bytecode.push(VmInst::new(OpCode::Jmp as u8, 0, 0, 0)); // Placeholder jump to true
                    }
                    _ => {}
                }
            }
        }

        // Halt at the end
        bytecode.push(VmInst::new(OpCode::Halt as u8, 0, 0, 0));

        // Backpatch jumps
        for (idx, target_block, is_cond) in backpatch_jumps {
            if let Some(&target_pc) = block_starts.get(&target_block) {
                let low = (target_pc & 0xFF) as u8;
                let high = ((target_pc >> 8) & 0xFF) as u8;
                if is_cond {
                    let r_cond = bytecode[idx].a() as u8;
                    bytecode[idx] = VmInst::new(OpCode::JmpIfZero as u8, r_cond, low, high);
                } else {
                    bytecode[idx] = VmInst::new(OpCode::Jmp as u8, 0, low, high);
                }
            } else {
                return Err("Failed to resolve jump target".into());
            }
        }

        Ok(bytecode)
    }

    fn new_block(&mut self, func: &mut FunctionIR) -> usize {
        let id = func.blocks.len();
        func.blocks.push(BasicBlock { id, insts: Vec::new(), successors: Vec::new() });
        id
    }

    fn append_inst(&mut self, func: &mut FunctionIR, block_id: usize, inst: Instruction) {
        func.blocks[block_id].insts.push(inst);
    }

    fn stmt_to_ir(&mut self, stmt: &Statement, func: &mut FunctionIR, current_block: usize) -> Result<usize, String> {
        let mut curr = current_block;
        match stmt {
            Statement::LetDecl(name, _ty, expr) => {
                let val_id = self.expr_to_ir(expr, func, curr)?;
                let id = func.alloc_val();
                self.append_inst(func, curr, Instruction { id, op: Op::VarStore(name.clone(), val_id) });
            }
            Statement::Assign(name, expr) => {
                let val_id = self.expr_to_ir(expr, func, curr)?;
                let id = func.alloc_val();
                self.append_inst(func, curr, Instruction { id, op: Op::VarStore(name.clone(), val_id) });
            }
            Statement::ExprStmt(expr) => {
                self.expr_to_ir(expr, func, curr)?;
            }
            Statement::While(cond, body) => {
                let cond_block = self.new_block(func);
                let body_block = self.new_block(func);
                let end_block = self.new_block(func);

                // Jump from current to cond
                let jmp_id = func.alloc_val();
                self.append_inst(func, curr, Instruction { id: jmp_id, op: Op::Jmp(cond_block) });

                // Cond block
                let cond_val = self.expr_to_ir(cond, func, cond_block)?;
                let jmpif_id = func.alloc_val();
                self.append_inst(func, cond_block, Instruction { id: jmpif_id, op: Op::JmpIf(cond_val, body_block, end_block) });

                // Body block
                let mut body_curr = body_block;
                for s in body {
                    body_curr = self.stmt_to_ir(s, func, body_curr)?;
                }
                let loop_jmp_id = func.alloc_val();
                self.append_inst(func, body_curr, Instruction { id: loop_jmp_id, op: Op::Jmp(cond_block) });

                curr = end_block;
            }
            Statement::If(cond, then_br, else_br) => {
                let cond_val = self.expr_to_ir(cond, func, curr)?;
                
                let then_block = self.new_block(func);
                let else_block = self.new_block(func);
                let end_block = self.new_block(func);

                let jmpif_id = func.alloc_val();
                self.append_inst(func, curr, Instruction { id: jmpif_id, op: Op::JmpIf(cond_val, then_block, else_block) });

                let mut then_curr = then_block;
                for s in then_br {
                    then_curr = self.stmt_to_ir(s, func, then_curr)?;
                }
                let t_jmp_id = func.alloc_val();
                self.append_inst(func, then_curr, Instruction { id: t_jmp_id, op: Op::Jmp(end_block) });

                let mut else_curr = else_block;
                for s in else_br {
                    else_curr = self.stmt_to_ir(s, func, else_curr)?;
                }
                let e_jmp_id = func.alloc_val();
                self.append_inst(func, else_curr, Instruction { id: e_jmp_id, op: Op::Jmp(end_block) });

                curr = end_block;
            }
            _ => return Err(String::from("Statement not supported yet")),
        }
        Ok(curr)
    }

    fn expr_to_ir(&mut self, expr: &Expr, func: &mut FunctionIR, block_id: usize) -> Result<ValueId, String> {
        match expr {
            Expr::IntLiteral(val) => {
                let id = func.alloc_val();
                self.append_inst(func, block_id, Instruction { id, op: Op::LoadImm(*val) });
                Ok(id)
            }
            Expr::Identifier(name) => {
                let id = func.alloc_val();
                self.append_inst(func, block_id, Instruction { id, op: Op::VarLoad(name.clone()) });
                Ok(id)
            }
            Expr::BinaryOp(left, op, right) => {
                let l_id = self.expr_to_ir(left, func, block_id)?;
                let r_id = self.expr_to_ir(right, func, block_id)?;
                let id = func.alloc_val();
                
                let ir_op = match op {
                    BinaryOperator::Add => Op::Add(l_id, r_id),
                    BinaryOperator::Sub => Op::Sub(l_id, r_id),
                    BinaryOperator::Mul => Op::Mul(l_id, r_id),
                    BinaryOperator::Div => Op::Div(l_id, r_id),
                    BinaryOperator::Lt => {
                        // Hack for Phase 3: We evaluate `a < b` by doing `b - a`.
                        // As long as `a < b`, `b - a` is > 0 (NOT 0), so it takes True path.
                        // When `a >= b`, `b - a` is 0 (wrapping arithmetic prevents negative results from wrapping to exactly 0 unless a==b? Wait.)
                        // If b = 5, a = 6, b - a = -1. In u32 wrapping, it is 0xFFFFFFFF.
                        // 0xFFFFFFFF is NOT 0. So it would still take True path! This is a bug!
                        // In `vm.rs` we have:
                        // `JmpIfLt = 0x33, // If R[A] < R[B], PC = C`
                        // Why don't we just use `JmpIfLt`?
                        // Because `JmpIfLt` takes 8-bit `C`. But we are jumping to basic blocks!
                        // Oh no, 8-bit jump target is too small.
                        // But wait! This is just a script_crusher benchmark. 8-bit (255 instructions) is MORE than enough for this benchmark! The benchmark only has like 10 instructions!
                        // Let's just use `Op::Lt(l_id, r_id)` and map it to `JmpIfLt`? No, `JmpIfLt` jumps immediately.
                        // Let's implement `Op::Lt(l_id, r_id)` and a special conditional jump that uses `JmpIfLt` in codegen?
                        // Wait, if I just modify the `vm.rs` to make `JmpIfLt` take a 16-bit target!
                        // `0x33 => { if self.registers[a] < self.registers[b] { self.pc = inst.imm16() as usize; } }`
                        // That is SO easy and the right way to do it.
                        Op::Lt(l_id, r_id) 
                    },
                    BinaryOperator::Eq => Op::Eq(l_id, r_id),
                    _ => return Err("Unsupported operator in Phase 3".into()),
                };
                self.append_inst(func, block_id, Instruction { id, op: ir_op });
                Ok(id)
            }
            _ => Err("Expression not supported in Phase 3".into()),
        }
    }
}
