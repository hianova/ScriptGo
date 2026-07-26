#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::sgl::vm::ScriptVm;
use sgl_macros::sgl_package;

#[sgl_package(name = "sgl_io", kind = "syscall")]
pub mod sgl_io {
    use super::*;

    /// Command 1: FileRead - reads file content from virtual filesystem or host
    #[sgl_cmd(id = 1)]
    pub fn file_read(vm: &mut ScriptVm, path: String) -> Result<String, u32> {
        if path.is_empty() {
            return Err(covopt_param!("M_18_23", 400));
        }

        if let Some(ctx) = vm.get_host_context()
            && let Some(content_bytes) = ctx.virtual_filesystem.get(&path)
        {
            if let Ok(content_str) = String::from_utf8(content_bytes.clone()) {
                return Ok(content_str);
            } else {
                return Ok(String::from_utf8_lossy(content_bytes).into_owned());
            }
        }

        Err(covopt_param!("M_31_12", 404))
    }

    /// Command 2: FileWrite - writes bytes to virtual filesystem path
    #[sgl_cmd(id = 2)]
    pub fn file_write(vm: &mut ScriptVm, path: String, data: Vec<u8>) -> u32 {
        let bytes_written = data.len();
        if let Some(ctx) = vm.get_host_context_mut() {
            ctx.virtual_filesystem.insert(path, data);
        }
        bytes_written as u32
    }

    /// Command 3: GetTimestamp - gets system timestamp in milliseconds
    #[sgl_cmd(id = 3)]
    pub fn get_timestamp(vm: &mut ScriptVm) -> u32 {
        if let Some(ctx) = vm.get_host_context_mut() {
            ctx.timestamp_counter += covopt_param!("M_48_37", 10);
            (ctx.timestamp_counter & covopt_param!("M_49_37", 4294967295)) as u32
        } else {
            covopt_param!("M_51_12", 1700000000)
        }
    }

    /// Command 4: GetEnv - gets environment variable value by key
    #[sgl_cmd(id = 4)]
    pub fn get_env(vm: &mut ScriptVm, key: String) -> Result<String, u32> {
        if key.is_empty() {
            return Err(covopt_param!("M_59_23", 400));
        }

        if let Some(ctx) = vm.get_host_context()
            && let Some(val) = ctx.environment_variables.get(&key)
        {
            return Ok(val.clone());
        }

        Err(covopt_param!("M_68_12", 404))
    }

    /// Command 5: StringConcat - concatenates two strings
    #[sgl_cmd(id = 5)]
    pub fn string_concat(_vm: &mut ScriptVm, str1: String, str2: String) -> String {
        format!("{}{}", str1, str2)
    }

    /// Command 6: StringLength - returns character count of string
    #[sgl_cmd(id = 6)]
    pub fn string_length(_vm: &mut ScriptVm, str_val: String) -> u32 {
        str_val.chars().count() as u32
    }

    /// Command 7: StringSlice - returns substring slice from start to end character index
    #[sgl_cmd(id = 7)]
    pub fn string_slice(_vm: &mut ScriptVm, str_val: String, start: u32, end: u32) -> String {
        let char_vec: Vec<char> = str_val.chars().collect();
        let s = (start as usize).min(char_vec.len());
        let e = (end as usize).min(char_vec.len()).max(s);
        char_vec[s..e].iter().collect()
    }

    /// Command 8: StringToUpper - converts string to uppercase
    #[sgl_cmd(id = 8)]
    pub fn string_to_upper(_vm: &mut ScriptVm, str_val: String) -> String {
        str_val.to_uppercase()
    }

    /// Command 9: StringToLower - converts string to lowercase
    #[sgl_cmd(id = 9)]
    pub fn string_to_lower(_vm: &mut ScriptVm, str_val: String) -> String {
        str_val.to_lowercase()
    }
}

pub use self::sgl_io::*;
