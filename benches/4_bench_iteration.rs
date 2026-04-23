use criterion::{Criterion, black_box, criterion_group, criterion_main};
use exlex::Exlex;

#[path = "common/mod.rs"]
mod common;

fn bench_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("Iteration_Drain");

    let exlex_str = common::loader::load_fixture("bushy_normal_sequential", "exlex");
    let json_str = common::loader::load_fixture("bushy_normal_sequential", "json");
    let toml_str = common::loader::load_fixture("bushy_normal_sequential", "toml");

    let exlex_parsed = Exlex::init_reader(&exlex_str, None, None, None, None).unwrap();
    let json_parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let toml_parsed: toml::Value = toml_str.parse().unwrap();

    let target_section = "bn_L0_N0";
    let json_section = json_parsed[target_section].as_object().unwrap();
    let toml_table = toml_parsed[target_section].as_table().unwrap();
    let target_sect_id = exlex_parsed
        .get_child(target_section, exlex_parsed.get_root())
        .unwrap();

    group.bench_function("Exlex_Iterate_Section", |b| {
        b.iter(|| {
            // Your iterator zips two flat array slices together
            for (k, v) in exlex_parsed.iter_section_properties(black_box(target_sect_id)) {
                black_box((k, v));
            }
        })
    });

    group.bench_function("Serde_Iterate_Object", |b| {
        b.iter(|| {
            for (k, v) in json_section.iter() {
                if let Some(s) = v.as_str(){ black_box((k, s));};
            }
        })
    });

    group.bench_function("Toml_Iterate_Table", |b| {
        b.iter(|| {
            for (k, v) in toml_table.iter() {
                if let Some(s) = v.as_str(){black_box((k, s));};
            }
        })
    });

    group.finish();
}

criterion_group!(benches, bench_iteration);
criterion_main!(benches);
