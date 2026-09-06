//! Criterion micro-benchmarks for Sphinx packet operations.

use criterion::{criterion_group, criterion_main, Criterion, black_box};

use blindhop_lib::sphinx::keys::{NodeInfo, RelayKeyPair, derive_hop_keys, compute_mac};
use blindhop_lib::sphinx::packet::{SphinxPacket, prefix_payload};
use blindhop_lib::config::PAYLOAD_SIZE;

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

fn bench_packet_create(c: &mut Criterion) {
    let payload = prefix_payload(b"benchmark payload message for sphinx packet creation test");

    let mut group = c.benchmark_group("sphinx_create");

    for hops in [1, 2, 3, 5] {
        let (_, route, _, dest) = make_route(hops);
        group.bench_function(format!("{}_hop", hops), |b| {
            b.iter(|| {
                let _ = SphinxPacket::create(black_box(&payload), &route, &dest).unwrap();
            });
        });
    }
    group.finish();
}

fn bench_packet_process(c: &mut Criterion) {
    let payload = prefix_payload(b"benchmark payload");
    let (relay_keys, route, _, dest) = make_route(3);

    // Create a packet
    let (packet, _) = SphinxPacket::create(&payload, &route, &dest).unwrap();

    c.bench_function("sphinx_process_relay", |b| {
        b.iter(|| {
            let pkt = packet.clone();
            let _ = pkt.process_at_relay(black_box(&relay_keys[0]), 0);
        });
    });
}

fn bench_packet_serialize(c: &mut Criterion) {
    let payload = prefix_payload(b"serialize test");
    let (_, route, _, dest) = make_route(3);
    let (packet, _) = SphinxPacket::create(&payload, &route, &dest).unwrap();

    c.bench_function("sphinx_serialize", |b| {
        b.iter(|| {
            let _ = black_box(packet.clone().to_bytes());
        });
    });

    let bytes = packet.to_bytes();
    c.bench_function("sphinx_deserialize", |b| {
        b.iter(|| {
            let _ = SphinxPacket::from_bytes(black_box(&bytes)).unwrap();
        });
    });
}

fn bench_key_derivation(c: &mut Criterion) {
    let shared = [42u8; 32];
    c.bench_function("blake3_hkdf_derive", |b| {
        b.iter(|| {
            let _ = derive_hop_keys(black_box(&shared));
        });
    });
}

fn bench_mac(c: &mut Criterion) {
    let key = [1u8; 32];
    let data = [0u8; 512];
    c.bench_function("blake3_mac_512b", |b| {
        b.iter(|| {
            let _ = compute_mac(black_box(&key), black_box(&data));
        });
    });
}

fn bench_payload_encrypt(c: &mut Criterion) {
    use blindhop_lib::sphinx::payload::encrypt_payload;
    let data = vec![42u8; 1024];
    let keys = vec![[1u8; 32], [2u8; 32], [3u8; 32]];

    c.bench_function("aes_ctr_encrypt_3layers_1kb", |b| {
        b.iter(|| {
            let _ = encrypt_payload(black_box(&data), black_box(&keys));
        });
    });
}

criterion_group!(
    benches,
    bench_packet_create,
    bench_packet_process,
    bench_packet_serialize,
    bench_key_derivation,
    bench_mac,
    bench_payload_encrypt,
);
criterion_main!(benches);
