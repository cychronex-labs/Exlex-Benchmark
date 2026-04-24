#!/bin/bash

# =============================================================================
# Exlex Benchmark Suite
# Hardware: Intel Core i3-6006U | 16GB DDR4-2133MHz | IPC ~1.7 | TLB 0.07%
# All results must be interpreted in the context of this hardware profile.
# L1: 32KB/core | L2: 256KB/core | L3: 3MB shared
# =============================================================================

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
cargo bench --quiet || echo "Warning: Some benchmarks failed, but archiving completed data."

echo -e "\n=> Phase 3: Archiving Raw Data & Flushing OS Buffers..."
cp -r target/criterion "$RESULTS_DIR/raw_criterion_backup"
sync

echo -e "\n=> Phase 4: Statistical Summary"
SUMMARY_FILE="$RESULTS_DIR/summary.txt"

# Hardware context written into the summary file for traceability
cat >> "$SUMMARY_FILE" << 'HWEOF'
================================================================================
Hardware: Intel Core i3-6006U | 16GB DDR4-2133MHz | IPC ~1.7 | TLB miss 0.07%
L1: 32KB/core | L2: 256KB/core | L3: 3MB shared
All timings in nanoseconds. Lower is better.
================================================================================
HWEOF

printf "%-35s | %-15s | %-15s | %-15s\n" "Benchmark Matrix" "Exlex" "Serde JSON" "Sonic-RS (SIMD)" | tee -a "$SUMMARY_FILE"
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

# --- Parse Init ---
for topo in flat_sparse_first flat_dense_last deep_sparse_path6 bushy_normal_middle lopsided_dense_last; do
    EXLEX_T=$(extract_time "Parse_Init" "exlex/${topo}")
    SERDE_T=$(extract_time "Parse_Init" "serde_json/${topo}")
    SONIC_T=$(extract_time "Parse_Init" "sonic_rs/${topo}")
    printf "%-35s | %-15s | %-15s | %-15s\n" "Parse: ${topo}" "$EXLEX_T" "$SERDE_T" "$SONIC_T" | tee -a "$SUMMARY_FILE"
done

printf "%-35s | %-15s | %-15s | %-15s\n" "-----------------------------------" "---------------" "---------------" "---------------" | tee -a "$SUMMARY_FILE"

# --- Flat Lookup ---
for op in Dense_Last Dense_Missing Hash_Collision_Probe; do
    EXLEX_T=$(extract_time "Lookup_Flat" "Exlex_${op}")
    SERDE_T=$(extract_time "Lookup_Flat" "Serde_${op}")
    printf "%-35s | %-15s | %-15s | %-15s\n" "Lookup: ${op}" "$EXLEX_T" "$SERDE_T" "N/A" | tee -a "$SUMMARY_FILE"
done

# --- Nested Traversal ---
echo -e "\n=> Nested Traversal Benchmarks"
EXLEX_T=$(extract_time "Lookup_Nested" "Exlex_Path_Depth_6")
SERDE_T=$(extract_time "Lookup_Nested" "Serde_Path_Depth_6")
TOML_T=$(extract_time "Lookup_Nested" "Toml_Path_Depth_6")
printf "%-35s | %-15s | %-15s | %-15s\n" "Depth 6 Pointer Hopping" "$EXLEX_T" "$SERDE_T" "TOML: $TOML_T" | tee -a "$SUMMARY_FILE"

# --- Iteration ---
echo -e "\n=> Iteration Benchmarks"
EXLEX_T=$(extract_time "Iteration_Drain" "Exlex_Iterate_Section")
SERDE_T=$(extract_time "Iteration_Drain" "Serde_Iterate_Object")
TOML_T=$(extract_time "Iteration_Drain" "Toml_Iterate_Table")
printf "%-35s | %-15s | %-15s | %-15s\n" "Sequential Flat Drain" "$EXLEX_T" "$SERDE_T" "TOML: $TOML_T" | tee -a "$SUMMARY_FILE"

# --- Mutation Arena ---
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

# --- Roundtrip ---
echo -e "\n=> Correctness Oracle Pipeline"
EXLEX_T=$(extract_time "Roundtrip_Pipeline" "Exlex_Parse_Mutate_Save")
TOML_T=$(extract_time "Roundtrip_Pipeline" "TomlEdit_Parse_Mutate_Save")
printf "%-35s | %-15s | %-15s | %-15s\n" "Parse -> Mutate -> Save" "$EXLEX_T" "N/A" "TOML: $TOML_T" | tee -a "$SUMMARY_FILE"

EXLEX_T=$(extract_time "Roundtrip_Pipeline" "Exlex_Mass_Update_Save")
TOML_T=$(extract_time "Roundtrip_Pipeline" "TomlEdit_Mass_Update_Save")
printf "%-35s | %-15s | %-15s | %-15s\n" "50x Mutate -> Save" "$EXLEX_T" "N/A" "TOML: $TOML_T" | tee -a "$SUMMARY_FILE"

# --- Asymptotic ---
echo -e "\n=> Asymptotic Crossover (Cache Horizon)"
EXLEX_T=$(extract_time "Asymptotic_Crossover" "Exlex/50")
JSON_T=$(extract_time "Asymptotic_Crossover" "Serde JSON/50")
printf "%-35s | %-15s | %-15s | %-15s\n" "Linear vs HashMap (50 Items)" "$EXLEX_T" "$JSON_T" "N/A" | tee -a "$SUMMARY_FILE"

# --- Cold Start (NEW) ---
# Measures complete boot path: parse + section traverse + first key read.
# Represents every application's startup sequence.
echo -e "\n=> Cold Start Benchmarks (parse + traverse + first read)"
printf "%-35s | %-15s | %-15s | %-15s | %-15s\n" \
    "Cold Start Topology" "Exlex" "SerdeJSON" "TomlEdit" "INI" | tee -a "$SUMMARY_FILE"
printf "%-35s | %-15s | %-15s | %-15s | %-15s\n" \
    "-----------------------------------" "---------------" "---------------" "---------------" "---------------" | tee -a "$SUMMARY_FILE"

for topo in bushy_normal_middle flat_dense_last lopsided_dense_last; do
    EXLEX_T=$(extract_time "Cold_Start" "Exlex/${topo}")
    SERDE_T=$(extract_time "Cold_Start" "SerdeJSON/${topo}")
    TOML_T=$(extract_time "Cold_Start" "TomlEdit/${topo}")
    INI_T=$(extract_time "Cold_Start" "INI/${topo}")
    printf "%-35s | %-15s | %-15s | %-15s | %-15s\n" \
        "ColdStart: ${topo}" "$EXLEX_T" "$SERDE_T" "$TOML_T" "$INI_T" | tee -a "$SUMMARY_FILE"
done

# --- Mixed Workload (NEW) ---
# 70% reads + 30% writes interleaved on the same section.
# SerdeJSON column shows read-only cost (no mutation API available).
echo -e "\n=> Mixed Workload Benchmarks (70% read / 30% write)"
printf "%-35s | %-15s | %-15s | %-15s\n" \
    "Workload" "Exlex" "TomlEdit" "SerdeJSON (reads only)" | tee -a "$SUMMARY_FILE"
printf "%-35s | %-15s | %-15s | %-15s\n" \
    "-----------------------------------" "---------------" "---------------" "---------------" | tee -a "$SUMMARY_FILE"

EXLEX_T=$(extract_time "Mixed_Workload" "Exlex/bushy_70R_30W")
TOML_T=$(extract_time "Mixed_Workload" "TomlEdit/bushy_70R_30W")
SERDE_T=$(extract_time "Mixed_Workload" "SerdeJSON_ReadBaseline/bushy_70R_30W")
printf "%-35s | %-15s | %-15s | %-15s\n" \
    "bushy 70R/30W" "$EXLEX_T" "$TOML_T" "$SERDE_T" | tee -a "$SUMMARY_FILE"

# --- Allocation Audit ---
echo -e "\n=> Phase 5: The Allocation Audit (Process Isolated)"
cargo build --bin alloc_audit --release --quiet

MEM_FILE="$RESULTS_DIR/memory_results.json"
echo "[" > "$MEM_FILE"

PARSERS=("exlex" "serde" "toml_edit")
OPS=("parse" "mutate")

FIRST=1
for OP in "${OPS[@]}"; do
    for P in "${PARSERS[@]}"; do
        if [ $FIRST -eq 0 ]; then echo "," >> "$MEM_FILE"; fi
        ./target/release/alloc_audit --parser "$P" --op "$OP" >> "$MEM_FILE"
        FIRST=0
    done
done
echo "]" >> "$MEM_FILE"
echo "=> Memory profiling complete. Data saved to $MEM_FILE"

echo -e "\n=> Benchmark complete. Data synced and permanently archived to $RESULTS_DIR"