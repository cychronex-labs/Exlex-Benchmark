use criterion::{Criterion, black_box, criterion_group, criterion_main};
use exlex::{Exlex, ExlexArena};

#[path = "common/mod.rs"]
mod common;

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("Roundtrip_Pipeline");

    let exlex_str = common::loader::load_fixture("mut_roundtrip_verify", "exlex");
    let toml_str = common::loader::load_fixture("mut_roundtrip_verify", "toml");

    let target_section = "bn_L0_N0";
    let new_val = "ROUNDTRIP_SUCCESS";

    let key0 = "key_bn_L0_N0_0";
    let key1 = "key_bn_L0_N0_1";
    let key2 = "key_bn_L0_N0_2";

    group.bench_function("Exlex_Parse_Mutate_Save", |b| {
        let mut arena = ExlexArena(String::with_capacity(4096));
        let mut buffer = String::with_capacity(exlex_str.len() * 2);

        b.iter(|| {
            arena.clear();
            let parsed = Exlex::init_reader(black_box(&exlex_str), None, None, None, None).unwrap();
            let sect_id = parsed
                .get_child(black_box(target_section), parsed.get_root())
                .unwrap();

            let mut mutator = parsed.init_mutator(&mut arena, &mut buffer).unwrap();

            mutator.update_prop(black_box(key0), black_box(new_val), sect_id);
            mutator.update_prop(black_box(key1), black_box(new_val), sect_id);
            mutator.update_prop(black_box(key2), black_box(new_val), sect_id);

            mutator.save();
            black_box(&buffer);
        })
    });

    group.bench_function("TomlEdit_Parse_Mutate_Save", |b| {
        b.iter(|| {
            let mut doc = black_box(&toml_str)
                .parse::<toml_edit::DocumentMut>()
                .unwrap();

            doc[black_box(target_section)][black_box(key0)] = toml_edit::value(black_box(new_val));
            doc[black_box(target_section)][black_box(key1)] = toml_edit::value(black_box(new_val));
            doc[black_box(target_section)][black_box(key2)] = toml_edit::value(black_box(new_val));

            let out = doc.to_string();
            black_box(out);
        })
    });

    // --- Mass Update Write Path ---
    let lopsided_exlex_str = common::loader::load_fixture("mut_mass_update_save", "exlex");
    let lopsided_toml_str = common::loader::load_fixture("mut_mass_update_save", "toml");
    
    let lopsided_exlex = Exlex::init_reader(&lopsided_exlex_str, None, None, None, None).unwrap();
    let heavy_sect_id = lopsided_exlex
        .get_child("ld_heavy", lopsided_exlex.get_root())
        .unwrap();

    let heavy_keys: Vec<String> = (0..50).map(|i| format!("heavy_key_{}", i)).collect();

    group.bench_function("Exlex_Mass_Update_Save", |b| {
        let mut arena = ExlexArena(String::with_capacity(8192));
        let mut buffer = String::with_capacity(lopsided_exlex_str.len() * 2);

        b.iter(|| {
            arena.clear();
            let mut mutator = lopsided_exlex
                .init_mutator(&mut arena, &mut buffer)
                .unwrap();

            for key in &heavy_keys {
                mutator.update_prop(
                    black_box(key),
                    black_box("mass_updated_value"),
                    heavy_sect_id,
                );
            }

            mutator.save();
            black_box(&buffer);
        })
    });

    group.bench_function("TomlEdit_Mass_Update_Save", |b| {
        b.iter_batched(
            || lopsided_toml_str.parse::<toml_edit::DocumentMut>().unwrap(), // FIXED FIXTURE TYPE
            |mut doc| {
                for key in &heavy_keys {
                    doc["ld_heavy"][black_box(key)] = toml_edit::value("mass_updated_value");
                }
                let out = doc.to_string();
                black_box(out);
            },
            criterion::BatchSize::SmallInput,
        )
    });
    group.finish();
}

criterion_group!(benches, bench_roundtrip);
criterion_main!(benches);