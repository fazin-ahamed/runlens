use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use runlens_core::identifier::Identifier;
use runlens_core::model::{Event, EventSource, PrivacyClassification, Severity};

fn build_event() -> Event {
    let session = Identifier::from_string("01h000000000000000000000ab").unwrap();
    let project = Identifier::from_string("01h000000000000000000000ac").unwrap();
    Event::build(
        Identifier::now(),
        session,
        project,
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

fn bench_event_creation(c: &mut Criterion) {
    let mut g = c.benchmark_group("event_creation");
    g.throughput(criterion::Throughput::Elements(1));
    g.bench_function("build", |b| b.iter(|| black_box(build_event())));
    g.finish();
}

criterion_group!(benches, bench_event_creation);
criterion_main!(benches);
