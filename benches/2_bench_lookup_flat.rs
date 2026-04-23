use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use exlex::Exlex;

#[path = "common/mod.rs"]
mod common;

fn bench_lookup_flat(c: &mut Criterion) {
    let mut group = c.benchmark_group("Lookup_Flat");

    // We use the flat_dense topology because it has 100 properties per section,
    // which stresses the linear scan of the hash arrays.
    let exlex_str = common::loader::load_fixture("flat_dense_last", "exlex");
    let json_str = common::loader::load_fixture("flat_dense_last", "json");

    let exlex_parsed = Exlex::init_reader(&exlex_str, None, None, None, None).unwrap();
    let json_parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let toml_str = common::loader::load_fixture("flat_dense_last", "toml");
    let ini_str = common::loader::load_fixture("flat_dense_last", "ini");
    let toml_parsed: toml::Value = toml_str.parse().unwrap();
    let ini_parsed = ini::Ini::load_from_str(&ini_str).unwrap();
    let target_section = "fd_L0_N0";
    let target_section_id = exlex_parsed
        .get_child(black_box(target_section), exlex_parsed.get_root())
        .unwrap();

    // 1. Worst Case Scenario: The last property in a dense section
    let last_key = "key_fd_L0_N0_99";
    group.bench_function("Exlex_Dense_Last", |b| {
        b.iter(|| {
            let val = exlex_parsed
                .get_property(black_box(last_key), target_section_id)
                .unwrap();
            black_box(val);
        })
    });
    group.bench_function("Serde_Dense_Last", |b| {
        b.iter(|| {
            let val = json_parsed[black_box(target_section)][black_box(last_key)]
                .as_str()
                .unwrap();
            black_box(val);
        })
    });

    // Inside the Dense_Last section:
    group.bench_function("Toml_Dense_Last", |b| {
        b.iter(|| {
            let val = toml_parsed[black_box(target_section)][black_box(last_key)]
                .as_str()
                .unwrap();
            black_box(val);
        })
    });
    group.bench_function("Ini_Dense_Last", |b| {
        b.iter(|| {
            let section = ini_parsed.section(Some(black_box(target_section))).unwrap();
            let val = section.get(black_box(last_key)).unwrap();
            black_box(val);
        })
    });
    // 2. Missing Key Scenario: Requires full linear scan of the bucket before failing
    let missing_key = "key_that_does_not_exist";
    group.bench_function("Exlex_Dense_Missing", |b| {
        b.iter(|| {
            let val = exlex_parsed.get_property(black_box(missing_key), target_section_id);
            black_box(val);
        })
    });
    group.bench_function("Serde_Dense_Missing", |b| {
        b.iter(|| {
            let val = json_parsed[black_box(target_section)].get(black_box(missing_key));
            black_box(val);
        })
    });

    // 3. Hash Collision Scenario
    // Dynamically generate a collision pair to test the `while let Some(rel_idx)` probe loop
    let (key1, key2) = common::collision_util::get_hash_collision_pair();

    // We must manually inject these keys into an Exlex parser instance to test it
    let mut collision_data = String::from("sect \"collision_test\" {\n");
    collision_data.push_str(&format!("  \"{}\": \"val1\"\n", key1));
    collision_data.push_str(&format!("  \"{}\": \"val2\"\n", key2)); // The target
    collision_data.push_str("}\n");

    let collision_exlex = Exlex::init_reader(&collision_data, None, None, None, None).unwrap();
    let collision_sect = collision_exlex
        .get_child("collision_test", collision_exlex.get_root())
        .unwrap();

    group.bench_function("Exlex_Hash_Collision_Probe", |b| {
        b.iter(|| {
            // This forces Exlex to match the hash, check the string (it fails),
            // and probe forward to the next index.
            let val = collision_exlex
                .get_property(black_box(&key2), collision_sect)
                .unwrap();
            black_box(val);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_lookup_flat);
criterion_main!(benches);
