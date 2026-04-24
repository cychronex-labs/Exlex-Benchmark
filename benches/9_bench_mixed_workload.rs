// Copyright 2026 Abdul Wahab Melethil Shibu (cychronex-labs)
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-APACHE> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Benchmark: Mixed Workload (70% reads, 30% writes)
//
// Simulates a realistic application runtime loop where config values
// are read frequently and occasionally mutated (feature flag flips,
// live config updates, hot reload patches).
//
// The 70/30 split is a deliberate approximation of production read/write
// ratios observed in server config and game config workloads.
//
// Serde JSON is included as a read-only baseline — it has no mutation
// support without cloning the entire value tree, so it represents the
// cost of the 7-read portion only. This is an honest comparison:
// Serde simply cannot do the write half of this workload.
//
// Hardware: Intel Core i3-6006U, 16GB DDR4-2133, IPC ~1.7, TLB miss 0.07%

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use exlex::{Exlex, ExlexArena};

#[path = "common/mod.rs"]
mod common;

fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("Mixed_Workload");

    let exlex_str = common::loader::load_fixture("bushy_normal_sequential", "exlex");
    let json_str  = common::loader::load_fixture("bushy_normal_sequential", "json");
    let toml_str  = common::loader::load_fixture("bushy_normal_sequential", "toml");

    let exlex_parsed = Exlex::init_reader(&exlex_str, None, None, None, None).unwrap();
    let json_parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let toml_doc = toml_str.parse::<toml_edit::DocumentMut>().unwrap();

    let target_section = "bn_L0_N0";
    let target_sect_id = exlex_parsed
        .get_child(target_section, exlex_parsed.get_root())
        .unwrap();

    // 7 reads + 3 writes = 10 ops total, 70/30 split
    let read_keys = [
        "key_bn_L0_N0_0",
        "key_bn_L0_N0_1",
        "key_bn_L0_N0_2",
        "key_bn_L0_N0_3",
        "key_bn_L0_N0_4",
        "key_bn_L0_N0_5",
        "key_bn_L0_N0_6",
    ];
    let write_keys = [
        "key_bn_L0_N0_7",
        "key_bn_L0_N0_8",
        "key_bn_L0_N0_9",
    ];
    let new_val = "MIXED_WORKLOAD_VALUE";

    // Common BenchmarkId input label — all parsers run the same logical workload
    let workload_id = "bushy_70R_30W";

    // --- Exlex ---
    // The immutable core is parsed once and shared. Each iteration creates a fresh
    // lightweight mutator overlay (clears arena + resets delta vectors).
    // Reads go directly to the immutable core; writes accumulate in the arena.
    // This is the intended usage pattern for a runtime config store.
    group.bench_with_input(
        BenchmarkId::new("Exlex", workload_id),
        &(),
        |b, _| {
            let mut arena  = ExlexArena(String::with_capacity(512));
            let mut buffer = String::with_capacity(512);
            b.iter(|| {
                arena.clear();
                let mut mutator = exlex_parsed
                    .init_mutator(&mut arena, &mut buffer)
                    .unwrap();
                // 7 reads — hit immutable flat arrays directly
                for key in &read_keys {
                    let val = exlex_parsed
                        .get_property(black_box(key), target_sect_id)
                        .unwrap();
                    black_box(val);
                }
                // 3 writes — append new values to arena, record deltas
                for key in &write_keys {
                    mutator.update_prop(black_box(key), black_box(new_val), target_sect_id);
                }
                black_box(&mutator);
            })
        },
    );

    // --- TOML Edit ---
    // Mutable document approach. Clone outside timer (iter_batched), then
    // perform reads and writes on the live document. This reflects TOML Edit's
    // intended usage — the document IS the mutable state.
    group.bench_with_input(
        BenchmarkId::new("TomlEdit", workload_id),
        &(),
        |b, _| {
            b.iter_batched(
                || toml_doc.clone(),
                |mut doc| {
                    // 7 reads
                    for key in &read_keys {
                        let val = doc[black_box(target_section)][black_box(key)]
                            .as_str()
                            .unwrap();
                        black_box(val);
                    }
                    // 3 writes
                    for key in &write_keys {
                        doc[black_box(target_section)][black_box(key)] =
                            toml_edit::value(black_box(new_val));
                    }
                    black_box(doc);
                },
                criterion::BatchSize::SmallInput,
            )
        },
    );

    // --- Serde JSON (Read-Only Baseline) ---
    // Serde JSON has no mutation API. This measures only the 7-read portion,
    // establishing what the read half of the workload costs in isolation.
    // A fair competitor for the full workload would require cloning the entire
    // Value tree per mutation, making it structurally equivalent to TOML Edit.
    group.bench_with_input(
        BenchmarkId::new("SerdeJSON_ReadBaseline", workload_id),
        &(),
        |b, _| {
            b.iter(|| {
                for key in &read_keys {
                    let val = json_parsed[black_box(target_section)][black_box(key)]
                        .as_str()
                        .unwrap();
                    black_box(val);
                }
            })
        },
    );

    group.finish();
}

criterion_group!(benches, bench_mixed_workload);
criterion_main!(benches);