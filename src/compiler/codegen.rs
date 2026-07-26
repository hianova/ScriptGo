#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use crate::compiler::ast::*;
use crate::compiler::ir::*;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use no_std_tool::collections::HashMap;
use crate::sgl::instruction::{Instruction as VmInst, OpCode};

pub struct CodeGen {
    vars_reg: HashMap<String, u8>,
    vm_regs: HashMap<ValueId, u8>,
    reg_counter: u8,
}

impl Default for CodeGen {
    fn default() -> Self {
        Self::new()
    }
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

        let start_block = BasicBlock {
            id: 0,
            insts: Vec::new(),
            successors: Vec::new(),
        };
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
                        if (0..covopt_param!("M_87_31", 256)).contains(&v) {
                            bytecode.push(VmInst::new(OpCode::LoadImm as u8, r, v as u8, 0));
                        } else {
                            let low = (v & covopt_param!("M_90_43", 255)) as u8;
                            let high = ((v >> covopt_param!("M_91_46", 8)) & covopt_param!("M_91_51", 255)) as u8;
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
                    Op::Mod(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::Mod as u8, r, r1, r2));
                    }
                    Op::Eq(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::CmpEq as u8, r, r1, r2));
                    }
                    Op::Ne(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        let r_eq = self.reg_counter;
                        self.reg_counter += 1;
                        let r_one = self.reg_counter;
                        self.reg_counter += 1;
                        bytecode.push(VmInst::new(OpCode::LoadImm as u8, r_one, 1, 0));
                        bytecode.push(VmInst::new(OpCode::CmpEq as u8, r_eq, r1, r2));
                        bytecode.push(VmInst::new(OpCode::Sub as u8, r, r_one, r_eq));
                    }
                    Op::Lt(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::CmpLt as u8, r, r1, r2));
                    }
                    Op::Gt(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::CmpLt as u8, r, r2, r1));
                    }
                    Op::Le(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        let r_cmp = self.reg_counter;
                        self.reg_counter += 1;
                        let r_one = self.reg_counter;
                        self.reg_counter += 1;
                        bytecode.push(VmInst::new(OpCode::LoadImm as u8, r_one, 1, 0));
                        bytecode.push(VmInst::new(OpCode::CmpLt as u8, r_cmp, r2, r1));
                        bytecode.push(VmInst::new(OpCode::Sub as u8, r, r_one, r_cmp));
                    }
                    Op::Ge(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        let r_cmp = self.reg_counter;
                        self.reg_counter += 1;
                        let r_one = self.reg_counter;
                        self.reg_counter += 1;
                        bytecode.push(VmInst::new(OpCode::LoadImm as u8, r_one, 1, 0));
                        bytecode.push(VmInst::new(OpCode::CmpLt as u8, r_cmp, r1, r2));
                        bytecode.push(VmInst::new(OpCode::Sub as u8, r, r_one, r_cmp));
                    }
                    Op::And(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::And as u8, r, r1, r2));
                    }
                    Op::Or(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::Or as u8, r, r1, r2));
                    }
                    Op::ShiftLeft(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::Shl as u8, r, r1, r2));
                    }
                    Op::ShiftRight(v1, v2) => {
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        bytecode.push(VmInst::new(OpCode::Shr as u8, r, r1, r2));
                    }
                    Op::VarLoad(name) => {
                        let r_dest = self.alloc_reg(inst.id);
                        let r_src = self.get_var_reg(name);
                        bytecode.push(VmInst::new(OpCode::Add as u8, r_dest, r_src, 0));
                        // Move src to dest
                    }
                    Op::VarStore(name, val) => {
                        let r_dest = self.get_var_reg(name);
                        let r_src = self.get_reg(*val);
                        bytecode.push(VmInst::new(OpCode::Add as u8, r_dest, r_src, 0));
                        // Move val to dest
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
                    Op::Call(name, args) => {
                        if name == "set_color" {
                            let r_arg0 = self.get_reg(args[0]);
                            let r_arg1 = self.get_reg(args[1]);
                            let r_arg2 = self.get_reg(args[2]);
                            bytecode.push(VmInst::new(
                                OpCode::SysCall as u8,
                                r_arg0,
                                r_arg1,
                                r_arg2,
                            ));
                        } else if name == "forward_pass" {
                            let r_dest = self.get_reg(inst.id);
                            let r_arg0 = self.get_reg(args[0]);
                            bytecode.push(VmInst::new(OpCode::NeuralCall as u8, r_dest, r_arg0, 0));
                        } else if name == "ui_call" {
                            let r_arg0 = self.get_reg(args[0]);
                            let r_arg1 = self.get_reg(args[1]);
                            let r_arg2 = self.get_reg(args[2]);
                            bytecode.push(VmInst::new(OpCode::UiCall as u8, r_arg0, r_arg1, r_arg2));
                        } else if name == "db_get_balance" {
                            let r_dest = self.get_reg(inst.id);
                            let r_arg0 = self.get_reg(args[0]);
                            bytecode.push(VmInst::new(
                                OpCode::HardwareCall as u8,
                                r_dest,
                                r_arg0,
                                0,
                            ));
                        } else if name == "db_get_status" {
                            let r_dest = self.get_reg(inst.id);
                            let r_arg0 = self.get_reg(args[0]);
                            bytecode.push(VmInst::new(
                                OpCode::HardwareCall as u8,
                                r_dest,
                                r_arg0,
                                1,
                            ));
                        } else if name == "vector_add" {
                            let r_dest = self.get_reg(inst.id);
                            // arg0 is size
                            let r_arg0 = self.get_reg(args[0]);
                            bytecode.push(VmInst::new(
                                OpCode::HardwareCall as u8,
                                r_dest,
                                r_arg0,
                                2,
                            ));
                        }
                    }
                    Op::MacroCall(name, args) => {
                        if name == "server.start" {
                            let r_port = self.get_reg(args[0]);
                            let r_id = self.reg_counter;
                            self.reg_counter += 1;
                            bytecode.push(VmInst::new(OpCode::LoadImm as u8, r_id, 2, 0));
                            bytecode.push(VmInst::new(OpCode::SysCall as u8, r_id, r_port, 0));
                        } else if name == "db.filter" {
                            let r_table = self.get_reg(args[0]);
                            let r_cond = self.get_reg(args[1]);
                            bytecode.push(VmInst::new(OpCode::HardwareCall as u8, covopt_param!("M_290_82", 3), r_table, r_cond));
                        } else if name == "ui.render" {
                            let r_dom = self.get_reg(args[0]);
                            bytecode.push(VmInst::new(OpCode::UiCall as u8, r_dom, 0, 0));
                        } else {
                            let r_arg0 = if !args.is_empty() { self.get_reg(args[0]) } else { 0 };
                            let r_id = self.reg_counter;
                            self.reg_counter += 1;
                            bytecode.push(VmInst::new(OpCode::LoadImm as u8, r_id, covopt_param!("M_298_83", 153), 0));
                            bytecode.push(VmInst::new(OpCode::SysCall as u8, r_id, r_arg0, 0));
                        }
                    }
                    Op::Spawn(target_pc) => {
                        let r_dest = self.get_reg(inst.id);
                        let b = (target_pc & covopt_param!("M_304_45", 255)) as u8;
                        let c = ((target_pc >> covopt_param!("M_305_47", 8)) & covopt_param!("M_305_52", 255)) as u8;
                        bytecode.push(VmInst::new(OpCode::Spawn as u8, r_dest, b, c));
                    }
                    Op::Await(task_id) => {
                        let r_dest = self.get_reg(inst.id);
                        let r_res = self.get_reg(*task_id);
                        bytecode.push(VmInst::new(OpCode::Await as u8, r_dest, r_res, 0));
                    }
                    Op::Yield => {
                        bytecode.push(VmInst::new(OpCode::Yield as u8, 0, 0, 0));
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
                let low = (target_pc & covopt_param!("M_327_39", 255)) as u8;
                let high = ((target_pc >> covopt_param!("M_328_42", 8)) & covopt_param!("M_328_47", 255)) as u8;
                if is_cond {
                    let r_cond = crate::inst_a!(bytecode[idx]) as u8;
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
        func.blocks.push(BasicBlock {
            id,
            insts: Vec::new(),
            successors: Vec::new(),
        });
        id
    }

    fn append_inst(&mut self, func: &mut FunctionIR, block_id: usize, inst: Instruction) {
        func.blocks[block_id].insts.push(inst);
    }

    fn stmt_to_ir(
        &mut self,
        stmt: &Statement,
        func: &mut FunctionIR,
        current_block: usize,
    ) -> Result<usize, String> {
        let mut curr = current_block;
        match stmt {
            Statement::LetDecl(name, _ty, expr) => {
                let val_id = self.expr_to_ir(expr, func, curr)?;
                let id = func.alloc_val();
                self.append_inst(
                    func,
                    curr,
                    Instruction {
                        id,
                        op: Op::VarStore(name.clone(), val_id),
                    },
                );
            }
            Statement::Assign(name, expr) => {
                let val_id = self.expr_to_ir(expr, func, curr)?;
                let id = func.alloc_val();
                self.append_inst(
                    func,
                    curr,
                    Instruction {
                        id,
                        op: Op::VarStore(name.clone(), val_id),
                    },
                );
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
                self.append_inst(
                    func,
                    curr,
                    Instruction {
                        id: jmp_id,
                        op: Op::Jmp(cond_block),
                    },
                );

                // Cond block
                let cond_val = self.expr_to_ir(cond, func, cond_block)?;
                let jmpif_id = func.alloc_val();
                self.append_inst(
                    func,
                    cond_block,
                    Instruction {
                        id: jmpif_id,
                        op: Op::JmpIf(cond_val, body_block, end_block),
                    },
                );

                // Body block
                let mut body_curr = body_block;
                for s in body {
                    body_curr = self.stmt_to_ir(s, func, body_curr)?;
                }
                let loop_jmp_id = func.alloc_val();
                self.append_inst(
                    func,
                    body_curr,
                    Instruction {
                        id: loop_jmp_id,
                        op: Op::Jmp(cond_block),
                    },
                );

                curr = end_block;
            }
            Statement::If(cond, then_br, else_br) => {
                let cond_val = self.expr_to_ir(cond, func, curr)?;

                let then_block = self.new_block(func);
                let else_block = self.new_block(func);
                let end_block = self.new_block(func);

                let jmpif_id = func.alloc_val();
                self.append_inst(
                    func,
                    curr,
                    Instruction {
                        id: jmpif_id,
                        op: Op::JmpIf(cond_val, then_block, else_block),
                    },
                );

                let mut then_curr = then_block;
                for s in then_br {
                    then_curr = self.stmt_to_ir(s, func, then_curr)?;
                }
                let t_jmp_id = func.alloc_val();
                self.append_inst(
                    func,
                    then_curr,
                    Instruction {
                        id: t_jmp_id,
                        op: Op::Jmp(end_block),
                    },
                );

                let mut else_curr = else_block;
                for s in else_br {
                    else_curr = self.stmt_to_ir(s, func, else_curr)?;
                }
                let e_jmp_id = func.alloc_val();
                self.append_inst(
                    func,
                    else_curr,
                    Instruction {
                        id: e_jmp_id,
                        op: Op::Jmp(end_block),
                    },
                );

                curr = end_block;
            }
            _ => return Err(format!("Statement not supported yet: {:?}", stmt)),
        }
        Ok(curr)
    }

    fn expr_to_ir(
        &mut self,
        expr: &Expr,
        func: &mut FunctionIR,
        block_id: usize,
    ) -> Result<ValueId, String> {
        match expr {
            Expr::IntLiteral(val) => {
                let id = func.alloc_val();
                self.append_inst(
                    func,
                    block_id,
                    Instruction {
                        id,
                        op: Op::LoadImm(*val),
                    },
                );
                Ok(id)
            }
            Expr::Identifier(name) => {
                let id = func.alloc_val();
                self.append_inst(
                    func,
                    block_id,
                    Instruction {
                        id,
                        op: Op::VarLoad(name.clone()),
                    },
                );
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
                    BinaryOperator::Mod => Op::Mod(l_id, r_id),
                    BinaryOperator::Eq => Op::Eq(l_id, r_id),
                    BinaryOperator::Ne => Op::Ne(l_id, r_id),
                    BinaryOperator::Lt => Op::Lt(l_id, r_id),
                    BinaryOperator::Gt => Op::Gt(l_id, r_id),
                    BinaryOperator::Le => Op::Le(l_id, r_id),
                    BinaryOperator::Ge => Op::Ge(l_id, r_id),
                    BinaryOperator::And => Op::And(l_id, r_id),
                    BinaryOperator::Or => Op::Or(l_id, r_id),
                };
                self.append_inst(func, block_id, Instruction { id, op: ir_op });
                Ok(id)
            }
            Expr::Call(name, args) => {
                let mut arg_ids = Vec::new();
                for arg in args {
                    arg_ids.push(self.expr_to_ir(arg, func, block_id)?);
                }
                let id = func.alloc_val();
                self.append_inst(
                    func,
                    block_id,
                    Instruction {
                        id,
                        op: Op::Call(name.clone(), arg_ids),
                    },
                );
                Ok(id)
            }
            Expr::MacroCall(name, args) => {
                if name == "spawn" {
                    if let Some(Expr::IntLiteral(target_pc)) = args.first() {
                        let id = func.alloc_val();
                        self.append_inst(
                            func,
                            block_id,
                            Instruction {
                                id,
                                op: Op::Spawn(*target_pc as u16),
                            },
                        );
                        return Ok(id);
                    } else {
                        return Err("spawn! requires an integer literal for target PC".into());
                    }
                } else if name == "await" {
                    let task_id = self.expr_to_ir(&args[0], func, block_id)?;
                    let id = func.alloc_val();
                    self.append_inst(
                        func,
                        block_id,
                        Instruction {
                            id,
                            op: Op::Await(task_id),
                        },
                    );
                    return Ok(id);
                } else if name == "yield" {
                    let id = func.alloc_val();
                    self.append_inst(
                        func,
                        block_id,
                        Instruction {
                            id,
                            op: Op::Yield,
                        },
                    );
                    return Ok(id);
                }

                let mut arg_ids = Vec::new();
                for arg in args {
                    arg_ids.push(self.expr_to_ir(arg, func, block_id)?);
                }
                let id = func.alloc_val();
                self.append_inst(
                    func,
                    block_id,
                    Instruction {
                        id,
                        op: Op::MacroCall(name.clone(), arg_ids),
                    },
                );
                Ok(id)
            }
            _ => Err("Expression not supported in Phase 3".into()),
        }
    }
}
