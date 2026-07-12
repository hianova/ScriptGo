use alloc::vec::Vec;
use crate::instruction::{Instruction, OpCode};

/// Parses a string of assembly instructions into a Vec<Instruction>
pub fn parse_asm(source: &str) -> Vec<Instruction> {
    let mut code = Vec::new();
    
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        
        let mnemonic = parts[0].to_uppercase();
        
        let p1 = parts.get(1).and_then(|s| s.parse::<u16>().ok()).unwrap_or(0);
        let p2 = parts.get(2).and_then(|s| s.parse::<u16>().ok()).unwrap_or(0);
        let p3 = parts.get(3).and_then(|s| s.parse::<u16>().ok()).unwrap_or(0);
        
        let u8_p1 = (p1 & 0xFF) as u8;
        let u8_p2 = (p2 & 0xFF) as u8;
        let u8_p3 = (p3 & 0xFF) as u8;
        
        let p2_low = (p2 & 0xFF) as u8;
        let p2_high = ((p2 >> 8) & 0xFF) as u8;

        let inst = match mnemonic.as_str() {
            "HALT" => Instruction::new(OpCode::Halt as u8, 0, 0, 0),
            "LOADIMM" => Instruction::new(OpCode::LoadImm as u8, u8_p1, u8_p2, 0),
            "LOADIMM16" => Instruction::new(OpCode::LoadImm16 as u8, u8_p1, p2_low, p2_high),
            
            "ADD" => Instruction::new(OpCode::Add as u8, u8_p1, u8_p2, u8_p3),
            "SUB" => Instruction::new(OpCode::Sub as u8, u8_p1, u8_p2, u8_p3),
            "MUL" => Instruction::new(OpCode::Mul as u8, u8_p1, u8_p2, u8_p3),
            "DIV" => Instruction::new(OpCode::Div as u8, u8_p1, u8_p2, u8_p3),
            "MOD" => Instruction::new(OpCode::Mod as u8, u8_p1, u8_p2, u8_p3),
            
            "AND" => Instruction::new(OpCode::And as u8, u8_p1, u8_p2, u8_p3),
            "OR"  => Instruction::new(OpCode::Or as u8, u8_p1, u8_p2, u8_p3),
            "XOR" => Instruction::new(OpCode::Xor as u8, u8_p1, u8_p2, u8_p3),
            "SHL" => Instruction::new(OpCode::Shl as u8, u8_p1, u8_p2, u8_p3),
            "SHR" => Instruction::new(OpCode::Shr as u8, u8_p1, u8_p2, u8_p3),

            "JMP" => {
                let j_low = (p1 & 0xFF) as u8;
                let j_high = ((p1 >> 8) & 0xFF) as u8;
                Instruction::new(OpCode::Jmp as u8, 0, j_low, j_high)
            },
            "JMPIFZERO" => Instruction::new(OpCode::JmpIfZero as u8, u8_p1, p2_low, p2_high),
            "JMPIFEQ" => Instruction::new(OpCode::JmpIfEq as u8, u8_p1, u8_p2, u8_p3),
            "JMPIFLT" => Instruction::new(OpCode::JmpIfLt as u8, u8_p1, u8_p2, u8_p3),
            "JMPIFGT" => Instruction::new(OpCode::JmpIfGt as u8, u8_p1, u8_p2, u8_p3),

            "CALL" => {
                let c_low = (p1 & 0xFF) as u8;
                let c_high = ((p1 >> 8) & 0xFF) as u8;
                Instruction::new(OpCode::Call as u8, 0, c_low, c_high)
            },
            "RET" => Instruction::new(OpCode::Ret as u8, 0, 0, 0),

            _ => Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        };
        code.push(inst);
    }
    
    code
}
