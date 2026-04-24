import json
import os
import sys

try:
    import plotly.graph_objects as go
    import plotly.io as pio
except ImportError:
    print("ERROR: Plotly is missing. Run: pip install plotly")
    sys.exit(1)

RESULTS_DIR = "results"

# Hardware context embedded into every dashboard for reproducibility.
# All numbers are meaningless without this anchor.
HW_PROFILE = {
    "cpu":    "Intel Core i3-6006U (Skylake, 2C/4T, 2.0GHz, no turbo)",
    "ram":    "16GB DDR4-2133MHz",
    "ipc":    "~1.7 (achieved)",
    "tlb":    "0.07% miss rate",
    "l1":     "32KB data / 32KB instruction per core",
    "l2":     "256KB per core",
    "l3":     "3MB shared",
}

if not os.path.exists(RESULTS_DIR):
    print(f"ERROR: {RESULTS_DIR} not found. Run ./run_benchmark.sh first.")
    sys.exit(1)

runs = sorted(
    [d for d in os.listdir(RESULTS_DIR) if os.path.isdir(os.path.join(RESULTS_DIR, d))]
)
if not runs:
    print("ERROR: No benchmark data found.")
    sys.exit(1)

latest_run = runs[-1]
backup_dir = os.path.join(RESULTS_DIR, latest_run, "raw_criterion_backup")

print(f"=> Analyzing Data from Run: {latest_run}")

db = {}

# --- Parse Criterion Speed Data ---
for root, dirs, files in os.walk(backup_dir):
    if "estimates.json" in files:
        with open(os.path.join(root, "estimates.json")) as f:
            data = json.load(f)
            point_est = data["mean"]["point_estimate"]

        rel_path = os.path.relpath(root, backup_dir)
        parts = rel_path.split(os.sep)

        if not parts or parts[-1] != "new":
            continue

        id_parts = parts[:-1]
        if not id_parts:
            continue

        group = id_parts[0]
        full_name = "_".join(id_parts).lower()

        # Parser name detection — order matters (toml_edit before toml)
        if "serde" in full_name:
            parser = "Serde JSON"
        elif "exlex" in full_name or "vml" in full_name:
            parser = "Exlex (DOD)"
        elif "toml_edit" in full_name or "tomledit" in full_name:
            parser = "TOML Edit"
        elif "toml" in full_name:
            parser = "TOML"
        elif "ini" in full_name:
            parser = "INI"
        elif "sonic" in full_name:
            parser = "Sonic-RS (SIMD)"
        elif "simd" in full_name:
            parser = "SIMD-JSON"
        elif "quick" in full_name:
            parser = "Quick-XML"
        elif "figment" in full_name:
            parser = "Figment (Layer)"
        else:
            parser = id_parts[1].capitalize() if len(id_parts) > 1 else "Unknown"

        if len(id_parts) == 2:
            scenario = id_parts[1]
        elif len(id_parts) >= 3:
            scenario = "_".join(id_parts[2:])
        else:
            scenario = "Default"

        if group not in db:
            db[group] = {}
        if scenario not in db[group]:
            db[group][scenario] = {}
        db[group][scenario][parser] = point_est

# --- Okabe-Ito Colorblind Safe Palette ---
COLOR_MAP = {
    "Exlex (DOD)":      "#00e5ff",
    "Serde JSON":       "#E69F00",
    "Sonic-RS (SIMD)":  "#009E73",
    "SIMD-JSON":        "#56B4E9",
    "TOML":             "#999999",
    "TOML Edit":        "#F0E442",
    "INI":              "#D55E00",
    "Quick-XML":        "#CC79A7",
    "Figment (Layer)":  "#0072B2",
}

# --- Annotations for specific benchmark groups ---
# These are displayed below the chart title to explain methodology decisions.
GROUP_ANNOTATIONS = {
    "Cold_Start": (
        "Measures parse + section traverse + first key read as one atomic operation. "
        "Represents every application's startup sequence. "
        "All parsers build a queryable structure from scratch per iteration."
    ),
    "Mixed_Workload": (
        "70% reads / 30% writes interleaved on the same section. "
        "Serde JSON column shows read-only cost — it has no mutation API. "
        "TOML Edit clones the document per iteration (its architecture cost). "
        "Exlex reuses the immutable parsed core and resets only the lightweight mutator overlay."
    ),
}

html_graphs = []

# --- Generate Criterion Speed Graphs ---
for group in sorted(db.keys()):
    scenarios = db[group]
    parsers_in_group = set()
    for s in scenarios.values():
        parsers_in_group.update(s.keys())

    fig = go.Figure()
    sorted_parsers = sorted(list(parsers_in_group))
    if "Exlex (DOD)" in sorted_parsers:
        sorted_parsers.remove("Exlex (DOD)")
        sorted_parsers.insert(0, "Exlex (DOD)")

    for p in sorted_parsers:
        x_vals = sorted(list(scenarios.keys()))
        y_vals = [scenarios[s].get(p, 0) for s in x_vals]
        fig.add_trace(
            go.Bar(name=p, x=x_vals, y=y_vals, marker_color=COLOR_MAP.get(p, "#ffffff"))
        )

    title_text = f"Benchmark Group: {group.replace('_', ' ').capitalize()}"
    annotation_text = GROUP_ANNOTATIONS.get(group, "")

    fig.update_layout(
        title=dict(text=title_text, font=dict(size=16)),
        barmode="group",
        template="plotly_dark",
        yaxis_title="Time (Nanoseconds) ↓ Lower is Better",
        xaxis_title="Topology / Scenario",
        font=dict(family="system-ui", size=14),
        margin=dict(t=80, b=40, l=40, r=40),
        legend=dict(orientation="h", yanchor="bottom", y=1.02, xanchor="right", x=1),
        annotations=[
            dict(
                text=annotation_text,
                xref="paper", yref="paper",
                x=0, y=1.12,
                showarrow=False,
                font=dict(size=11, color="#aaaaaa"),
                align="left",
            )
        ] if annotation_text else [],
    )
    html_graphs.append(fig.to_html(full_html=False, include_plotlyjs=False))

# --- Generate Allocation Audit Graphs ---
mem_file = os.path.join(RESULTS_DIR, latest_run, "memory_results.json")
if os.path.exists(mem_file):
    with open(mem_file) as f:
        mem_data = json.load(f)

    html_graphs.append(
        "<h2 style='color:#00e5ff; border-bottom: 1px solid #333; padding-bottom: 10px; margin-top: 50px;'>"
        "The Allocation Audit (Memory Overhead)</h2>"
    )

    metrics = [
        ("blocks", "Heap Allocations (Count)"),
        ("bytes",  "Heap Footprint (Bytes)"),
    ]

    for metric_key, metric_title in metrics:
        fig = go.Figure()

        parsers = sorted(
            list(set([d["parser"] for d in mem_data if d["parser"] != "Unknown"]))
        )
        if "Exlex (DOD)" in parsers:
            parsers.remove("Exlex (DOD)")
            parsers.insert(0, "Exlex (DOD)")

        for p in parsers:
            x_vals = ["Parse Init", "Mutation Overhead"]
            y_vals = [
                next((d[metric_key] for d in mem_data if d["parser"] == p and d["op"] == "parse"), 0),
                next((d[metric_key] for d in mem_data if d["parser"] == p and d["op"] == "mutate"), 0),
            ]
            fig.add_trace(
                go.Bar(name=p, x=x_vals, y=y_vals, marker_color=COLOR_MAP.get(p, "#ffffff"))
            )

        fig.update_layout(
            title=metric_title,
            barmode="group",
            template="plotly_dark",
            yaxis_title=metric_title + " ↓ Lower is Better",
            font=dict(family="system-ui", size=14),
            margin=dict(t=60, b=40, l=40, r=40),
            legend=dict(orientation="h", yanchor="bottom", y=1.02, xanchor="right", x=1),
        )
        html_graphs.append(fig.to_html(full_html=False, include_plotlyjs=False))

# --- Hardware profile card for the dashboard header ---
hw_card = f"""
<div style="max-width:1400px; margin: 0 auto 30px auto; background:#1a1a1a; border:1px solid #333;
     border-radius:8px; padding:14px 20px; font-size:12px; color:#888; display:flex;
     flex-wrap:wrap; gap:20px;">
  <span style="color:#00e5ff; font-weight:600; margin-right:8px;">Hardware</span>
  <span>{HW_PROFILE['cpu']}</span>
  <span>·</span>
  <span>{HW_PROFILE['ram']}</span>
  <span>·</span>
  <span>IPC {HW_PROFILE['ipc']}</span>
  <span>·</span>
  <span>TLB {HW_PROFILE['tlb']}</span>
  <span>·</span>
  <span>L1 {HW_PROFILE['l1']}</span>
  <span>·</span>
  <span>L2 {HW_PROFILE['l2']}</span>
  <span>·</span>
  <span>L3 {HW_PROFILE['l3']}</span>
</div>
"""

# --- Compile the HTML ---
FINAL_HTML = f"""
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Exlex Telemetry | {latest_run}</title>
    <script src="https://cdn.plot.ly/plotly-2.32.0.min.js"></script>
    <style>
        body {{ background-color: #111111; color: #ffffff; font-family: system-ui, sans-serif; margin: 0; padding: 20px; }}
        h1 {{ text-align: center; margin-bottom: 5px; color: #00e5ff; }}
        h3 {{ text-align: center; margin-top: 0; margin-bottom: 20px; color: #888; font-weight: 400; }}
        .graph-container {{ max-width: 1400px; margin: 0 auto 50px auto; background: #1e1e1e; border-radius: 8px; padding: 15px; box-shadow: 0 10px 20px rgba(0,0,0,0.5); }}
    </style>
</head>
<body>
    <h1>Exlex Performance Matrix</h1>
    <h3>Timestamp: {latest_run}</h3>
    {hw_card}
    {"".join([f'<div class="graph-container">{g}</div>' for g in html_graphs])}
</body>
</html>
"""

output_file = f"dashboard_{latest_run}.html"
with open(output_file, "w") as f:
    f.write(FINAL_HTML)

print(f"=> Telemetry Complete! Open {output_file} in your browser.")