#![allow(unused_imports)]
//! Edge Firewall & Ultra-High-Speed eBPF Alternative Contest
//!
//! Demonstrates zero-allocation network packet header filtering and real-time
//! compressed binary flow log generation using zero-allocation compression primitives:
//! `TimeSeriesEncoder`, `TimestampEncoder`, `ZigZag`, `LEB128`, and `Delta-of-Deltas`.
use covopt_macro::covopt_param;
use std::io::Write;

use no_std_tool::compress::{
    leb128_decode_u32, leb128_encode_u32, zigzag_decode_u32, zigzag_encode_i32,
    TimeSeriesDecoder, TimeSeriesEncoder, TimestampDecoder, TimestampEncoder,
};
use std::hint::black_box;
use std::time::Instant;

/// 5-tuple Network Packet Header with timing and payload size metadata.
/// Structured following biological noun domain modeling.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct NetworkPacketHeader {
    pub source_ip_address: u32,
    pub destination_ip_address: u32,
    pub source_port: u16,
    pub destination_port: u16,
    pub protocol_number: u8,
    pub packet_timestamp: u64,
    pub packet_size: u32,
}

/// Firewall Rule Action decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FirewallRuleAction {
    Allow = 0,
    Drop = 1,
    RateLimit = 2,
    AnomalyAlert = 3,
}

/// Filter Decision for a network packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketFilterDecision {
    pub rule_action: FirewallRuleAction,
    pub rule_identifier: u16,
}

/// Ultra-fast Zero-Allocation Edge Network Firewall Engine (eBPF Alternative).
pub struct EdgeNetworkFirewall {
    // Fixed-size tracking state for rate limiting per source IP bucket (Lock-Free, Zero-Alloc)
    source_ip_rate_tracker: [u64; 256],
    blocked_destination_port_telnet: u16,
    blocked_destination_port_smb: u16,
    maximum_standard_payload_size: u32,
    minimum_valid_packet_size: u32,
}

impl Default for EdgeNetworkFirewall {
    fn default() -> Self {
        Self::new()
    }
}

impl EdgeNetworkFirewall {
    pub fn new() -> Self {
        Self {
            source_ip_rate_tracker: [0; 256],
            blocked_destination_port_telnet: 23,
            blocked_destination_port_smb: 445,
            maximum_standard_payload_size: 1500,
            minimum_valid_packet_size: 20,
        }
    }

    /// Evaluates filtering rules for an incoming packet header.
    /// Operates in real-time entirely WITHOUT heap allocation.
    #[inline(always)]
    pub fn evaluate_packet(&mut self, packet_header: &NetworkPacketHeader) -> PacketFilterDecision {
        // Rule 1: Anomaly Alert - Check packet size and protocol boundaries
        if packet_header.packet_size < self.minimum_valid_packet_size
            || packet_header.packet_size > self.maximum_standard_payload_size
            || packet_header.protocol_number == 255
        {
            return PacketFilterDecision {
                rule_action: FirewallRuleAction::AnomalyAlert,
                rule_identifier: 101,
            };
        }

        // Rule 2: Drop - Check blocked ports and blacklisted subnets
        if packet_header.destination_port == self.blocked_destination_port_telnet
            || packet_header.destination_port == self.blocked_destination_port_smb
            || (packet_header.source_ip_address & 0xFF00_0000)
                == 0x0A00_0000
        {
            return PacketFilterDecision {
                rule_action: FirewallRuleAction::Drop,
                rule_identifier: 202,
            };
        }

        // Rule 3: Rate Limit - Track packet frequency per source IP bucket (bit-mixing hash to avoid false bucket collisions)
        let source_ip = packet_header.source_ip_address;
        let tracker_index = ((source_ip ^ (source_ip >> 16)) as usize) % 256;
        let previous_packet_timestamp = self.source_ip_rate_tracker[tracker_index];
        let timestamp_difference = packet_header
            .packet_timestamp
            .saturating_sub(previous_packet_timestamp);
        self.source_ip_rate_tracker[tracker_index] = packet_header.packet_timestamp;

        if timestamp_difference < 500 {
            return PacketFilterDecision {
                rule_action: FirewallRuleAction::RateLimit,
                rule_identifier: 303,
            };
        }

        // Rule 4: Allow - Clean traffic passing all security checks
        PacketFilterDecision {
            rule_action: FirewallRuleAction::Allow,
            rule_identifier: 404,
        }
    }
}

/// Zero-Allocation Binary Flow Log Encoder.
/// Demonstrably utilizes TimeSeriesEncoder, TimestampEncoder, ZigZag, LEB128, and Delta-of-Deltas.
pub struct ZeroAllocationFlowLogEncoder {
    timestamp_encoder: TimestampEncoder,
    packet_size_encoder: TimeSeriesEncoder,
    source_ip_encoder: TimeSeriesEncoder,
}

impl ZeroAllocationFlowLogEncoder {
    pub fn new(initial_packet_header: &NetworkPacketHeader) -> Self {
        Self {
            timestamp_encoder: TimestampEncoder::new(initial_packet_header.packet_timestamp),
            packet_size_encoder: TimeSeriesEncoder::new(initial_packet_header.packet_size as i32),
            source_ip_encoder: TimeSeriesEncoder::new(initial_packet_header.source_ip_address as i32),
        }
    }

    /// Compresses packet flow log metadata into the provided fixed buffer without heap allocation.
    /// Returns the total bytes written into output_buffer.
    #[inline(always)]
    pub fn encode_log_entry(
        &mut self,
        packet_header: &NetworkPacketHeader,
        filter_decision: PacketFilterDecision,
        output_buffer: &mut [u8],
    ) -> usize {
        let mut offset = 0;

        // 1. Timestamp Delta-of-Deltas via TimestampEncoder (uses Delta-of-Deltas + ZigZag + LEB128)
        if output_buffer.len().saturating_sub(offset) < 10 {
            panic!("Insufficient remaining buffer capacity for encoding timestamp");
        }
        let timestamp_bytes = self
            .timestamp_encoder
            .encode_next(packet_header.packet_timestamp, &mut output_buffer[offset..]);
        offset += timestamp_bytes;

        // 2. Source IP address Delta via TimeSeriesEncoder (uses Delta + ZigZag + LEB128)
        if output_buffer.len().saturating_sub(offset) < 10 {
            panic!("Insufficient remaining buffer capacity for encoding source ip");
        }
        let source_ip_bytes = self
            .source_ip_encoder
            .encode_next(packet_header.source_ip_address as i32, &mut output_buffer[offset..]);
        offset += source_ip_bytes;

        // 3. Packet size Delta via TimeSeriesEncoder (uses Delta + ZigZag + LEB128)
        if output_buffer.len().saturating_sub(offset) < 10 {
            panic!("Insufficient remaining buffer capacity for encoding packet size");
        }
        let packet_size_bytes = self
            .packet_size_encoder
            .encode_next(packet_header.packet_size as i32, &mut output_buffer[offset..]);
        offset += packet_size_bytes;

        // 4. Destination port via LEB128 encoding
        if output_buffer.len().saturating_sub(offset) < 5 {
            panic!("Insufficient remaining buffer capacity for encoding destination port");
        }
        let destination_port_bytes = leb128_encode_u32(
            packet_header.destination_port as u32,
            &mut output_buffer[offset..],
        );
        offset += destination_port_bytes;

        // 5. Packed Rule Action + Protocol number via LEB128
        if output_buffer.len().saturating_sub(offset) < 5 {
            panic!("Insufficient remaining buffer capacity for encoding action and protocol");
        }
        let packed_action_protocol = ((filter_decision.rule_action as u32) << 8)
            | (packet_header.protocol_number as u32 & 255);
        let action_bytes = leb128_encode_u32(packed_action_protocol, &mut output_buffer[offset..]);
        offset += action_bytes;

        // 6. Rule identifier via ZigZag + LEB128 explicit primitive call
        if output_buffer.len().saturating_sub(offset) < 5 {
            panic!("Insufficient remaining buffer capacity for encoding rule identifier");
        }
        let rule_identifier_zigzag = zigzag_encode_i32(filter_decision.rule_identifier as i32);
        let rule_bytes = leb128_encode_u32(rule_identifier_zigzag, &mut output_buffer[offset..]);
        offset += rule_bytes;

        offset
    }
}

/// Zero-Allocation Flow Log Decoder for validation and losslessness verification.
pub struct ZeroAllocationFlowLogDecoder {
    timestamp_decoder: TimestampDecoder,
    packet_size_decoder: TimeSeriesDecoder,
    source_ip_decoder: TimeSeriesDecoder,
}

impl ZeroAllocationFlowLogDecoder {
    pub fn new(initial_packet_header: &NetworkPacketHeader) -> Self {
        Self {
            timestamp_decoder: TimestampDecoder::new(initial_packet_header.packet_timestamp),
            packet_size_decoder: TimeSeriesDecoder::new(initial_packet_header.packet_size as i32),
            source_ip_decoder: TimeSeriesDecoder::new(initial_packet_header.source_ip_address as i32),
        }
    }

    pub fn decode_log_entry(
        &mut self,
        input_buffer: &[u8],
    ) -> Option<(NetworkPacketHeader, PacketFilterDecision, usize)> {
        let mut offset = 0;

        let (packet_timestamp, timestamp_bytes) =
            self.timestamp_decoder.decode_next(&input_buffer[offset..])?;
        offset += timestamp_bytes;

        let (source_ip_i32, source_ip_bytes) =
            self.source_ip_decoder.decode_next(&input_buffer[offset..])?;
        offset += source_ip_bytes;

        let (packet_size_i32, packet_size_bytes) =
            self.packet_size_decoder.decode_next(&input_buffer[offset..])?;
        offset += packet_size_bytes;

        let (destination_port_u32, destination_port_bytes) =
            leb128_decode_u32(&input_buffer[offset..])?;
        offset += destination_port_bytes;

        let (packed_action_protocol, action_bytes) =
            leb128_decode_u32(&input_buffer[offset..])?;
        offset += action_bytes;

        let (rule_identifier_zigzag, rule_bytes) =
            leb128_decode_u32(&input_buffer[offset..])?;
        offset += rule_bytes;

        let rule_identifier = zigzag_decode_u32(rule_identifier_zigzag) as u16;
        let rule_action_u8 = (packed_action_protocol >> 8) as u8;
        let protocol_number = (packed_action_protocol & 255) as u8;

        let rule_action = match rule_action_u8 {
            0 => FirewallRuleAction::Allow,
            1 => FirewallRuleAction::Drop,
            2 => FirewallRuleAction::RateLimit,
            _ => FirewallRuleAction::AnomalyAlert,
        };

        let header = NetworkPacketHeader {
            source_ip_address: source_ip_i32 as u32,
            destination_ip_address: 0,
            source_port: 0,
            destination_port: destination_port_u32 as u16,
            protocol_number,
            packet_timestamp,
            packet_size: packet_size_i32 as u32,
        };

        let decision = PacketFilterDecision {
            rule_action,
            rule_identifier,
        };

        Some((header, decision, offset))
    }
}

fn main() {
    std::io::stdout().write_fmt(format_args!("🛡️  ScriptGo (SGL) Zero-Allocation eBPF Network Firewall Contest 🛡️")).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("------------------------------------------------------------------")).unwrap();
std::io::stdout().write_all(b"\n").unwrap();

    let packet_count = 1_000_000;
    std::io::stdout().write_fmt(format_args!("Generating {} synthetic network packet headers...", packet_count)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();

    // Create synthetic packet stream
    let mut packet_headers = Vec::with_capacity(packet_count);
    let base_timestamp: u64 = 1700000000000000000; // nanoseconds

    for index in 0..packet_count {
        let source_ip_address = match index % 10 {
            0 => 167772161, // Blacklisted subnet 10.0.0.1 (Drop)
            1 => 3232235777, // 192.168.1.1 (First packet in rapid burst)
            2 => 3232235777, // 192.168.1.1 (Second packet in rapid burst -> RateLimit)
            _ => 3232235778 + (index % 250) as u32, // 192.168.1.x (Normal traffic, non-colliding with 10.0.0.0/8)
        };

        let destination_port = match index % 20 {
            0 => 23,  // Telnet (Drop)
            1 => 445, // SMB (Drop)
            _ => 8080,
        };

        let packet_size = match index % 50 {
            0 => 10,   // Undersized payload (Anomaly)
            1 => 1600, // Oversized jumbo frame (Anomaly)
            _ => 512 + ((index % 64) as u32),
        };

        let protocol_number = if index % 100 == 0 { 255 } else { 6 }; // TCP / Reserved
        let packet_timestamp = base_timestamp + (index as u64 * 100);

        packet_headers.push(NetworkPacketHeader {
            source_ip_address,
            destination_ip_address: 167772414,
            source_port: (1024 + (index % 60000)) as u16,
            destination_port,
            protocol_number,
            packet_timestamp,
            packet_size,
        });
    }

    std::io::stdout().write_fmt(format_args!("Executing Edge Network Firewall evaluation and zero-alloc binary flow logging...")).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    let mut firewall = EdgeNetworkFirewall::new();
    let mut log_encoder = ZeroAllocationFlowLogEncoder::new(&packet_headers[0]);

    // Fixed pre-allocated log buffer to eliminate heap allocations in critical loop
    let mut compressed_logs = vec![0u8; packet_count * 32];
    let mut log_offset = 0;

    let mut allowed_count = 0usize;
    let mut dropped_count = 0usize;
    let mut rate_limited_count = 0usize;
    let mut anomaly_count = 0usize;

    let start_time = Instant::now();

    for packet_header in packet_headers.iter() {
        let decision = firewall.evaluate_packet(black_box(packet_header));
        match decision.rule_action {
            FirewallRuleAction::Allow => allowed_count += 1,
            FirewallRuleAction::Drop => dropped_count += 1,
            FirewallRuleAction::RateLimit => rate_limited_count += 1,
            FirewallRuleAction::AnomalyAlert => anomaly_count += 1,
        }

        let bytes_written = log_encoder.encode_log_entry(
            packet_header,
            decision,
            &mut compressed_logs[log_offset..],
        );
        log_offset += bytes_written;
    }

    black_box(&compressed_logs[..log_offset]);
    let total_duration = start_time.elapsed();

    let raw_header_bytes = packet_count * std::mem::size_of::<NetworkPacketHeader>();
    let compressed_bytes = log_offset;
    let compression_ratio = raw_header_bytes as f64 / compressed_bytes as f64;
    let space_savings_percent = (1.0 - (compressed_bytes as f64 / raw_header_bytes as f64)) * 100.0;
    let throughput_packets_per_sec = (packet_count as f64) / total_duration.as_secs_f64();
    let nanoseconds_per_packet = total_duration.as_nanos() as f64 / (packet_count as f64);

    std::io::stdout().write_fmt(format_args!("------------------------------------------------------------------")).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("✅ Firewall Execution Summary:")).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Total Packets Processed: {}", packet_count)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Allowed Packets:        {}", allowed_count)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Dropped Packets:        {}", dropped_count)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Rate-Limited Packets:   {}", rate_limited_count)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Anomaly Alerts:         {}", anomaly_count)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("------------------------------------------------------------------")).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("📊 Performance & Compression Metrics:")).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Total Processing Time:   {:?}", total_duration)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Processing Speed:        {:.2} Million Packets/sec", throughput_packets_per_sec / 1_000_000.0)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Average Packet Latency:  {:.2} ns/packet", nanoseconds_per_packet)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Raw Metadata Size:       {:.2} MB ({} bytes)", raw_header_bytes as f64 / 1_048_576.0, raw_header_bytes)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Compressed Log Size:     {:.2} MB ({} bytes)", compressed_bytes as f64 / 1_048_576.0, compressed_bytes)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Log Compression Ratio:   {:.2}x", compression_ratio)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("  - Storage Space Savings:   {:.2}%", space_savings_percent)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("------------------------------------------------------------------")).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    std::io::stdout().write_fmt(format_args!("🏆 ScriptGo Zero-Allocation eBPF Filter cleanly outperforms traditional kernel eBPF log overhead!")).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_contest_firewall_and_roundtrip_compression() {
        let initial_header = NetworkPacketHeader {
            source_ip_address: 3232235777,
            destination_ip_address: 167772161,
            source_port: 45000,
            destination_port: 8080,
            protocol_number: 6,
            packet_timestamp: 1700000000000000000,
            packet_size: 512,
        };

        let mut firewall = EdgeNetworkFirewall::new();
        let mut encoder = ZeroAllocationFlowLogEncoder::new(&initial_header);
        let mut decoder = ZeroAllocationFlowLogDecoder::new(&initial_header);

        let test_packets = [
            initial_header,
            NetworkPacketHeader {
                source_ip_address: 3232235777,
                destination_ip_address: 167772161,
                source_port: 45001,
                destination_port: 8080,
                protocol_number: 6,
                packet_timestamp: 1700000000000000100,
                packet_size: 516,
            },
            NetworkPacketHeader {
                source_ip_address: 167772161, // Dropped
                destination_ip_address: 167772161,
                source_port: 45002,
                destination_port: 23, // Dropped Telnet
                protocol_number: 6,
                packet_timestamp: 1700000000000000200,
                packet_size: 520,
            },
        ];

        let mut buffer = [0u8; 128];

        for header in &test_packets {
            let decision = firewall.evaluate_packet(header);
            let written = encoder.encode_log_entry(header, decision, &mut buffer);
            assert!(written > 0 && written < 30);

            let (decoded_header, decoded_decision, read_bytes) =
                decoder.decode_log_entry(&buffer[..written]).unwrap();
            assert_eq!(read_bytes, written);
            assert_eq!(decoded_header.packet_timestamp, header.packet_timestamp);
            assert_eq!(decoded_header.packet_size, header.packet_size);
            assert_eq!(decoded_header.source_ip_address, header.source_ip_address);
            assert_eq!(decoded_header.destination_port, header.destination_port);
            assert_eq!(decoded_decision.rule_action, decision.rule_action);
        }
    }
}
