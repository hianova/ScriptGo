#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::sgl::vm::ScriptVm;

/// Categories and Identifiers for SysCall (OpCode 29)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SysCallCommand {
    FileRead = 1,
    FileWrite = 2,
    GetTimestamp = 3,
    GetEnv = 4,
    StringConcat = 5,
    StringLength = 6,
    StringSlice = 7,
    StringToUpper = 8,
    StringToLower = 9,
    Unknown = 255,
}

impl SysCallCommand {
    pub fn from_u32(val: u32) -> Self {
        match val {
            1 => Self::FileRead,
            2 => Self::FileWrite,
            3 => Self::GetTimestamp,
            4 => Self::GetEnv,
            5 => Self::StringConcat,
            6 => Self::StringLength,
            7 => Self::StringSlice,
            8 => Self::StringToUpper,
            9 => Self::StringToLower,
            _ => Self::Unknown,
        }
    }
}

/// Categories and Identifiers for HardwareCall (OpCode 35)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum HardwareCallCommand {
    HttpGet = 1,
    HttpPost = 2,
    SocketConnect = 3,
    SocketSend = 4,
    SocketRecv = 5,
    NetworkStatus = 6,
    Unknown = 255,
}

impl HardwareCallCommand {
    pub fn from_u32(val: u32) -> Self {
        match val {
            1 => Self::HttpGet,
            2 => Self::HttpPost,
            3 => Self::SocketConnect,
            4 => Self::SocketSend,
            5 => Self::SocketRecv,
            6 => Self::NetworkStatus,
            _ => Self::Unknown,
        }
    }
}

/// Internal structure representing simulated network sockets
#[derive(Debug, Clone, Default)]
pub struct HostSocket {
    pub socket_id: u32,
    pub address: String,
    pub is_connected: bool,
    pub send_buffer: Vec<u8>,
    pub receive_buffer: Vec<u8>,
}

/// Stateful Host Context attached to ScriptVm for persistent host operations
#[derive(Debug, Clone)]
pub struct HostContext {
    pub virtual_filesystem: BTreeMap<String, Vec<u8>>,
    pub environment_variables: BTreeMap<String, String>,
    pub http_mock_routes: BTreeMap<String, String>,
    pub sockets: BTreeMap<u32, HostSocket>,
    pub next_socket_id: u32,
    pub network_online: bool,
    pub scratch_heap_pointer: usize,
    pub timestamp_counter: u64,
}

impl Default for HostContext {
    fn default() -> Self {
        Self::new()
    }
}

impl HostContext {
    pub fn new() -> Self {
        let mut context = Self {
            virtual_filesystem: BTreeMap::new(),
            environment_variables: BTreeMap::new(),
            http_mock_routes: BTreeMap::new(),
            sockets: BTreeMap::new(),
            next_socket_id: 1,
            network_online: true,
            scratch_heap_pointer: covopt_param!("M_108_34", 512),
            timestamp_counter: covopt_param!("M_109_31", 1700000000000),
        };

        // Populate initial default environment and mock routes
        context.environment_variables.insert("ENV".to_string(), "production".to_string());
        context.environment_variables.insert("USER".to_string(), "sgl_runner".to_string());
        context.virtual_filesystem.insert(
            "test.txt".to_string(),
            b"Hello SGL Virtual Filesystem!".to_vec(),
        );
        context.http_mock_routes.insert(
            "https://api.sgl.internal/status".to_string(),
            r#"{"status":"online","service":"sgl-runtime"}"#.to_string(),
        );

        context
    }

    /// Allocate dynamic buffer memory inside VM memory range (512..1000)
    pub fn allocate_vm_memory(&mut self, size: usize) -> usize {
        let aligned_size = (size + covopt_param!("M_129_35", 3)) & !covopt_param!("M_129_41", 3);
        if self.scratch_heap_pointer + aligned_size > covopt_param!("M_130_54", 1000) {
            self.scratch_heap_pointer = covopt_param!("M_131_40", 512);
        }
        let address = self.scratch_heap_pointer;
        self.scratch_heap_pointer += aligned_size;
        address
    }

    /// Dispatch SysCall (OpCode 29)
    pub fn dispatch_syscall(
        &mut self,
        vm: &mut ScriptVm,
        destination_register: usize,
        subcommand_or_argument: usize,
        argument_or_pointer: usize,
    ) {
        let reg_subcmd = vm.registers[subcommand_or_argument];
        let direct_cmd = subcommand_or_argument as u32;

        let command = match SysCallCommand::from_u32(reg_subcmd) {
            SysCallCommand::Unknown => SysCallCommand::from_u32(direct_cmd),
            cmd => cmd,
        };

        match command {
            SysCallCommand::FileRead => {
                self.handle_file_read(vm, destination_register, argument_or_pointer);
            }
            SysCallCommand::FileWrite => {
                self.handle_file_write(vm, destination_register, argument_or_pointer);
            }
            SysCallCommand::GetTimestamp => {
                self.handle_get_timestamp(vm, destination_register);
            }
            SysCallCommand::GetEnv => {
                self.handle_get_env(vm, destination_register, argument_or_pointer);
            }
            SysCallCommand::StringConcat => {
                self.handle_string_concat(vm, destination_register, argument_or_pointer);
            }
            SysCallCommand::StringLength => {
                self.handle_string_length(vm, destination_register, argument_or_pointer);
            }
            SysCallCommand::StringSlice => {
                self.handle_string_slice(vm, destination_register, argument_or_pointer);
            }
            SysCallCommand::StringToUpper => {
                self.handle_string_to_upper(vm, destination_register, argument_or_pointer);
            }
            SysCallCommand::StringToLower => {
                self.handle_string_to_lower(vm, destination_register, argument_or_pointer);
            }
            SysCallCommand::Unknown => {
                vm.registers[destination_register] = 0;
            }
        }
    }

    /// Dispatch HardwareCall (OpCode 35)
    pub fn dispatch_hardware_call(
        &mut self,
        vm: &mut ScriptVm,
        destination_register: usize,
        subcommand_or_argument: usize,
        argument_or_pointer: usize,
    ) {
        let reg_subcmd = vm.registers[subcommand_or_argument];
        let direct_b = subcommand_or_argument as u32;
        let direct_c = argument_or_pointer as u32;

        let command = match HardwareCallCommand::from_u32(reg_subcmd) {
            HardwareCallCommand::Unknown => match HardwareCallCommand::from_u32(direct_b) {
                HardwareCallCommand::Unknown => HardwareCallCommand::from_u32(direct_c),
                cmd => cmd,
            },
            cmd => cmd,
        };

        match command {
            HardwareCallCommand::HttpGet => {
                self.handle_http_get(vm, destination_register, argument_or_pointer);
            }
            HardwareCallCommand::HttpPost => {
                self.handle_http_post(vm, destination_register, argument_or_pointer);
            }
            HardwareCallCommand::SocketConnect => {
                self.handle_socket_connect(vm, destination_register, argument_or_pointer);
            }
            HardwareCallCommand::SocketSend => {
                self.handle_socket_send(vm, destination_register, argument_or_pointer);
            }
            HardwareCallCommand::SocketRecv => {
                self.handle_socket_recv(vm, destination_register, argument_or_pointer);
            }
            HardwareCallCommand::NetworkStatus => {
                self.handle_network_status(vm, destination_register);
            }
            HardwareCallCommand::Unknown => {
                vm.registers[destination_register] = 0;
            }
        }
    }

    // --- SysCall Handlers ---

    fn handle_file_read(&mut self, vm: &mut ScriptVm, dest_reg: usize, path_ptr_reg: usize) {
        let path_address = if path_ptr_reg < covopt_param!("M_236_45", 256) {
            vm.registers[path_ptr_reg] as usize
        } else {
            path_ptr_reg
        };

        if let Ok(path_string) = vm.read_string(path_address, None)
            && let Some(content_bytes) = self.virtual_filesystem.get(&path_string).cloned()
        {
            let allocation_address = self.allocate_vm_memory(content_bytes.len() + 1);
            if vm.write_bytes(allocation_address, &content_bytes).is_ok() {
                // Null terminate if possible
                let _ = vm.write_bytes(allocation_address + content_bytes.len(), &[0]);
                vm.registers[dest_reg] = allocation_address as u32;
                return;
            }
        }
        vm.registers[dest_reg] = 0;
    }

    fn handle_file_write(&mut self, vm: &mut ScriptVm, dest_reg: usize, arg_reg: usize) {
        let path_address = vm.registers[arg_reg] as usize;
        let data_address = vm.registers[arg_reg.wrapping_add(1) % covopt_param!("M_258_66", 256)] as usize;
        let data_length = vm.registers[arg_reg.wrapping_add(2) % covopt_param!("M_259_65", 256)] as usize;

        if let Ok(path_string) = vm.read_string(path_address, None) {
            let data_bytes = if data_length > 0 {
                vm.read_bytes(data_address, data_length, false)
                    .unwrap_or_default()
            } else {
                vm.read_bytes(data_address, 0, true).unwrap_or_default()
            };

            let bytes_written = data_bytes.len();
            self.virtual_filesystem.insert(path_string, data_bytes);
            vm.registers[dest_reg] = bytes_written as u32;
        } else {
            vm.registers[dest_reg] = 0;
        }
    }

    fn handle_get_timestamp(&mut self, vm: &mut ScriptVm, dest_reg: usize) {
        self.timestamp_counter += covopt_param!("M_278_34", 10);
        vm.registers[dest_reg] = (self.timestamp_counter & covopt_param!("M_279_59", 4294967295)) as u32;
    }

    fn handle_get_env(&mut self, vm: &mut ScriptVm, dest_reg: usize, key_ptr_reg: usize) {
        let key_address = if key_ptr_reg < covopt_param!("M_283_43", 256) {
            vm.registers[key_ptr_reg] as usize
        } else {
            key_ptr_reg
        };

        if let Ok(key_string) = vm.read_string(key_address, None)
            && let Some(value_string) = self.environment_variables.get(&key_string).cloned()
        {
            let allocation_address = self.allocate_vm_memory(value_string.len() + 1);
            if vm
                .write_string(allocation_address, &value_string, true)
                .is_ok()
            {
                vm.registers[dest_reg] = allocation_address as u32;
                return;
            }
        }
        vm.registers[dest_reg] = 0;
    }

    fn handle_string_concat(&mut self, vm: &mut ScriptVm, dest_reg: usize, arg_reg: usize) {
        let str1_addr = if arg_reg < covopt_param!("M_305_37", 256) && vm.registers[arg_reg] > 0 {
            vm.registers[arg_reg] as usize
        } else {
            arg_reg
        };
        let second_reg = arg_reg.wrapping_add(1) % covopt_param!("M_310_51", 256);
        let str2_addr = if second_reg < covopt_param!("M_311_40", 256) && vm.registers[second_reg] > 0 {
            vm.registers[second_reg] as usize
        } else {
            second_reg
        };

        let str1 = vm.read_string(str1_addr, None).unwrap_or_default();
        let str2 = vm.read_string(str2_addr, None).unwrap_or_default();

        let concatenated = format!("{}{}", str1, str2);
        let allocation_address = self.allocate_vm_memory(concatenated.len() + 1);
        if vm
            .write_string(allocation_address, &concatenated, true)
            .is_ok()
        {
            vm.registers[dest_reg] = allocation_address as u32;
        } else {
            vm.registers[dest_reg] = 0;
        }
    }

    fn handle_string_length(&mut self, vm: &mut ScriptVm, dest_reg: usize, str_ptr_reg: usize) {
        let str_addr = if str_ptr_reg < covopt_param!("M_333_40", 256) && vm.registers[str_ptr_reg] > 0 {
            vm.registers[str_ptr_reg] as usize
        } else {
            str_ptr_reg
        };

        if let Ok(text) = vm.read_string(str_addr, None) {
            vm.registers[dest_reg] = text.chars().count() as u32;
        } else {
            vm.registers[dest_reg] = 0;
        }
    }

    fn handle_string_slice(&mut self, vm: &mut ScriptVm, dest_reg: usize, arg_reg: usize) {
        let str_addr = if arg_reg < covopt_param!("M_347_36", 256) && vm.registers[arg_reg] > 0 {
            vm.registers[arg_reg] as usize
        } else {
            arg_reg
        };
        let start_reg = arg_reg.wrapping_add(1) % covopt_param!("M_352_50", 256);
        let end_reg = arg_reg.wrapping_add(2) % covopt_param!("M_353_48", 256);

        let start_idx = vm.registers[start_reg] as usize;
        let end_idx = vm.registers[end_reg] as usize;

        if let Ok(text) = vm.read_string(str_addr, None) {
            let char_vec: Vec<char> = text.chars().collect();
            let start = start_idx.min(char_vec.len());
            let end = end_idx.min(char_vec.len()).max(start);

            let sliced: String = char_vec[start..end].iter().collect();
            let allocation_address = self.allocate_vm_memory(sliced.len() + 1);
            if vm.write_string(allocation_address, &sliced, true).is_ok() {
                vm.registers[dest_reg] = allocation_address as u32;
                return;
            }
        }
        vm.registers[dest_reg] = 0;
    }

    fn handle_string_to_upper(&mut self, vm: &mut ScriptVm, dest_reg: usize, str_ptr_reg: usize) {
        let str_addr = if str_ptr_reg < covopt_param!("M_374_40", 256) && vm.registers[str_ptr_reg] > 0 {
            vm.registers[str_ptr_reg] as usize
        } else {
            str_ptr_reg
        };

        if let Ok(text) = vm.read_string(str_addr, None) {
            let upper = text.to_uppercase();
            let allocation_address = self.allocate_vm_memory(upper.len() + 1);
            if vm.write_string(allocation_address, &upper, true).is_ok() {
                vm.registers[dest_reg] = allocation_address as u32;
                return;
            }
        }
        vm.registers[dest_reg] = 0;
    }

    fn handle_string_to_lower(&mut self, vm: &mut ScriptVm, dest_reg: usize, str_ptr_reg: usize) {
        let str_addr = if str_ptr_reg < covopt_param!("M_392_40", 256) && vm.registers[str_ptr_reg] > 0 {
            vm.registers[str_ptr_reg] as usize
        } else {
            str_ptr_reg
        };

        if let Ok(text) = vm.read_string(str_addr, None) {
            let lower = text.to_lowercase();
            let allocation_address = self.allocate_vm_memory(lower.len() + 1);
            if vm.write_string(allocation_address, &lower, true).is_ok() {
                vm.registers[dest_reg] = allocation_address as u32;
                return;
            }
        }
        vm.registers[dest_reg] = 0;
    }

    // --- HardwareCall Handlers ---

    fn handle_http_get(&mut self, vm: &mut ScriptVm, dest_reg: usize, url_ptr_reg: usize) {
        let url_addr = if url_ptr_reg < covopt_param!("M_412_40", 256) && vm.registers[url_ptr_reg] > 0 {
            vm.registers[url_ptr_reg] as usize
        } else {
            url_ptr_reg
        };

        if let Ok(url) = vm.read_string(url_addr, None) {
            let response_body = if let Some(route_response) = self.http_mock_routes.get(&url) {
                route_response.clone()
            } else {
                format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nMock Response for {}", url)
            };

            let allocation_address = self.allocate_vm_memory(response_body.len() + 1);
            if vm
                .write_string(allocation_address, &response_body, true)
                .is_ok()
            {
                vm.registers[dest_reg] = allocation_address as u32;
                return;
            }
        }
        vm.registers[dest_reg] = 0;
    }

    fn handle_http_post(&mut self, vm: &mut ScriptVm, dest_reg: usize, arg_reg: usize) {
        let url_addr = if arg_reg < covopt_param!("M_438_36", 256) && vm.registers[arg_reg] > 0 {
            vm.registers[arg_reg] as usize
        } else {
            arg_reg
        };
        let body_reg = arg_reg.wrapping_add(1) % covopt_param!("M_443_49", 256);
        let body_addr = if body_reg < covopt_param!("M_444_38", 256) && vm.registers[body_reg] > 0 {
            vm.registers[body_reg] as usize
        } else {
            body_reg
        };

        let url = vm.read_string(url_addr, None).unwrap_or_default();
        let body = vm.read_string(body_addr, None).unwrap_or_default();

        let response_body = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{{\"posted_to\":\"{}\",\"bytes_received\":{}}}",
            url,
            body.len()
        );

        let allocation_address = self.allocate_vm_memory(response_body.len() + 1);
        if vm
            .write_string(allocation_address, &response_body, true)
            .is_ok()
        {
            vm.registers[dest_reg] = allocation_address as u32;
        } else {
            vm.registers[dest_reg] = 0;
        }
    }

    fn handle_socket_connect(&mut self, vm: &mut ScriptVm, dest_reg: usize, addr_reg: usize) {
        let addr_ptr = if addr_reg < covopt_param!("M_471_37", 256) && vm.registers[addr_reg] > 0 {
            vm.registers[addr_reg] as usize
        } else {
            addr_reg
        };

        let target_address = vm
            .read_string(addr_ptr, None)
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string());

        let socket_id = self.next_socket_id;
        self.next_socket_id += 1;

        let host_socket = HostSocket {
            socket_id,
            address: target_address,
            is_connected: true,
            send_buffer: Vec::new(),
            receive_buffer: Vec::new(),
        };

        self.sockets.insert(socket_id, host_socket);
        vm.registers[dest_reg] = socket_id;
    }

    fn handle_socket_send(&mut self, vm: &mut ScriptVm, dest_reg: usize, arg_reg: usize) {
        let socket_id = if arg_reg < covopt_param!("M_497_37", 256) { vm.registers[arg_reg] } else { arg_reg as u32 };
        let buf_reg = arg_reg.wrapping_add(1) % covopt_param!("M_498_48", 256);
        let buf_addr = vm.registers[buf_reg] as usize;
        let len_reg = arg_reg.wrapping_add(2) % covopt_param!("M_500_48", 256);
        let buf_len = vm.registers[len_reg] as usize;

        let bytes = vm.read_bytes(buf_addr, buf_len, buf_len == 0).unwrap_or_default();

        if let Some(socket) = self.sockets.get_mut(&socket_id) {
            let sent_count = bytes.len();
            socket.send_buffer.extend_from_slice(&bytes);
            vm.registers[dest_reg] = sent_count as u32;
        } else {
            vm.registers[dest_reg] = 0;
        }
    }

    fn handle_socket_recv(&mut self, vm: &mut ScriptVm, dest_reg: usize, arg_reg: usize) {
        let socket_id = if arg_reg < covopt_param!("M_515_37", 256) { vm.registers[arg_reg] } else { arg_reg as u32 };
        let buf_reg = arg_reg.wrapping_add(1) % covopt_param!("M_516_48", 256);
        let dest_buf_addr = vm.registers[buf_reg] as usize;

        if let Some(socket) = self.sockets.get_mut(&socket_id) {
            let bytes_to_read = socket.receive_buffer.len();
            if bytes_to_read > 0 {
                let _ = vm.write_bytes(dest_buf_addr, &socket.receive_buffer);
                socket.receive_buffer.clear();
                vm.registers[dest_reg] = bytes_to_read as u32;
                return;
            }
        }
        vm.registers[dest_reg] = 0;
    }

    fn handle_network_status(&mut self, vm: &mut ScriptVm, dest_reg: usize) {
        vm.registers[dest_reg] = if self.network_online { 1 } else { 0 };
    }
}
