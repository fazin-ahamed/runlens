use criterion::{criterion_group, criterion_main, black_box, Criterion};
use chrono::Utc;
use runlens_core::identifier::Identifier;
use runlens_core::model::{Event, EventSource, PrivacyClassification, Severity};
use runlens_storage::repo::Repository;

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

fn bench_append(c: &mut Criterion) {
    let mut g = c.benchmark_group("storage_append");
    g.throughput(criterion::Throughput::Elements(1));

    g.bench_function("single_in_memory", |b| {
        let repo = Repository::in_memory().unwrap();
        let mut ev = build_event();
        b.iter(|| {
            ev.sequence += 1;
            black_box(repo.append_event(&ev).unwrap());
        });
    });

    g.bench_function("batch_1k", |b| {
        b.iter_batched(
            || (Repository::in_memory().unwrap(), Vec::with_capacity(1000)),
            |(repo, mut events)| {
                events.clear();
                for i in 0..1000 {
                    let mut ev = build_event();
                    ev.sequence = i;
                    events.push(ev);
                }
                black_box(repo.batch_append_events(&events).unwrap());
            },
            criterion::BatchSize::SmallInput,
        )
    });
    g.finish();
}

criterion_group!(benches, bench_append);
criterion_main!(benches);
