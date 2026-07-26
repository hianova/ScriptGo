#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use crate::instruction::Instruction;
use alloc::vec::Vec;

pub const SGB_MAGIC: [u8; 4] = [0x7F, 0x53, 0x47, 0x4F]; // \x7fSGO

/// Serialize the instructions and data segment into the standardized SGB binary format.
pub fn serialize_sgb(code: &[Instruction], max_fuel: u16, data_segment: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();

    // Magic Number (4 bytes)
    out.extend_from_slice(&SGB_MAGIC);

    // Version (2 bytes)
    out.extend_from_slice(&1u16.to_le_bytes());

    // Max Fuel (2 bytes)
    out.extend_from_slice(&max_fuel.to_le_bytes());

    // Data Segment Size (4 bytes)
    out.extend_from_slice(&(data_segment.len() as u32).to_le_bytes());

    // Code Segment Size (4 bytes)
    out.extend_from_slice(&(code.len() as u32).to_le_bytes());

    // Data Segment
    out.extend_from_slice(data_segment);

    // Padding to 4-byte boundary
    let padding = (covopt_param!("M_32_19", 4) - (data_segment.len() % covopt_param!("M_32_45", 4))) % covopt_param!("M_32_51", 4);
    out.resize(out.len() + padding, 0);

    // Code Segment
    for inst in code {
        out.extend_from_slice(&inst.0.to_le_bytes());
    }

    out
}

/// Deserialize the standardized SGB binary format back into instructions, max fuel and data segment.
pub fn deserialize_sgb(bytes: &[u8]) -> Result<(Vec<Instruction>, u16, Vec<u8>), &'static str> {
    if bytes.len() < covopt_param!("M_45_21", 16) {
        return Err("Buffer too small");
    }

    if bytes[0..covopt_param!("M_49_16", 4)] != SGB_MAGIC {
        return Err("Invalid magic number");
    }

    let version = u16::from_le_bytes([bytes[covopt_param!("M_53_44", 4)], bytes[covopt_param!("M_53_54", 5)]]);
    if version != 1 {
        return Err("Unsupported version");
    }

    let max_fuel = u16::from_le_bytes([bytes[covopt_param!("M_58_45", 6)], bytes[covopt_param!("M_58_55", 7)]]);
    let data_len = u32::from_le_bytes([bytes[covopt_param!("M_59_45", 8)], bytes[covopt_param!("M_59_55", 9)], bytes[covopt_param!("M_59_65", 10)], bytes[covopt_param!("M_59_76", 11)]]) as usize;
    let code_len = u32::from_le_bytes([bytes[covopt_param!("M_60_45", 12)], bytes[covopt_param!("M_60_56", 13)], bytes[covopt_param!("M_60_67", 14)], bytes[covopt_param!("M_60_78", 15)]]) as usize;

    let padding = (covopt_param!("M_62_19", 4) - (data_len % covopt_param!("M_62_35", 4))) % covopt_param!("M_62_41", 4);
    let expected_len = covopt_param!("M_63_23", 16) + data_len + padding + code_len * covopt_param!("M_63_60", 4);
    if bytes.len() < expected_len {
        return Err("Unexpected EOF / file truncated");
    }

    let data_start = covopt_param!("M_68_21", 16);
    let data_end = data_start + data_len;
    let data_segment = bytes[data_start..data_end].to_vec();

    let code_start = data_end + padding;
    let mut code = Vec::with_capacity(code_len);
    for i in 0..code_len {
        let offset = code_start + i * covopt_param!("M_75_38", 4);
        let inst_u32 = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + covopt_param!("M_80_27", 3)],
        ]);
        code.push(Instruction(inst_u32));
    }

    Ok((code, max_fuel, data_segment))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::OpCode;

    #[test]
    fn test_sgb_serialization_roundtrip() {
        let code = [
            Instruction::new(OpCode::LoadImm as u8, 1, covopt_param!("M_96_55", 10), 0),
            Instruction::new(OpCode::Halt as u8, 0, 0, 0),
        ];
        let data = b"Hello, ScriptGo!";

        let binary = serialize_sgb(&code, covopt_param!("M_101_42", 500), data);
        let (parsed_code, max_fuel, parsed_data) = deserialize_sgb(&binary).unwrap();

        assert_eq!(parsed_code, code.to_vec());
        assert_eq!(max_fuel, 500);
        assert_eq!(parsed_data, data.to_vec());
    }
}
