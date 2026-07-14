use crate::compiler::ast::*;
use crate::compiler::ir::*;
use crate::compiler::optimizer::Optimizer;
use scriptgo_vm::instruction::{Instruction as VmInst, OpCode};
use alloc::vec::Vec;
use alloc::string::String;
use no_std_tool::collections::HashMap;

pub struct CodeGen {
    vars: HashMap<String, ValueId>,
    vm_regs: HashMap<ValueId, u8>,
    reg_counter: u8,
}

impl CodeGen {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            vm_regs: HashMap::new(),
            reg_counter: 1, // Reg 0 reserved
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

    pub fn compile(&mut self, prog: &Program) -> Result<Vec<VmInst>, String> {
        // Step 1: AST to IR
        let mut func_ir = FunctionIR::new(String::from("main"));
        let mut block = BasicBlock { id: 0, insts: Vec::new(), successors: Vec::new() };

        for stmt in &prog.statements {
            self.stmt_to_ir(stmt, &mut func_ir, &mut block)?;
        }
        func_ir.blocks.push(block);

        // Step 2: SSA IR Optimization (Phase 2 core)
        Optimizer::optimize(&mut func_ir);

        // Step 3: IR to Bytecode (Register Allocation & Code Emission)
        let mut bytecode = Vec::new();

        for block in &func_ir.blocks {
            for inst in &block.insts {
                // match if let guard (Rust 1.95+ feature)
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
                    Op::TensorMul(v1, v2) => {
                        // High-level abstraction fusion: directly emits NeuralCall (0xFF) to invoke vec101 via FFI
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        let r2 = self.get_reg(*v2);
                        // Using NeuralCall with specific command for TensorMul
                        bytecode.push(VmInst::new(OpCode::NeuralCall as u8, r, r1, r2));
                    }
                    Op::DbFilter(v1, _) => {
                        // Directly emitting FFI call for cdDB query execution
                        let r = self.alloc_reg(inst.id);
                        let r1 = self.get_reg(*v1);
                        bytecode.push(VmInst::new(OpCode::Call as u8, r, r1, 0)); // Simplified
                    }
                    Op::FfiCall { func_id: _func_id, args } => {
                        // Standard FFI 
                        let r = self.alloc_reg(inst.id);
                        let r1 = if !args.is_empty() { self.get_reg(args[0]) } else { 0 };
                        let r2 = if args.len() > 1 { self.get_reg(args[1]) } else { 0 };
                        // Simplified C-ABI Call mapping in VM
                        bytecode.push(VmInst::new(OpCode::UiCall as u8, r, r1, r2)); 
                    }
                    _ => {}
                }
            }
        }

        bytecode.push(VmInst::new(OpCode::Halt as u8, 0, 0, 0));
        Ok(bytecode)
    }

    fn stmt_to_ir(&mut self, stmt: &Statement, func: &mut FunctionIR, block: &mut BasicBlock) -> Result<(), String> {
        match stmt {
            Statement::LetDecl(name, _ty, expr) => {
                let val_id = self.expr_to_ir(expr, func, block)?;
                self.vars.insert(name.clone(), val_id);
            }
            Statement::ExprStmt(expr) => {
                self.expr_to_ir(expr, func, block)?;
            }
            _ => return Err("Statement not supported in Phase 2 IR generation".into()),
        }
        Ok(())
    }

    fn expr_to_ir(&mut self, expr: &Expr, func: &mut FunctionIR, block: &mut BasicBlock) -> Result<ValueId, String> {
        match expr {
            Expr::IntLiteral(val) => {
                let id = func.alloc_val();
                block.insts.push(Instruction { id, op: Op::LoadImm(*val) });
                Ok(id)
            }
            Expr::Identifier(name) => {
                if let Some(&val) = self.vars.get(name) {
                    Ok(val)
                } else {
                    Err("Undefined variable".into())
                }
            }
            Expr::BinaryOp(left, op, right) => {
                let l_id = self.expr_to_ir(left, func, block)?;
                let r_id = self.expr_to_ir(right, func, block)?;
                let id = func.alloc_val();
                let ir_op = match op {
                    BinaryOperator::Add => Op::Add(l_id, r_id),
                    BinaryOperator::Sub => Op::Sub(l_id, r_id),
                    BinaryOperator::Mul => Op::Mul(l_id, r_id),
                    BinaryOperator::Div => Op::Div(l_id, r_id),
                    _ => return Err("Unsupported operator in Phase 2".into()),
                };
                block.insts.push(Instruction { id, op: ir_op });
                Ok(id)
            }
            Expr::MethodCall(base, method_name, _args) => {
                let base_id = self.expr_to_ir(base, func, block)?;
                let id = func.alloc_val();
                if method_name == "filter" {
                    // Macro Expansion & Static Fusion for DB operations
                    block.insts.push(Instruction { id, op: Op::DbFilter(base_id, String::from("condition")) });
                }
                Ok(id)
            }
            _ => Err("Expression not supported in Phase 2 IR generation".into()),
        }
    }
}
