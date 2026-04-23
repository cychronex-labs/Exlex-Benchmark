use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use exlex::Exlex;
use figment::providers::Format;

#[path = "common/mod.rs"]
mod common;

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("Parse_Init");

    // We test across different scale and structural dimensions
    let topologies = [
        "flat_sparse_first",
        "flat_dense_last",
        "deep_sparse_path6",
        "bushy_normal_middle",
        "lopsided_dense_last",
    ];

    for topo in topologies {
        // 1. Load completely outside the benchmark loop to prevent I/O pollution
        let exlex_str = common::loader::load_fixture(topo, "exlex");
        let json_str = common::loader::load_fixture(topo, "json");
        let toml_str = common::loader::load_fixture(topo, "toml");
        let ini_str = common::loader::load_fixture(topo, "ini");
        let xml_str = common::loader::load_fixture(topo, "xml");
        // --- Exlex ---
        group.bench_with_input(BenchmarkId::new("exlex", topo), &exlex_str, |b, data| {
            b.iter(|| {
                // Testing with preallocator enabled (default None behavior)
                let parsed = Exlex::init_reader(black_box(data), None, None, None, None).unwrap();
                black_box(parsed);
            })
        });

        // --- Serde JSON (The industry standard baseline) ---
        group.bench_with_input(
            BenchmarkId::new("serde_json", topo),
            &json_str,
            |b, data| {
                b.iter(|| {
                    let parsed: serde_json::Value = serde_json::from_str(black_box(data)).unwrap();
                    black_box(parsed);
                })
            },
        );

        // --- Sonic-RS (SIMD JSON parser, closest architectural peer) ---
        group.bench_with_input(BenchmarkId::new("sonic_rs", topo), &json_str, |b, data| {
            b.iter(|| {
                let parsed: sonic_rs::Value = sonic_rs::from_str(black_box(data)).unwrap();
                black_box(parsed);
            })
        });
        group.bench_with_input(BenchmarkId::new("simd_json", topo), &json_str, |b, data| {
            b.iter_batched(
                || data.as_bytes().to_vec(), // The Setup: Happens outside the timer
                |mut bytes| {
                    // The Loop: Strictly times the parser
                    let parsed: simd_json::OwnedValue =
                        simd_json::from_slice(black_box(&mut bytes)).unwrap();
                    black_box(parsed);
                },
                criterion::BatchSize::SmallInput,
            )
        });
        // --- TOML Edit (The mutation competitor) ---
        group.bench_with_input(BenchmarkId::new("toml_edit", topo), &toml_str, |b, data| {
            b.iter(|| {
                let parsed = black_box(data).parse::<toml_edit::DocumentMut>().unwrap();
                black_box(parsed);
            })
        });

        // --- Rust-INI (Closest format structure) ---
        group.bench_with_input(BenchmarkId::new("rust_ini", topo), &ini_str, |b, data| {
            b.iter(|| {
                let parsed = ini::Ini::load_from_str(black_box(data)).unwrap();
                black_box(parsed);
            })
        });
        // --- Quick-XML (Closest no_std zero-copy peer) ---
        group.bench_with_input(
            BenchmarkId::new("quick_xml", topo),
            &xml_str, // Uses the correctly loaded file from the top of the loop
            |b, xml_data| {
                b.iter(|| {
                    let mut reader = quick_xml::Reader::from_str(black_box(xml_data));
                    let mut buf = Vec::with_capacity(1024); // Allocate ONCE outside the loop
                    loop {
                        match reader.read_event_into(&mut buf) {
                            Ok(quick_xml::events::Event::Eof) => break,
                            Err(_) => break,
                            _ => buf.clear(), // Clear buffer to reuse memory (zero-alloc simulation)
                        }
                    }
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("figment_json", topo),
            &json_str,
            |b, data| {
                b.iter(|| {
                    // We test how fast Figment can extract a value wrapped around a JSON provider
                    let parsed =
                        figment::Figment::from(figment::providers::Json::string(black_box(data)));
                    // Force an extraction to ensure the tree is actually built/evaluated
                    let _ = parsed
                        .extract_inner::<serde_json::Value>("")
                        .unwrap_or_default();
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
