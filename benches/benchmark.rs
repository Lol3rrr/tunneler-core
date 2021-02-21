use criterion::{criterion_group, criterion_main, Criterion};

use tunneler_core::message::{Message, MessageHeader, MessageType};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut data = [0; 13];
    MessageHeader::new(132, MessageType::Data, 200).serialize(&mut data);
    c.bench_function("Deserialize-Message-Header", |b| {
        b.iter(|| MessageHeader::deserialize(&data))
    });
    let header = MessageHeader::new(132, MessageType::Data, 200);
    let mut serialize_out = [0; 13];
    c.bench_function("Serialize-Message-Header", |b| {
        b.iter(|| header.serialize(&mut serialize_out))
    });

    let msg = Message::new(header, vec![0; 4092]);
    c.bench_function("Serialize-Message", |b| {
        b.iter(|| msg.serialize(&mut serialize_out))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
