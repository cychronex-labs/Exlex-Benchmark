use dhat::{Alloc, HeapStats, Profiler};
use exlex::{Exlex, ExlexArena};
use std::env;

#[path = "../../benches/common/mod.rs"]
mod common;

#[global_allocator]
static ALLOC: Alloc = Alloc;

fn main() {
    let _profiler = Profiler::builder().testing().build();

    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        panic!("Usage: alloc_audit --parser <name> --op <parse|mutate>");
    }

    let parser = args.get(2).expect("Missing --parser");
    let op = args.get(4).expect("Missing --op");

    let ext = match parser.as_str() {
        "exlex" => "exlex",
        "serde" => "json",
        "toml_edit" => "toml",
        _ => panic!("Unsupported parser target"),
    };

    let file_str = common::loader::load_fixture("bushy_normal_middle", ext);

    // Pre-generate 5,000 strings OUTSIDE the profiler.
    // We are going to hammer the parsers with massive sequential updates.
    let massive_vals: Vec<String> = (0..5000).map(|i| format!("heavy_val_string_{}", i)).collect();

    let (blocks, bytes) = match (parser.as_str(), op.as_str()) {
        ("exlex", "parse") => {
            let start = HeapStats::get();
            let _parsed = Exlex::init_reader(&file_str, None, None, None, None).unwrap();
            let end = HeapStats::get();
            (end.total_blocks - start.total_blocks, end.total_bytes - start.total_bytes)
        }
        ("exlex", "mutate") => {
            let parsed = Exlex::init_reader(&file_str, None, None, None, None).unwrap();
            let sect_id = parsed.get_child("bn_L0_N0", parsed.get_root()).unwrap();

            // 1. START TIMER HERE: We honestly capture the 8KB Arena tax
            let start = HeapStats::get();

            // 2. Pre-allocate the arena to absorb the 5,000 updates
            let mut arena = ExlexArena(String::with_capacity(8192));
            let mut buffer = String::with_capacity(8192);
            let mut mutator = parsed.init_mutator(&mut arena, &mut buffer).unwrap();

            // 3. THE HEAVY WORKLOAD
            // Exlex will pack all 5,000 strings into the pre-allocated arena
            for i in 0..5000 {
                mutator.update_prop("key_bn_L0_N0_0", &massive_vals[i], sect_id);
            }

            let end = HeapStats::get();
            (end.total_blocks - start.total_blocks, end.total_bytes - start.total_bytes)
        }
        ("serde", "parse") => {
            let start = HeapStats::get();
            let _parsed: serde_json::Value = serde_json::from_str(&file_str).unwrap();
            let end = HeapStats::get();
            (end.total_blocks - start.total_blocks, end.total_bytes - start.total_bytes)
        }
        ("serde", "mutate") => {
            let mut parsed: serde_json::Value = serde_json::from_str(&file_str).unwrap();

            let start = HeapStats::get();

            // THE HEAVY WORKLOAD
            // Serde will `malloc` 5,000 brand new heap strings
            for i in 0..5000 {
                parsed["bn_L0_N0"]["key_bn_L0_N0_0"] = serde_json::Value::String(massive_vals[i].clone());
            }

            let end = HeapStats::get();
            (end.total_blocks - start.total_blocks, end.total_bytes - start.total_bytes)
        }
        ("toml_edit", "parse") => {
            let start = HeapStats::get();
            let _parsed = file_str.parse::<toml_edit::DocumentMut>().unwrap();
            let end = HeapStats::get();
            (end.total_blocks - start.total_blocks, end.total_bytes - start.total_bytes)
        }
        ("toml_edit", "mutate") => {
            let mut parsed = file_str.parse::<toml_edit::DocumentMut>().unwrap();

            let start = HeapStats::get();

            // THE HEAVY WORKLOAD
            // TOML Edit will `malloc` 5,000 brand new heap strings
            for i in 0..5000 {
                parsed["bn_L0_N0"]["key_bn_L0_N0_0"] = toml_edit::value(&massive_vals[i]);
            }

            let end = HeapStats::get();
            (end.total_blocks - start.total_blocks, end.total_bytes - start.total_bytes)
        }
        _ => (0, 0),
    };

    let parser_name = match parser.as_str() {
        "exlex" => "Exlex (DOD)",
        "serde" => "Serde JSON",
        "toml_edit" => "TOML Edit",
        _ => "Unknown",
    };

    println!(
        "{{\"parser\": \"{}\", \"op\": \"{}\", \"blocks\": {}, \"bytes\": {}}}",
        parser_name, op, blocks, bytes
    );
}
