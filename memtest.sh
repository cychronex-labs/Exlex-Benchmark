RESULTS_DIR="results/memtest_$(date +"%Y-%m-%d_%H-%M-%S")"
mkdir -p "$RESULTS_DIR"

echo -e "\n=> Phase 5: The Allocation Audit (Process Isolated)"
cargo build --bin alloc_audit --release --quiet

MEM_FILE="$RESULTS_DIR/memory_results.json"
echo "[" > "$MEM_FILE"

# We isolate Exlex, Serde, and TOML Edit for the Memory Audit
PARSERS=("exlex" "serde" "toml_edit")
OPS=("parse" "mutate")

FIRST=1
for OP in "${OPS[@]}"; do
    for P in "${PARSERS[@]}"; do
        if [ $FIRST -eq 0 ]; then echo "," >> "$MEM_FILE"; fi
        # Calling the binary independently zeroes out the OS RAM buffer
        ./target/release/alloc_audit --parser "$P" --op "$OP" >> "$MEM_FILE"
        FIRST=0
    done
done
echo "]" >> "$MEM_FILE"
echo "=> Memory profiling complete. Data saved to $MEM_FILE"
