#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::sgl::host_handlers::HostSocket;
use crate::sgl::vm::ScriptVm;
use sgl_macros::sgl_package;

#[sgl_package(name = "sgl_net", kind = "hardware")]
pub mod sgl_net {
    use super::*;

    /// Command 1: HttpGet - performs HTTP GET request
    #[sgl_cmd(id = 1)]
    pub fn http_get(vm: &mut ScriptVm, url: String) -> Result<String, u32> {
        if url.is_empty() {
            return Err(covopt_param!("M_19_23", 400));
        }

        if let Some(ctx) = vm.get_host_context()
            && let Some(response) = ctx.http_mock_routes.get(&url)
        {
            return Ok(response.clone());
        }

        Ok(format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nMock Response for {}",
            url
        ))
    }

    /// Command 2: HttpPost - performs HTTP POST request with body
    #[sgl_cmd(id = 2)]
    pub fn http_post(_vm: &mut ScriptVm, url: String, body: String) -> Result<String, u32> {
        let response_body = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{{\"posted_to\":\"{}\",\"bytes_received\":{}}}",
            url,
            body.len()
        );
        Ok(response_body)
    }

    /// Command 3: SocketConnect - connects socket to address and returns socket ID
    #[sgl_cmd(id = 3)]
    pub fn socket_connect(vm: &mut ScriptVm, addr: String) -> u32 {
        let target_address = if addr.is_empty() {
            String::from("127.0.0.1:8080")
        } else {
            addr
        };

        if let Some(ctx) = vm.get_host_context_mut() {
            let socket_id = ctx.next_socket_id;
            ctx.next_socket_id += 1;

            let host_socket = HostSocket {
                socket_id,
                address: target_address,
                is_connected: true,
                send_buffer: Vec::new(),
                receive_buffer: Vec::new(),
            };

            ctx.sockets.insert(socket_id, host_socket);
            socket_id
        } else {
            1
        }
    }

    /// Command 4: SocketSend - sends data buffer over socket
    #[sgl_cmd(id = 4)]
    pub fn socket_send(vm: &mut ScriptVm, socket_id: u32, data: Vec<u8>) -> u32 {
        let len = data.len();
        if let Some(ctx) = vm.get_host_context_mut() {
            if let Some(socket) = ctx.sockets.get_mut(&socket_id) {
                socket.send_buffer.extend_from_slice(&data);
                len as u32
            } else {
                0
            }
        } else {
            len as u32
        }
    }

    /// Command 5: SocketRecv - receives data into destination buffer address
    #[sgl_cmd(id = 5)]
    pub fn socket_recv(vm: &mut ScriptVm, socket_id: u32, buf_addr: u32) -> u32 {
        let recv_bytes = if let Some(ctx) = vm.get_host_context_mut() {
            if let Some(socket) = ctx.sockets.get_mut(&socket_id) {
                let buf = socket.receive_buffer.clone();
                socket.receive_buffer.clear();
                Some(buf)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(bytes) = recv_bytes {
            let len = bytes.len();
            if len > 0 {
                let _ = vm.write_bytes(buf_addr as usize, &bytes);
                return len as u32;
            }
        }
        0
    }

    /// Command 6: NetworkStatus - returns 1 if network is online, 0 otherwise
    #[sgl_cmd(id = 6)]
    pub fn network_status(vm: &mut ScriptVm) -> u32 {
        if let Some(ctx) = vm.get_host_context() {
            if ctx.network_online { 1 } else { 0 }
        } else {
            1
        }
    }
}

pub use self::sgl_net::*;
