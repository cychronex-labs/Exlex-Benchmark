use criterion::{Criterion, black_box, criterion_group, criterion_main};
use exlex::{Exlex, ExlexArena};

#[path = "common/mod.rs"]
mod common;

fn bench_mutate(c: &mut Criterion) {
    let mut group = c.benchmark_group("Mutation_Arena");

    let exlex_str = common::loader::load_fixture("mut_update_existing", "exlex");
    let toml_str = common::loader::load_fixture("mut_update_existing", "toml");

    let exlex_parsed = Exlex::init_reader(&exlex_str, None, None, None, None).unwrap();
    let toml_doc = toml_str.parse::<toml_edit::DocumentMut>().unwrap();

    let target_section = "bn_L0_N0";
    let target_sect_id = exlex_parsed
        .get_child(target_section, exlex_parsed.get_root())
        .unwrap();
    let existing_key = "key_bn_L0_N0_0";
    let new_val = "MUTATED_VALUE";

    // 1. Update Existing Property
    group.bench_function("Exlex_Update_Existing", |b| {
        let mut arena = ExlexArena(String::with_capacity(1024));
        let mut buffer = String::with_capacity(1024);
        b.iter(|| {
            arena.clear(); // CRITICAL MEMORY LEAK FIX
            let mut mutator = exlex_parsed.init_mutator(&mut arena, &mut buffer).unwrap();
            mutator.update_prop(black_box(existing_key), black_box(new_val), target_sect_id);
            black_box(&mutator);
        })
    });

    group.bench_function("TomlEdit_Update_Existing", |b| {
        b.iter_batched(
            || toml_doc.clone(), // Clone outside timing loop
            |mut doc| {
                doc[black_box(target_section)][black_box(existing_key)] =
                    toml_edit::value(black_box(new_val));
                black_box(doc);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // 2. Add New Section
    let new_sect_name = "BRAND_NEW_SECTION";
    group.bench_function("Exlex_Add_Section", |b| {
        let mut arena = ExlexArena(String::with_capacity(1024));
        let mut buffer = String::with_capacity(1024);
        b.iter(|| {
            arena.clear();
            let mut mutator = exlex_parsed.init_mutator(&mut arena, &mut buffer).unwrap();
            mutator
                .new_section(black_box(new_sect_name), exlex_parsed.get_root())
                .unwrap();
            black_box(&mutator);
        })
    });

    group.bench_function("TomlEdit_Add_Section", |b| {
        b.iter_batched(
            || toml_doc.clone(),
            |mut doc| {
                let table = toml_edit::Table::new();
                doc.insert(black_box(new_sect_name), toml_edit::Item::Table(table));
                black_box(doc);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // 3. Delete Property
    group.bench_function("Exlex_Delete_Property", |b| {
        let mut arena = ExlexArena(String::with_capacity(1024));
        let mut buffer = String::with_capacity(1024);
        b.iter(|| {
            arena.clear();
            let mut mutator = exlex_parsed.init_mutator(&mut arena, &mut buffer).unwrap();
            mutator
                .delete_property(black_box(existing_key), target_sect_id)
                .unwrap();
            black_box(&mutator);
        })
    });

    group.bench_function("TomlEdit_Delete_Property", |b| {
        b.iter_batched(
            || toml_doc.clone(), // PHANTOM DELETION FIX
            |mut doc| {
                doc[black_box(target_section)]
                    .as_table_mut()
                    .unwrap()
                    .remove(black_box(existing_key));
                black_box(doc);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // 4. Delete Section
    group.bench_function("Exlex_Delete_Section", |b| {
        let mut arena = ExlexArena(String::with_capacity(1024));
        let mut buffer = String::with_capacity(1024);
        b.iter(|| {
            arena.clear();
            let mut mutator = exlex_parsed.init_mutator(&mut arena, &mut buffer).unwrap();
            mutator.delete_section(black_box(target_sect_id));
            black_box(&mutator);
        })
    });

    group.bench_function("TomlEdit_Delete_Section", |b| {
        b.iter_batched(
            || toml_doc.clone(),
            |mut doc| {
                doc.remove(black_box(target_section));
                black_box(doc);
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bench_mutate);
criterion_main!(benches);
