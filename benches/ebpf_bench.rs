//! Criterion Benchmark for ScriptGo eBPF Edge Firewall & Zero-Allocation Log Compression
#[macro_use] extern crate covopt_macro;
use covopt_macro::covopt_param;

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use no_std_tool::compress::{
    leb128_encode_u32, zigzag_encode_i32, TimeSeriesEncoder, TimestampEncoder,
};

/// 5-tuple Network Packet Header with metadata.
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

/// Filter Decision for a packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketFilterDecision {
    pub rule_action: FirewallRuleAction,
    pub rule_identifier: u16,
}

/// Ultra-fast Zero-Allocation Edge Network Firewall Engine.
pub struct EdgeNetworkFirewall {
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
            blocked_destination_port_telnet: covopt_param!("M_58_45", 23),
            blocked_destination_port_smb: covopt_param!("M_59_42", 445),
            maximum_standard_payload_size: covopt_param!("M_60_43", 1500),
            minimum_valid_packet_size: covopt_param!("M_61_39", 20),
        }
    }

    #[inline(always)]
    pub fn evaluate_packet(&mut self, packet_header: &NetworkPacketHeader) -> PacketFilterDecision {
        if packet_header.packet_size < self.minimum_valid_packet_size
            || packet_header.packet_size > self.maximum_standard_payload_size
            || packet_header.protocol_number == covopt_param!("M_69_48", 255)
        {
            return PacketFilterDecision {
                rule_action: FirewallRuleAction::AnomalyAlert,
                rule_identifier: covopt_param!("M_73_33", 101),
            };
        }

        if packet_header.destination_port == self.blocked_destination_port_telnet
            || packet_header.destination_port == self.blocked_destination_port_smb
            || (packet_header.source_ip_address & covopt_param!("M_79_50", 4278190080))
                == covopt_param!("M_80_19", 167772160)
        {
            return PacketFilterDecision {
                rule_action: FirewallRuleAction::Drop,
                rule_identifier: covopt_param!("M_84_33", 202),
            };
        }

        let source_ip = packet_header.source_ip_address;
        let tracker_index = ((source_ip ^ (source_ip >> covopt_param!("M_89_56", 16))) as usize) % covopt_param!("M_89_73", 256);
        let previous_packet_timestamp = self.source_ip_rate_tracker[tracker_index];
        let timestamp_difference = packet_header
            .packet_timestamp
            .saturating_sub(previous_packet_timestamp);
        self.source_ip_rate_tracker[tracker_index] = packet_header.packet_timestamp;

        if timestamp_difference < covopt_param!("M_96_34", 500) {
            return PacketFilterDecision {
                rule_action: FirewallRuleAction::RateLimit,
                rule_identifier: covopt_param!("M_99_33", 303),
            };
        }

        PacketFilterDecision {
            rule_action: FirewallRuleAction::Allow,
            rule_identifier: covopt_param!("M_105_29", 404),
        }
    }
}

/// Zero-Allocation Binary Flow Log Encoder.
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

    #[inline(always)]
    pub fn encode_log_entry(
        &mut self,
        packet_header: &NetworkPacketHeader,
        filter_decision: PacketFilterDecision,
        output_buffer: &mut [u8],
    ) -> usize {
        let mut offset = 0;

        if output_buffer.len().saturating_sub(offset) < covopt_param!("M_135_56", 10) {
            panic!("Insufficient remaining buffer capacity for encoding timestamp");
        }
        let timestamp_bytes = self
            .timestamp_encoder
            .encode_next(packet_header.packet_timestamp, &mut output_buffer[offset..]);
        offset += timestamp_bytes;

        if output_buffer.len().saturating_sub(offset) < covopt_param!("M_143_56", 10) {
            panic!("Insufficient remaining buffer capacity for encoding source ip");
        }
        let source_ip_bytes = self
            .source_ip_encoder
            .encode_next(packet_header.source_ip_address as i32, &mut output_buffer[offset..]);
        offset += source_ip_bytes;

        if output_buffer.len().saturating_sub(offset) < covopt_param!("M_151_56", 10) {
            panic!("Insufficient remaining buffer capacity for encoding packet size");
        }
        let packet_size_bytes = self
            .packet_size_encoder
            .encode_next(packet_header.packet_size as i32, &mut output_buffer[offset..]);
        offset += packet_size_bytes;

        if output_buffer.len().saturating_sub(offset) < covopt_param!("M_159_56", 5) {
            panic!("Insufficient remaining buffer capacity for encoding destination port");
        }
        let destination_port_bytes = leb128_encode_u32(
            packet_header.destination_port as u32,
            &mut output_buffer[offset..],
        );
        offset += destination_port_bytes;

        if output_buffer.len().saturating_sub(offset) < covopt_param!("M_168_56", 5) {
            panic!("Insufficient remaining buffer capacity for encoding action and protocol");
        }
        let packed_action_protocol = ((filter_decision.rule_action as u32) << covopt_param!("M_171_78", 8))
            | (packet_header.protocol_number as u32 & covopt_param!("M_172_54", 255));
        let action_bytes = leb128_encode_u32(packed_action_protocol, &mut output_buffer[offset..]);
        offset += action_bytes;

        if output_buffer.len().saturating_sub(offset) < covopt_param!("M_176_56", 5) {
            panic!("Insufficient remaining buffer capacity for encoding rule identifier");
        }
        let rule_identifier_zigzag = zigzag_encode_i32(filter_decision.rule_identifier as i32);
        let rule_bytes = leb128_encode_u32(rule_identifier_zigzag, &mut output_buffer[offset..]);
        offset += rule_bytes;

        offset
    }
}

fn generate_benchmark_packet_batch(packet_count: usize) -> Vec<NetworkPacketHeader> {
    let mut packet_headers = Vec::with_capacity(packet_count);
    let base_timestamp: u64 = covopt_param!("M_189_30", 1700000000000000000);

    for index in 0..packet_count {
        let source_ip_address = match index % covopt_param!("M_192_46", 10) {
            0 => covopt_param!("M_193_17", 167772161),
            1 => covopt_param!("M_194_17", 3232235777),
            2 => covopt_param!("M_195_17", 3232235777),
            _ => covopt_param!("M_196_17", 3232235778) + (index % covopt_param!("M_196_39", 250)) as u32,
        };

        let destination_port = match index % covopt_param!("M_199_45", 20) {
            0 => covopt_param!("M_200_17", 23),
            1 => covopt_param!("M_201_17", 445),
            _ => covopt_param!("M_202_17", 8080),
        };

        let packet_size = match index % covopt_param!("M_205_40", 50) {
            0 => covopt_param!("M_206_17", 10),
            1 => covopt_param!("M_207_17", 1600),
            _ => covopt_param!("M_208_17", 512) + ((index % covopt_param!("M_208_33", 64)) as u32),
        };

        let protocol_number = if index % covopt_param!("M_211_41", 100) == 0 { covopt_param!("M_211_52", 255) } else { covopt_param!("M_211_65", 6) };
        let packet_timestamp = base_timestamp + (index as u64 * covopt_param!("M_212_64", 100)) + (index % covopt_param!("M_212_80", 5)) as u64;

        packet_headers.push(NetworkPacketHeader {
            source_ip_address,
            destination_ip_address: covopt_param!("M_216_36", 167772414),
            source_port: (covopt_param!("M_217_26", 1024) + (index % covopt_param!("M_217_42", 60000))) as u16,
            destination_port,
            protocol_number,
            packet_timestamp,
            packet_size,
        });
    }

    packet_headers
}

fn bench_ebpf_packet_filtering(c: &mut Criterion) {
    let mut group = c.benchmark_group("ebpf_packet_filtering");
    let packet_count = covopt_param!("M_230_23", 10000);
    let packet_headers = generate_benchmark_packet_batch(packet_count);

    group.throughput(Throughput::Elements(packet_count as u64));

    group.bench_function("packet_filter_throughput", |b| {
        let mut firewall = EdgeNetworkFirewall::new();
        b.iter(|| {
            let mut decision_accumulator = 0u32;
            for packet_header in packet_headers.iter() {
                let decision = firewall.evaluate_packet(black_box(packet_header));
                decision_accumulator = decision_accumulator.wrapping_add(decision.rule_identifier as u32);
            }
            black_box(decision_accumulator);
        });
    });

    group.finish();
}

fn bench_zero_allocation_log_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("ebpf_log_compression");
    let packet_count = covopt_param!("M_252_23", 10000);
    let packet_headers = generate_benchmark_packet_batch(packet_count);

    let raw_bytes = packet_count * std::mem::size_of::<NetworkPacketHeader>();
    group.throughput(Throughput::Bytes(raw_bytes as u64));

    group.bench_function("zero_alloc_log_compression_throughput", |b| {
        let mut firewall = EdgeNetworkFirewall::new();
        let decisions: Vec<PacketFilterDecision> = packet_headers
            .iter()
            .map(|header| firewall.evaluate_packet(header))
            .collect();

        b.iter(|| {
            let mut encoder = ZeroAllocationFlowLogEncoder::new(&packet_headers[0]);
            let mut output_buffer = [0u8; 64];
            let mut total_bytes_written = 0usize;

            for (header, decision) in packet_headers.iter().zip(decisions.iter()) {
                let written = encoder.encode_log_entry(
                    black_box(header),
                    black_box(*decision),
                    &mut output_buffer,
                );
                total_bytes_written += written;
            }

            black_box(total_bytes_written);
        });
    });

    group.finish();
}

fn bench_full_ebpf_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("ebpf_full_pipeline");
    let packet_count = covopt_param!("M_288_23", 10000);
    let packet_headers = generate_benchmark_packet_batch(packet_count);

    group.throughput(Throughput::Elements(packet_count as u64));

    group.bench_function("full_filter_and_compress_pipeline", |b| {
        b.iter(|| {
            let mut firewall = EdgeNetworkFirewall::new();
            let mut encoder = ZeroAllocationFlowLogEncoder::new(&packet_headers[0]);
            let mut local_log_buffer = [0u8; 64];
            let mut total_written_bytes = 0usize;

            for packet_header in packet_headers.iter() {
                let decision = firewall.evaluate_packet(black_box(packet_header));
                let written = encoder.encode_log_entry(
                    packet_header,
                    decision,
                    &mut local_log_buffer,
                );
                total_written_bytes += written;
            }

            black_box(total_written_bytes);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_ebpf_packet_filtering,
    bench_zero_allocation_log_compression,
    bench_full_ebpf_pipeline
);
criterion_main!(benches);
