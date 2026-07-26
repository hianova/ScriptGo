#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::sgl::vm::{ScriptVm, VmError};

/// Represents a mounted UI component within the Tauri UI Engine
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiComponent {
    pub id: u32,
    pub component_type: String,
    pub mounted: bool,
    pub props: BTreeMap<String, String>,
    pub event_listeners: Vec<String>,
    pub sync_count: u32,
}

impl UiComponent {
    pub fn new(id: u32, component_type: String) -> Self {
        Self {
            id,
            component_type,
            mounted: true,
            props: BTreeMap::new(),
            event_listeners: Vec::new(),
            sync_count: 0,
        }
    }
}

/// Represents an IPC event signal sent over Tauri IPC channel
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiEvent {
    pub event_id: u32,
    pub component_id: u32,
    pub channel: String,
    pub payload: String,
}

/// Stream buffer for rendering and large document streaming (e.g. 100MB Markdown documents)
#[derive(Debug, Clone, Default)]
pub struct StreamBuffer {
    pub chunks: Vec<Vec<u8>>,
    pub total_bytes: usize,
    pub is_complete: bool,
}

impl StreamBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_chunk(&mut self, chunk: &[u8]) {
        self.total_bytes += chunk.len();
        self.chunks.push(chunk.to_vec());
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
        self.total_bytes = 0;
        self.is_complete = false;
    }

    pub fn assemble(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(self.total_bytes);
        for chunk in &self.chunks {
            buffer.extend_from_slice(chunk);
        }
        buffer
    }
}

/// Stateful Host Engine handling Tauri UI IPC actions (UiCall OpCode 36)
#[derive(Debug, Clone, Default)]
pub struct UiDispatcher {
    pub components: BTreeMap<u32, UiComponent>,
    pub event_listeners: BTreeMap<String, u32>,
    pub event_queue: Vec<UiEvent>,
    pub stream_buffer: StreamBuffer,
    pub mount_count: u32,
    pub event_signal_count: u32,
    pub prop_update_count: u32,
    pub render_count: u32,
    pub next_event_id: u32,
    pub last_rendered_output: Option<String>,
}

impl UiDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.components.clear();
        self.event_listeners.clear();
        self.event_queue.clear();
        self.stream_buffer.clear();
        self.mount_count = 0;
        self.event_signal_count = 0;
        self.prop_update_count = 0;
        self.render_count = 0;
        self.next_event_id = 0;
        self.last_rendered_output = None;
    }

    /// Dispatch UiCall instruction (OpCode 36)
    pub fn dispatch(
        &mut self,
        vm: &ScriptVm,
        reg_a: usize,
        reg_b: usize,
        reg_c: usize,
    ) -> Result<(), VmError> {
        let val_a = if reg_a < covopt_param!("M_122_31", 256) { vm.registers[reg_a] } else { 0 };
        let val_b = if reg_b < covopt_param!("M_123_31", 256) { vm.registers[reg_b] } else { 0 };
        let val_c = if reg_c < covopt_param!("M_124_31", 256) { vm.registers[reg_c] } else { 0 };

        // Determine command action: 1 = Mount, 2 = Event Listener, 3 = Prop Update, 4 = Render/Stream
        let command = if (1..=covopt_param!("M_127_30", 4)).contains(&val_b) {
            val_b
        } else if (1..=covopt_param!("M_129_23", 4)).contains(&(reg_b as u32)) {
            reg_b as u32
        } else {
            1
        };

        // Determine target component ID
        let component_id = if val_a != 0 {
            val_a
        } else if reg_a != 0 {
            reg_a as u32
        } else {
            1
        };

        // Determine memory payload address or parameter
        let payload_address = if val_c != 0 {
            val_c as usize
        } else {
            reg_c
        };

        match command {
            1 => self.handle_mount(vm, component_id, payload_address)?,
            2 => self.handle_event_listener(vm, component_id, payload_address)?,
            3 => self.handle_prop_update(vm, component_id, payload_address)?,
            4 => self.handle_render_stream(vm, component_id, payload_address)?,
            _ => {}
        }

        Ok(())
    }

    fn handle_mount(
        &mut self,
        vm: &ScriptVm,
        component_id: u32,
        payload_address: usize,
    ) -> Result<(), VmError> {
        let type_string = if payload_address != 0 {
            vm.read_string(payload_address, Some(covopt_param!("M_169_49", 64)))
                .unwrap_or_else(|_| "container".to_string())
        } else {
            "container".to_string()
        };

        let component_type = if type_string.is_empty() {
            "container".to_string()
        } else {
            type_string
        };

        let component = UiComponent::new(component_id, component_type);
        self.components.insert(component_id, component);
        self.mount_count += 1;
        Ok(())
    }

    fn handle_event_listener(
        &mut self,
        vm: &ScriptVm,
        component_id: u32,
        payload_address: usize,
    ) -> Result<(), VmError> {
        let channel_name = if payload_address != 0 {
            vm.read_string(payload_address, Some(covopt_param!("M_194_49", 128)))
                .unwrap_or_else(|_| format!("tauri://ipc/channel_{}", component_id))
        } else {
            format!("tauri://ipc/channel_{}", component_id)
        };

        let channel = if channel_name.is_empty() {
            format!("tauri://ipc/channel_{}", component_id)
        } else {
            channel_name
        };

        if let Some(comp) = self.components.get_mut(&component_id)
            && !comp.event_listeners.contains(&channel)
        {
            comp.event_listeners.push(channel.clone());
        }
        self.event_listeners.insert(channel.clone(), component_id);

        self.next_event_id += 1;
        let event = UiEvent {
            event_id: self.next_event_id,
            component_id,
            channel,
            payload: format!("event_signal_{}", self.next_event_id),
        };
        self.event_queue.push(event);
        self.event_signal_count += 1;
        Ok(())
    }

    fn handle_prop_update(
        &mut self,
        vm: &ScriptVm,
        component_id: u32,
        payload_address: usize,
    ) -> Result<(), VmError> {
        let prop_string = if payload_address != 0 {
            vm.read_string(payload_address, Some(covopt_param!("M_232_49", 256)))
                .unwrap_or_else(|_| format!("state_sync={}", payload_address))
        } else {
            format!("state_sync={}", payload_address)
        };

        if let Some(comp) = self.components.get_mut(&component_id) {
            comp.sync_count += 1;
            if prop_string.contains('=') {
                for part in prop_string.split(',') {
                    if let Some((key, value)) = part.split_once('=') {
                        comp.props.insert(key.trim().to_string(), value.trim().to_string());
                    }
                }
            } else {
                comp.props.insert("payload".to_string(), prop_string);
            }
        }
        self.prop_update_count += 1;
        Ok(())
    }

    fn handle_render_stream(
        &mut self,
        vm: &ScriptVm,
        component_id: u32,
        payload_address: usize,
    ) -> Result<(), VmError> {
        let payload_length = if vm.registers[0] > 0 {
            vm.registers[0] as usize
        } else {
            0
        };

        if payload_address != 0 && payload_length > 0 {
            if let Ok(ptr) = vm.get_ptr(payload_address, payload_length) {
                let slice = unsafe { core::slice::from_raw_parts(ptr, payload_length) };
                self.stream_buffer.append_chunk(slice);
            }
        } else if payload_address != 0
            && let Ok(chunk_bytes) = vm.read_bytes(payload_address, covopt_param!("M_272_68", 1024), true)
            && !chunk_bytes.is_empty()
        {
            self.stream_buffer.append_chunk(&chunk_bytes);
        }

        self.render_count += 1;

        let mut rendered = format!(
            "<ui-root component_id=\"{}\" render_count=\"{}\">\n",
            component_id, self.render_count
        );
        if let Some(comp) = self.components.get(&component_id) {
            rendered.push_str(&format!("  <{}\n", comp.component_type));
            for (key, value) in &comp.props {
                rendered.push_str(&format!("    {}=\"{}\"\n", key, value));
            }
            rendered.push_str("  />\n");
        }
        if self.stream_buffer.total_bytes > 0 {
            rendered.push_str(&format!(
                "  <stream-buffer total_bytes=\"{}\" chunk_count=\"{}\"/>\n",
                self.stream_buffer.total_bytes,
                self.stream_buffer.chunk_count()
            ));
        }
        rendered.push_str("</ui-root>");
        self.last_rendered_output = Some(rendered);

        Ok(())
    }
}
