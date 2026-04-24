// Copyright 2026 Abdul Wahab Melethil Shibu (cychronex-labs)
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-APACHE> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Benchmark: Cold Start
//
// Measures the complete application boot path: parse config from scratch,
// traverse to the target section, and read the first critical value.
// This is the most universally representative real-world scenario —
// every application does exactly this once at startup.
//
// Hardware: Intel Core i3-6006U, 16GB DDR4-2133, IPC ~1.7, TLB miss 0.07%

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use exlex::Exlex;

#[path = "common/mod.rs"]
mod common;

// Maps each topology to (section, key) representing the first value an app would read.
// Deliberately targets key_*_0 (position 0) to remove lookup scan cost from the measurement —
// we are timing the parse + traverse cost, not the search cost.
fn topology_target(topo: &str) -> (&'static str, &'static str) {
    match topo {
        "bushy_normal_middle"  => ("bn_L0_N0", "key_bn_L0_N0_0"),
        "flat_dense_last"      => ("fd_L0_N0", "key_fd_L0_N0_0"),
        "lopsided_dense_last"  => ("ld_heavy",  "heavy_key_0"),
        _                      => ("section",   "key"),
    }
}

fn bench_cold_start(c: &mut Criterion) {
    let mut group = c.benchmark_group("Cold_Start");

    // Three topologies chosen for distinct structural characteristics:
    // bushy_normal_middle  — realistic app config (tree, moderate density)
    // flat_dense_last      — service config (flat, 100 props per section)
    // lopsided_dense_last  — feature flag store (one heavy section, many empty)
    let topologies = [
        "bushy_normal_middle",
        "flat_dense_last",
        "lopsided_dense_last",
    ];

    for topo in topologies {
        let exlex_str = common::loader::load_fixture(topo, "exlex");
        let json_str  = common::loader::load_fixture(topo, "json");
        let toml_str  = common::loader::load_fixture(topo, "toml");
        let ini_str   = common::loader::load_fixture(topo, "ini");

        let (target_section, target_key) = topology_target(topo);

        // --- Exlex ---
        group.bench_with_input(BenchmarkId::new("Exlex", topo), &exlex_str, |b, data| {
            b.iter(|| {
                let parsed = Exlex::init_reader(black_box(data), None, None, None, None).unwrap();
                let sect   = parsed.get_child(black_box(target_section), parsed.get_root()).unwrap();
                let val    = parsed.get_property(black_box(target_key), sect).unwrap();
                black_box(val);
            })
        });

        // --- Serde JSON ---
        group.bench_with_input(BenchmarkId::new("SerdeJSON", topo), &json_str, |b, data| {
            b.iter(|| {
                let parsed: serde_json::Value = serde_json::from_str(black_box(data)).unwrap();
                let val = parsed[black_box(target_section)][black_box(target_key)]
                    .as_str()
                    .unwrap();
                black_box(val);
            })
        });

        // --- TOML Edit ---
        // Used over plain toml crate because it is the mutation competitor and
        // the most commonly used in production for read+write workflows.
        group.bench_with_input(BenchmarkId::new("TomlEdit", topo), &toml_str, |b, data| {
            b.iter(|| {
                let parsed = black_box(data).parse::<toml_edit::DocumentMut>().unwrap();
                let val = parsed[black_box(target_section)][black_box(target_key)]
                    .as_str()
                    .unwrap();
                black_box(val);
            })
        });

        // --- Rust INI ---
        // Included because INI is the structurally closest flat-section format.
        // Note: INI fixtures for bushy topology use top-level section names only
        // (children become dot-separated, e.g. "bn_L0_N0.bn_L1_N0") which
        // means this benchmark hits the same section as Exlex.
        group.bench_with_input(BenchmarkId::new("INI", topo), &ini_str, |b, data| {
            b.iter(|| {
                let parsed  = ini::Ini::load_from_str(black_box(data)).unwrap();
                let section = parsed.section(Some(black_box(target_section))).unwrap();
                let val     = section.get(black_box(target_key)).unwrap();
                black_box(val);
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_cold_start);
criterion_main!(benches);