//! Integration test: Sphinx packet encode → N-hop relay processing → payload recovery.
//!
//! Proves the core cryptographic primitive works correctly.

use blindhop_lib::config::PACKET_SIZE;
use blindhop_lib::sphinx::keys::{NodeInfo, RelayKeyPair};
use blindhop_lib::sphinx::packet::{prefix_payload, RelayAction, SphinxPacket};

fn make_route(n: usize) -> (Vec<RelayKeyPair>, Vec<NodeInfo>, RelayKeyPair, NodeInfo) {
    let mut relay_keys = Vec::new();
    let mut route = Vec::new();
    for i in 0..n {
        let kp = RelayKeyPair::generate();
        route.push(NodeInfo {
            public_key: kp.public,
            address: format!("127.0.0.1:{}", 9401 + i).parse().unwrap(),
        });
        relay_keys.push(kp);
    }
    let dest_kp = RelayKeyPair::generate();
    let dest = NodeInfo {
        public_key: dest_kp.public,
        address: "127.0.0.1:8080".parse().unwrap(),
    };
    (relay_keys, route, dest_kp, dest)
}

#[test]
fn test_1_hop_roundtrip() {
    let message = b"single hop test message";
    let (relay_keys, route, _, dest) = make_route(1);
    let payload = prefix_payload(message);

    let (packet, _) = SphinxPacket::create(&payload, &route, &dest).unwrap();
    assert_eq!(packet.to_bytes().len(), PACKET_SIZE);

    // Process at relay 0 (exit)
    let action = packet.process_at_relay(&relay_keys[0], 0).unwrap();
    match action {
        RelayAction::Deliver { payload: recovered } => {
            assert_eq!(&recovered, message, "Recovered payload must match original");
        }
        RelayAction::Forward { .. } => panic!("Expected Deliver, got Forward"),
    }
}

#[test]
fn test_3_hop_roundtrip() {
    let message = b"three hop test message through the sphinx mixnet";
    let (relay_keys, route, _, dest) = make_route(3);
    let payload = prefix_payload(message);

    let (packet, _) = SphinxPacket::create(&payload, &route, &dest).unwrap();

    // Process through each relay
    let mut current_packet = packet;
    for i in 0..3 {
        let action = current_packet.process_at_relay(&relay_keys[i], i).unwrap();
        match action {
            RelayAction::Forward { next_addr, packet } => {
                assert_eq!(
                    next_addr,
                    route.get(i + 1).map(|n| n.address).unwrap_or(dest.address),
                    "Next address must match route"
                );
                assert_eq!(packet.to_bytes().len(), PACKET_SIZE, "Packet size must remain 2048");
                current_packet = packet;
            }
            RelayAction::Deliver { payload: recovered } => {
                assert_eq!(i, 2, "Deliver should only happen at last hop");
                assert_eq!(&recovered, message, "Recovered payload must match");
                return;
            }
        }
    }
    panic!("Should have delivered after 3 hops");
}

#[test]
fn test_5_hop_roundtrip() {
    let message = b"maximum hops test";
    let (relay_keys, route, _, dest) = make_route(5);
    let payload = prefix_payload(message);

    let (packet, _) = SphinxPacket::create(&payload, &route, &dest).unwrap();

    let mut current_packet = packet;
    for i in 0..5 {
        let action = current_packet.process_at_relay(&relay_keys[i], i).unwrap();
        match action {
            RelayAction::Forward { packet, .. } => {
                current_packet = packet;
            }
            RelayAction::Deliver { payload: recovered } => {
                assert_eq!(i, 4, "Deliver at last hop");
                assert_eq!(&recovered, message);
                return;
            }
        }
    }
    panic!("Should have delivered after 5 hops");
}

#[test]
fn test_packet_always_2048_bytes() {
    // Test with various payload sizes
    for size in [0, 1, 10, 100, 500, 1000, 1500] {
        let message = vec![42u8; size];
        let (_, route, _, dest) = make_route(3);
        let payload = prefix_payload(&message);

        if payload.len() > blindhop_lib::config::PAYLOAD_SIZE {
            continue; // Skip if too large
        }

        let (packet, _) = SphinxPacket::create(&payload, &route, &dest).unwrap();
        let bytes = packet.to_bytes();
        assert_eq!(
            bytes.len(),
            PACKET_SIZE,
            "Packet with {}-byte payload must be {} bytes",
            size,
            PACKET_SIZE
        );
    }
}

#[test]
fn test_wrong_key_fails() {
    let message = b"test";
    let (_, route, _, dest) = make_route(1);
    let payload = prefix_payload(message);
    let (packet, _) = SphinxPacket::create(&payload, &route, &dest).unwrap();

    // Try to process with a wrong key
    let wrong_key = RelayKeyPair::generate();
    let result = packet.process_at_relay(&wrong_key, 0);
    assert!(result.is_err(), "Processing with wrong key must fail");
}

#[test]
fn test_serialization_roundtrip() {
    let message = b"serialize me";
    let (_, route, _, dest) = make_route(3);
    let payload = prefix_payload(message);
    let (packet, _) = SphinxPacket::create(&payload, &route, &dest).unwrap();

    let bytes = packet.to_bytes();
    let packet2 = SphinxPacket::from_bytes(&bytes).unwrap();
    let bytes2 = packet2.to_bytes();
    assert_eq!(bytes, bytes2, "Serialization must be deterministic");
}
