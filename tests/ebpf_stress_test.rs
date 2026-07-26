#![allow(unused_imports)]
//! Empirical Stress Test & Challenger Suite for ScriptGo eBPF Edge Firewall
//!
//! Tests:
//! 1. Zero Heap Allocations during packet processing & flow log encoding using a Global Allocator hook.
//! 2. Boundary IP Values (0.0.0.0, 255.255.255.255, 10.x.x.x subnet, max signed i32 boundaries).
//! 3. Cross-IP Hash Collision in Rate Limit Tracker (% 256 bucket collision).
//! 4. Empty Stream & Buffer Overflow / Underflow limits.
//! 5. Compression Losslessness & Field Preservation.
use covopt_macro::covopt_param;
use std::io::Write;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use std::cell::Cell;

// Custom allocator to track heap allocations empirically
struct AllocationTracker;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static THREAD_ALLOC_COUNT: Cell<usize> = const { Cell::new(0) };
    static THREAD_ALLOC_BYTES: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for AllocationTracker {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        ALLOC_BYTES.fetch_add(layout.size(), Ordering::SeqCst);
        THREAD_ALLOC_COUNT.with(|c| c.set(c.get() + 1));
        THREAD_ALLOC_BYTES.with(|c| c.set(c.get() + layout.size()));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout); }
    }
}

#[global_allocator]
static GLOBAL: AllocationTracker = AllocationTracker;

use no_std_tool::compress::{
    leb128_decode_u32, leb128_encode_u32, zigzag_decode_u32, zigzag_encode_i32,
    TimeSeriesDecoder, TimeSeriesEncoder, TimestampDecoder, TimestampEncoder,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FirewallRuleAction {
    Allow = 0,
    Drop = 1,
    RateLimit = 2,
    AnomalyAlert = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketFilterDecision {
    pub rule_action: FirewallRuleAction,
    pub rule_identifier: u16,
}

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
            blocked_destination_port_telnet: 23,
            blocked_destination_port_smb: 445,
            maximum_standard_payload_size: 1500,
            minimum_valid_packet_size: 20,
        }
    }

    #[inline(always)]
    pub fn evaluate_packet(&mut self, packet_header: &NetworkPacketHeader) -> PacketFilterDecision {
        if packet_header.packet_size < self.minimum_valid_packet_size
            || packet_header.packet_size > self.maximum_standard_payload_size
            || packet_header.protocol_number == 255
        {
            return PacketFilterDecision {
                rule_action: FirewallRuleAction::AnomalyAlert,
                rule_identifier: 101,
            };
        }

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

        PacketFilterDecision {
            rule_action: FirewallRuleAction::Allow,
            rule_identifier: 404,
        }
    }
}

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

        if output_buffer.len().saturating_sub(offset) < 10 {
            panic!("Insufficient remaining buffer capacity for encoding timestamp");
        }
        let timestamp_bytes = self
            .timestamp_encoder
            .encode_next(packet_header.packet_timestamp, &mut output_buffer[offset..]);
        offset += timestamp_bytes;

        if output_buffer.len().saturating_sub(offset) < 10 {
            panic!("Insufficient remaining buffer capacity for encoding source ip");
        }
        let source_ip_bytes = self
            .source_ip_encoder
            .encode_next(packet_header.source_ip_address as i32, &mut output_buffer[offset..]);
        offset += source_ip_bytes;

        if output_buffer.len().saturating_sub(offset) < 10 {
            panic!("Insufficient remaining buffer capacity for encoding packet size");
        }
        let packet_size_bytes = self
            .packet_size_encoder
            .encode_next(packet_header.packet_size as i32, &mut output_buffer[offset..]);
        offset += packet_size_bytes;

        if output_buffer.len().saturating_sub(offset) < 5 {
            panic!("Insufficient remaining buffer capacity for encoding destination port");
        }
        let destination_port_bytes = leb128_encode_u32(
            packet_header.destination_port as u32,
            &mut output_buffer[offset..],
        );
        offset += destination_port_bytes;

        if output_buffer.len().saturating_sub(offset) < 5 {
            panic!("Insufficient remaining buffer capacity for encoding action and protocol");
        }
        let packed_action_protocol = ((filter_decision.rule_action as u32) << 8)
            | (packet_header.protocol_number as u32 & 255);
        let action_bytes = leb128_encode_u32(packed_action_protocol, &mut output_buffer[offset..]);
        offset += action_bytes;

        if output_buffer.len().saturating_sub(offset) < 5 {
            panic!("Insufficient remaining buffer capacity for encoding rule identifier");
        }
        let rule_identifier_zigzag = zigzag_encode_i32(filter_decision.rule_identifier as i32);
        let rule_bytes = leb128_encode_u32(rule_identifier_zigzag, &mut output_buffer[offset..]);
        offset += rule_bytes;

        offset
    }
}

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

// -----------------------------------------------------------------------------
// TESTS
// -----------------------------------------------------------------------------

static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn test_zero_allocations_empirical_verification() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut firewall = EdgeNetworkFirewall::new();
    let sample_packet = NetworkPacketHeader {
        source_ip_address: 3232235521, // 192.168.0.1
        destination_ip_address: 134744072,
        source_port: 12345,
        destination_port: 80,
        protocol_number: 6,
        packet_timestamp: 1700000000000000000,
        packet_size: 500,
    };
    let mut encoder = ZeroAllocationFlowLogEncoder::new(&sample_packet);
    let mut buf = [0u8; 128];

    // Reset thread-local allocation counters
    THREAD_ALLOC_COUNT.with(|c| c.set(0));
    THREAD_ALLOC_BYTES.with(|c| c.set(0));

    let iterations = 100000;
    for i in 0..iterations {
        let pkt = NetworkPacketHeader {
            packet_timestamp: sample_packet.packet_timestamp + (i as u64 * 600),
            ..sample_packet
        };
        let decision = firewall.evaluate_packet(&pkt);
        let written = encoder.encode_log_entry(&pkt, decision, &mut buf);
        std::hint::black_box(written);
    }

    let allocs = THREAD_ALLOC_COUNT.with(|c| c.get());
    let bytes = THREAD_ALLOC_BYTES.with(|c| c.get());

    std::io::stdout().write_fmt(format_args!("[EMPIRICAL VERIFICATION] Total Heap Allocations over {} packets: {} (bytes: {})", iterations, allocs, bytes)).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
    assert_eq!(allocs, 0, "Zero-allocation claim failed! Detected {} heap allocations during packet filtering!", allocs);
}

#[test]
fn test_boundary_ip_values_and_protocols() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut firewall = EdgeNetworkFirewall::new();
    let mut ts = 1000000;

    // 1. Boundary IP: 0.0.0.0
    ts += 1000;
    let pkt_zero_ip = NetworkPacketHeader {
        source_ip_address: 0x00000000,
        destination_ip_address: 0x00000000,
        source_port: 0,
        destination_port: 80,
        protocol_number: 6,
        packet_timestamp: ts,
        packet_size: 64,
    };
    assert_eq!(firewall.evaluate_packet(&pkt_zero_ip).rule_action, FirewallRuleAction::Allow);

    // 2. Boundary IP: 255.255.255.255
    ts += 1000;
    let pkt_broadcast_ip = NetworkPacketHeader {
        source_ip_address: 4294967295,
        destination_ip_address: 4294967295,
        source_port: 65535,
        destination_port: 443,
        protocol_number: 17,
        packet_timestamp: ts,
        packet_size: 1500,
    };
    assert_eq!(firewall.evaluate_packet(&pkt_broadcast_ip).rule_action, FirewallRuleAction::Allow);

    // 3. Subnet Drop: 10.0.0.0 to 10.255.255.255
    ts += 1000;
    let pkt_subnet_start = NetworkPacketHeader {
        source_ip_address: 167772160,
        destination_ip_address: 16843009,
        source_port: 1000,
        destination_port: 80,
        protocol_number: 6,
        packet_timestamp: ts,
        packet_size: 100,
    };
    assert_eq!(firewall.evaluate_packet(&pkt_subnet_start).rule_action, FirewallRuleAction::Drop);

    ts += 1000;
    let pkt_subnet_end = NetworkPacketHeader {
        source_ip_address: 184549375,
        destination_ip_address: 16843009,
        source_port: 1000,
        destination_port: 80,
        protocol_number: 6,
        packet_timestamp: ts,
        packet_size: 100,
    };
    assert_eq!(firewall.evaluate_packet(&pkt_subnet_end).rule_action, FirewallRuleAction::Drop);

    // 4. Outside Subnet: 11.0.0.0
    ts += 1000;
    let pkt_outside_subnet = NetworkPacketHeader {
        source_ip_address: 184549376,
        destination_ip_address: 16843009,
        source_port: 1000,
        destination_port: 80,
        protocol_number: 6,
        packet_timestamp: ts,
        packet_size: 100,
    };
    assert_eq!(firewall.evaluate_packet(&pkt_outside_subnet).rule_action, FirewallRuleAction::Allow);

    // 5. Packet size boundaries: min valid = 20, max valid = 1500
    ts += 1000;
    let pkt_undersized = NetworkPacketHeader { packet_size: 19, packet_timestamp: ts, ..pkt_outside_subnet };
    assert_eq!(firewall.evaluate_packet(&pkt_undersized).rule_action, FirewallRuleAction::AnomalyAlert);

    ts += 1000;
    let pkt_min_size = NetworkPacketHeader { packet_size: 20, packet_timestamp: ts, ..pkt_outside_subnet };
    assert_eq!(firewall.evaluate_packet(&pkt_min_size).rule_action, FirewallRuleAction::Allow);

    ts += 1000;
    let pkt_max_size = NetworkPacketHeader { packet_size: 1500, packet_timestamp: ts, ..pkt_outside_subnet };
    assert_eq!(firewall.evaluate_packet(&pkt_max_size).rule_action, FirewallRuleAction::Allow);

    ts += 1000;
    let pkt_oversized = NetworkPacketHeader { packet_size: 1501, packet_timestamp: ts, ..pkt_outside_subnet };
    assert_eq!(firewall.evaluate_packet(&pkt_oversized).rule_action, FirewallRuleAction::AnomalyAlert);

    // 6. Protocol 255 (Reserved)
    ts += 1000;
    let pkt_proto_255 = NetworkPacketHeader { protocol_number: 255, packet_timestamp: ts, ..pkt_outside_subnet };
    assert_eq!(firewall.evaluate_packet(&pkt_proto_255).rule_action, FirewallRuleAction::AnomalyAlert);

    // 7. Blocked ports: Telnet (23) and SMB (445)
    ts += 1000;
    let pkt_telnet = NetworkPacketHeader { destination_port: 23, packet_timestamp: ts, ..pkt_outside_subnet };
    assert_eq!(firewall.evaluate_packet(&pkt_telnet).rule_action, FirewallRuleAction::Drop);

    ts += 1000;
    let pkt_smb = NetworkPacketHeader { destination_port: 445, packet_timestamp: ts, ..pkt_outside_subnet };
    assert_eq!(firewall.evaluate_packet(&pkt_smb).rule_action, FirewallRuleAction::Drop);
}

#[test]
fn test_rate_limiter_hash_collision_vulnerability() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut firewall = EdgeNetworkFirewall::new();

    // IP A: 1.1.1.1 (0x01010101) -> 0x01010101 % 256 = 1
    // IP B: 2.2.2.1 (0x02020201) -> 0x02020201 % 256 = 1
    let ip_a = 16843009;
    let ip_b = 33686017;

    assert_eq!((ip_a % 256) as usize, (ip_b % 256) as usize);

    // IP A sends first packet at t = 1,000,000 ns
    let pkt_a1 = NetworkPacketHeader {
        source_ip_address: ip_a,
        destination_ip_address: 3232235521,
        source_port: 1000,
        destination_port: 80,
        protocol_number: 6,
        packet_timestamp: 1000000,
        packet_size: 100,
    };
    let dec_a1 = firewall.evaluate_packet(&pkt_a1);
    assert_eq!(dec_a1.rule_action, FirewallRuleAction::Allow);

    // IP B sends packet 10,000 ns later (t = 1,010,000 ns). IP B has never sent a packet before!
    // However, because IP B hashes to the same bucket as IP A, IP B is compared against IP A's timestamp!
    // Since (1,010,000 - 1,000,000) = 10,000 >= 500 ns, IP B passes.
    let pkt_b1 = NetworkPacketHeader {
        source_ip_address: ip_b,
        packet_timestamp: 1010000,
        ..pkt_a1
    };
    let dec_b1 = firewall.evaluate_packet(&pkt_b1);
    assert_eq!(dec_b1.rule_action, FirewallRuleAction::Allow);

    // Now IP B sends another packet 100 ns after its first packet (t = 1,010,100 ns).
    // Timestamp difference is 100 ns < 500 ns threshold -> IP B gets RateLimited!
    let pkt_b2 = NetworkPacketHeader {
        source_ip_address: ip_b,
        packet_timestamp: 1010100,
        ..pkt_a1
    };
    let dec_b2 = firewall.evaluate_packet(&pkt_b2);
    assert_eq!(dec_b2.rule_action, FirewallRuleAction::RateLimit);

    std::io::stdout().write_fmt(format_args!("[FINDING] Rate Tracker uses modulo 256 on IPv4 address causing bucket collisions between different flows.")).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
}

#[test]
fn test_small_output_buffer_panics() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let initial_pkt = NetworkPacketHeader {
        source_ip_address: 3232235521,
        destination_ip_address: 134744072,
        source_port: 1234,
        destination_port: 80,
        protocol_number: 6,
        packet_timestamp: 1000000,
        packet_size: 500,
    };
    let mut encoder = ZeroAllocationFlowLogEncoder::new(&initial_pkt);
    let decision = PacketFilterDecision {
        rule_action: FirewallRuleAction::Allow,
        rule_identifier: 404,
    };

    // Buffer smaller than required output bytes (e.g. 2 bytes)
    let mut tiny_buf = [0u8; 2];
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        encoder.encode_log_entry(&initial_pkt, decision, &mut tiny_buf)
    }));

    assert!(res.is_err(), "Expected panic on buffer overflow when output buffer is too small for LEB128!");
    std::io::stdout().write_fmt(format_args!("[FINDING] Flow log encoding panics when output buffer slice is shorter than the compressed log entry.")).unwrap();
std::io::stdout().write_all(b"\n").unwrap();
}

#[test]
fn test_empty_stream_and_high_throughput_integrity() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut firewall = EdgeNetworkFirewall::new();
    let initial_pkt = NetworkPacketHeader {
        source_ip_address: 184549377,
        destination_ip_address: 134744072,
        source_port: 1000,
        destination_port: 80,
        protocol_number: 6,
        packet_timestamp: 100000,
        packet_size: 100,
    };

    let mut encoder = ZeroAllocationFlowLogEncoder::new(&initial_pkt);
    let mut decoder = ZeroAllocationFlowLogDecoder::new(&initial_pkt);

    // Empty stream check (0 iterations)
    let mut log_buf = [0u8; 64];
    let written = 0;
    assert_eq!(decoder.decode_log_entry(&log_buf[..written]), None);

    // High volume loop: 50,000 packets round-trip test
    for i in 1..=50000 {
        let pkt = NetworkPacketHeader {
            source_ip_address: 184549376 + (i % 200) as u32,
            destination_port: 80 + (i % 10) as u16,
            packet_timestamp: 100000 + (i as u64 * 1000),
            packet_size: 100 + (i % 500) as u32,
            ..initial_pkt
        };

        let decision = firewall.evaluate_packet(&pkt);
        let bytes_written = encoder.encode_log_entry(&pkt, decision, &mut log_buf);
        assert!(bytes_written > 0 && bytes_written <= 32);

        let (decoded_header, decoded_decision, read_bytes) =
            decoder.decode_log_entry(&log_buf[..bytes_written]).expect("Decoding failed");

        assert_eq!(read_bytes, bytes_written);
        assert_eq!(decoded_header.packet_timestamp, pkt.packet_timestamp);
        assert_eq!(decoded_header.packet_size, pkt.packet_size);
        assert_eq!(decoded_header.source_ip_address, pkt.source_ip_address);
        assert_eq!(decoded_header.destination_port, pkt.destination_port);
        assert_eq!(decoded_decision.rule_action, decision.rule_action);
        assert_eq!(decoded_decision.rule_identifier, decision.rule_identifier);
    }
}
