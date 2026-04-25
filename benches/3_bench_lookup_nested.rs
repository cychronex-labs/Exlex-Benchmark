use criterion::{Criterion, black_box, criterion_group, criterion_main};
use exlex::Exlex;

#[path = "common/mod.rs"]
mod common;

fn bench_lookup_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("Lookup_Nested");

    // We use the deep_sparse matrix: 8 levels deep, 1 child per level.
    // Worst case scenario for get_child_path.
    let exlex_str = common::loader::load_fixture("deep_sparse_path6", "exlex");
    let json_str = common::loader::load_fixture("deep_sparse_path6", "json");
    let toml_str = common::loader::load_fixture("deep_sparse_path6", "toml");

    let exlex_parsed = Exlex::init_reader(&exlex_str, None, None, None, None).unwrap();
    let json_parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let toml_parsed: toml::Value = toml_str.parse().unwrap();

    // The path down to Level 6
    let path = [
        "ds_L0_N0", "ds_L1_N0", "ds_L2_N0", "ds_L3_N0", "ds_L4_N0", "ds_L5_N0", "ds_L6_N0",
    ];
    let target_key = "key_ds_L6_N0_1";

    group.bench_function("Exlex_Path_Depth_6", |b| {
        b.iter(|| {
            // Start at the root
            let mut current_sect = exlex_parsed.get_root();

            // Iteratively dive down the section path
            for &sect_name in black_box(&path) {
                current_sect = exlex_parsed.get_child(sect_name, current_sect).unwrap();
            }

            // Finally, fetch the property in that deeply nested section
            let val = exlex_parsed
                .get_property(black_box(target_key), current_sect)
                .unwrap();

            black_box(val);
        })
    });

    // 2. Serde JSON Nested Object Chaining
    group.bench_function("Serde_Path_Depth_6", |b| {
        b.iter(|| {
            let val = json_parsed
                .get(black_box(path[0]))
                .unwrap()
                .get(black_box(path[1]))
                .unwrap()
                .get(black_box(path[2]))
                .unwrap()
                .get(black_box(path[3]))
                .unwrap()
                .get(black_box(path[4]))
                .unwrap()
                .get(black_box(path[5]))
                .unwrap()
                .get(black_box(path[6]))
                .unwrap()
                .get(black_box(target_key))
                .unwrap()
                .as_str()
                .unwrap();
            black_box(val);
        })
    });

    // 3. TOML Nested Table Chaining
    group.bench_function("Toml_Path_Depth_6", |b| {
        b.iter(|| {
            let val = toml_parsed
                .get(black_box(path[0]))
                .unwrap()
                .get(black_box(path[1]))
                .unwrap()
                .get(black_box(path[2]))
                .unwrap()
                .get(black_box(path[3]))
                .unwrap()
                .get(black_box(path[4]))
                .unwrap()
                .get(black_box(path[5]))
                .unwrap()
                .get(black_box(path[6]))
                .unwrap()
                .get(target_key)
                .unwrap()
                .as_str()
                .unwrap();
            black_box(val);
        })
    });
    group.bench_function("Exlex_Get_Child", |b| {
        let root = exlex_parsed.get_root();
        b.iter(|| {
            let sect = exlex_parsed
                .get_child(black_box(path[0]), black_box(root))
                .unwrap();
            black_box(sect);
        })
    });

    group.bench_function("Serde_Get_Object", |b| {
        b.iter(|| {
            let obj = json_parsed.get(black_box(path[0])).unwrap();
            black_box(obj);
        })
    });
    group.finish();
}

criterion_group!(benches, bench_lookup_nested);
criterion_main!(benches);
