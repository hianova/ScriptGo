use alloc::vec::Vec;
use crate::instruction::{Instruction, OpCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsmError {
    InvalidMnemonic { line: usize },
}

/// Parses a string of assembly instructions into a Vec<Instruction>
pub fn parse_asm(source: &str) -> Result<Vec<Instruction>, AsmError> {
    let mut code = Vec::new();
    
    for (line_idx, line) in source.lines().enumerate() {
        let line_num = line_idx + 1;
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

            "FADD" => Instruction::new(OpCode::FAdd as u8, u8_p1, u8_p2, u8_p3),
            "FSUB" => Instruction::new(OpCode::FSub as u8, u8_p1, u8_p2, u8_p3),
            "FMUL" => Instruction::new(OpCode::FMul as u8, u8_p1, u8_p2, u8_p3),
            "FDIV" => Instruction::new(OpCode::FDiv as u8, u8_p1, u8_p2, u8_p3),
            
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
            "JMPIFFLT" => Instruction::new(OpCode::JmpIfFLt as u8, u8_p1, u8_p2, u8_p3),
            "JMPIFFGT" => Instruction::new(OpCode::JmpIfFGt as u8, u8_p1, u8_p2, u8_p3),

            "CALL" => {
                let c_low = (p1 & 0xFF) as u8;
                let c_high = ((p1 >> 8) & 0xFF) as u8;
                Instruction::new(OpCode::Call as u8, 0, c_low, c_high)
            },
            "RET" => Instruction::new(OpCode::Ret as u8, 0, 0, 0),

            "PRINTREG" => Instruction::new(OpCode::PrintReg as u8, u8_p1, 0, 0),
            
            "LOAD" => Instruction::new(OpCode::Load as u8, u8_p1, u8_p2, u8_p3),
            "STORE" => Instruction::new(OpCode::Store as u8, u8_p1, u8_p2, u8_p3),

            "EXP" => Instruction::new(OpCode::Exp as u8, u8_p1, u8_p2, 0),
            "RSQRT" => Instruction::new(OpCode::Rsqrt as u8, u8_p1, u8_p2, 0),
            "SILU" => Instruction::new(OpCode::Silu as u8, u8_p1, u8_p2, 0),

            "UICALL" => Instruction::new(OpCode::UiCall as u8, u8_p1, u8_p2, u8_p3),
            "NEURALCALL" => Instruction::new(OpCode::NeuralCall as u8, u8_p1, u8_p2, u8_p3),

            _ => return Err(AsmError::InvalidMnemonic { line: line_num }),
        };
        code.push(inst);
    }
    
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_parse() {
        let source = "LOADIMM 1 5\nADD 2 1 1\nPRINTREG 2\nHALT";
        let result = parse_asm(source);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert_eq!(code.len(), 4);
        assert_eq!(code[0].opcode(), OpCode::LoadImm as u8);
        assert_eq!(code[2].opcode(), OpCode::PrintReg as u8);
    }

    #[test]
    fn test_invalid_parse() {
        let source = "LOADIMM 1 5\nINVALID_OP 2 1 1\nHALT";
        let result = parse_asm(source);
        assert_eq!(result, Err(AsmError::InvalidMnemonic { line: 2 }));
    }
}

