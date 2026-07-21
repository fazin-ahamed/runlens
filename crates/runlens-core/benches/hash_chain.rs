use criterion::{criterion_group, criterion_main, black_box, Criterion};
use chrono::Utc;
use runlens_core::chain;
use runlens_core::identifier::Identifier;
use runlens_core::model::{Event, EventSource, PrivacyClassification, Severity};

fn build_event() -> Event {
    Event::build(
        Identifier::now(),
        Identifier::from_string("01h000000000000000000000ab").unwrap(),
        Identifier::from_string("01h000000000000000000000ac").unwrap(),
        0,
        EventSource::Cli,
        "command.start",
        Severity::Info,
        Utc::now(),
        0,
        1,
        serde_json::json!({"command": "cargo test", "meta": true, "tags": ["a", "b", "c"]}),
        PrivacyClassification::Public,
    )
}

fn bench_hash_chain(c: &mut Criterion) {
    let mut prior = String::from(chain::GENESIS_HASH);

    let mut g = c.benchmark_group("hash_chain");
    g.throughput(criterion::Throughput::Elements(1));
    g.bench_function("seal", |b| {
        b.iter(|| {
            let mut ev = build_event();
            let h = chain::seal(&mut ev, &prior);
            prior = black_box(h);
        })
    });
    g.bench_function("verify_chain_1k", |b| {
        let mut events: Vec<Event> = Vec::with_capacity(1000);
        for _ in 0..1000 {
            let mut ev = build_event();
            let h = chain::seal(&mut ev, &prior);
            prior = h;
            events.push(ev);
        }
        b.iter(|| {
            let mut copy = events.clone();
            black_box(chain::seal_chain(&mut copy));
        })
    });
    g.finish();
}

criterion_group!(benches, bench_hash_chain);
criterion_main!(benches);
