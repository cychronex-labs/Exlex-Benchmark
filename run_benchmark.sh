#!/bin/bash
set -eo pipefail

echo "=> Verifying toolchain..."
if ! command -v cargo &> /dev/null; then echo "ERROR: cargo not found."; exit 1; fi
if ! command -v jq &> /dev/null; then echo "ERROR: jq is required."; exit 1; fi

TIMESTAMP=$(date +"%Y-%m-%d_%H-%M-%S")
RESULTS_DIR="results/${TIMESTAMP}"
mkdir -p "$RESULTS_DIR"

echo "=> Phase 1: Generating Multi-Dimensional Fixtures..."
cargo run --bin gen_fixtures --release

echo -e "\n=> Phase 2: Executing Benchmark Matrix..."
# Enable ALL benchmarks now that we've patched the panics
cargo bench --quiet || echo "Warning: Some benchmarks failed, but archiving completed data."

echo -e "\n=> Phase 3: Archiving Raw Data & Flushing OS Buffers..."
cp -r target/criterion "$RESULTS_DIR/raw_criterion_backup"
sync

echo -e "\n=> Phase 4: Statistical Summary"
SUMMARY_FILE="$RESULTS_DIR/summary.txt"
printf "%-35s | %-15s | %-15s | %-15s\n" "Benchmark Matrix" "Exlex" "Serde JSON" "Sonic-RS (SIMD)" | tee "$SUMMARY_FILE"
printf "%-35s | %-15s | %-15s | %-15s\n" "-----------------------------------" "---------------" "---------------" "---------------" | tee -a "$SUMMARY_FILE"

extract_time() {
    local group=$1
    local id=$2
    local file="$RESULTS_DIR/raw_criterion_backup/${group}/${id}/new/estimates.json"
    if [ -f "$file" ]; then
        local raw_val=$(jq -r '.mean.point_estimate' "$file")
        LC_NUMERIC=C printf "%.1f ns" "$raw_val"
    else
        echo "N/A"
    fi
}

# FIXED: Loop matching 1_bench_parse.rs topologies
for topo in flat_sparse_first flat_dense_last deep_sparse_path6 bushy_normal_middle lopsided_dense_last; do
    EXLEX_T=$(extract_time "Parse_Init" "exlex/${topo}")
    SERDE_T=$(extract_time "Parse_Init" "serde_json/${topo}")
    SONIC_T=$(extract_time "Parse_Init" "sonic_rs/${topo}")
    printf "%-35s | %-15s | %-15s | %-15s\n" "Parse: ${topo}" "$EXLEX_T" "$SERDE_T" "$SONIC_T" | tee -a "$SUMMARY_FILE"
done

printf "%-35s | %-15s | %-15s | %-15s\n" "-----------------------------------" "---------------" "---------------" "---------------" | tee -a "$SUMMARY_FILE"

for op in Dense_Last Dense_Missing Hash_Collision_Probe; do
    EXLEX_T=$(extract_time "Lookup_Flat" "Exlex_${op}")
    SERDE_T=$(extract_time "Lookup_Flat" "Serde_${op}")
    printf "%-35s | %-15s | %-15s | %-15s\n" "Lookup: ${op}" "$EXLEX_T" "$SERDE_T" "N/A" | tee -a "$SUMMARY_FILE"
done

echo -e "\n=> Nested Traversal Benchmarks"
EXLEX_T=$(extract_time "Lookup_Nested" "Exlex_Path_Depth_6")
SERDE_T=$(extract_time "Lookup_Nested" "Serde_Path_Depth_6")
TOML_T=$(extract_time "Lookup_Nested" "Toml_Path_Depth_6")
printf "%-35s | %-15s | %-15s | %-15s\n" "Depth 6 Pointer Hopping" "$EXLEX_T" "$SERDE_T" "TOML: $TOML_T" | tee -a "$SUMMARY_FILE"

echo -e "\n=> Iteration Benchmarks"
EXLEX_T=$(extract_time "Iteration_Drain" "Exlex_Iterate_Section")
SERDE_T=$(extract_time "Iteration_Drain" "Serde_Iterate_Object")
TOML_T=$(extract_time "Iteration_Drain" "Toml_Iterate_Table")
printf "%-35s | %-15s | %-15s | %-15s\n" "Sequential Flat Drain" "$EXLEX_T" "$SERDE_T" "TOML: $TOML_T" | tee -a "$SUMMARY_FILE"

echo -e "\n=> Arena Mutator Benchmarks"
EXLEX_T=$(extract_time "Mutation_Arena" "Exlex_Update_Existing")
TOML_T=$(extract_time "Mutation_Arena" "TomlEdit_Update_Existing")
printf "%-35s | %-15s | %-15s | %-15s\n" "Update Existing Prop" "$EXLEX_T" "N/A" "TOML: $TOML_T" | tee -a "$SUMMARY_FILE"

EXLEX_T=$(extract_time "Mutation_Arena" "Exlex_Add_Section")
TOML_T=$(extract_time "Mutation_Arena" "TomlEdit_Add_Section")
printf "%-35s | %-15s | %-15s | %-15s\n" "Allocate New Section" "$EXLEX_T" "N/A" "TOML: $TOML_T" | tee -a "$SUMMARY_FILE"

EXLEX_T=$(extract_time "Mutation_Arena" "Exlex_Delete_Property")
TOML_T=$(extract_time "Mutation_Arena" "TomlEdit_Delete_Property")
printf "%-35s | %-15s | %-15s | %-15s\n" "Delete Property" "$EXLEX_T" "N/A" "TOML: $TOML_T" | tee -a "$SUMMARY_FILE"

echo -e "\n=> Correctness Oracle Pipeline"
EXLEX_T=$(extract_time "Roundtrip_Pipeline" "Exlex_Parse_Mutate_Save")
TOML_T=$(extract_time "Roundtrip_Pipeline" "TomlEdit_Parse_Mutate_Save")
printf "%-35s | %-15s | %-15s | %-15s\n" "Parse -> Mutate -> Save" "$EXLEX_T" "N/A" "TOML: $TOML_T" | tee -a "$SUMMARY_FILE"

EXLEX_T=$(extract_time "Roundtrip_Pipeline" "Exlex_Mass_Update_Save")
TOML_T=$(extract_time "Roundtrip_Pipeline" "TomlEdit_Mass_Update_Save")
printf "%-35s | %-15s | %-15s | %-15s\n" "50x Mutate -> Save" "$EXLEX_T" "N/A" "TOML: $TOML_T" | tee -a "$SUMMARY_FILE"

echo -e "\n=> Benchmark complete. Data synced and permanently archived to $RESULTS_DIR"