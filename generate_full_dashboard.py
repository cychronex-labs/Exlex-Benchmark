import json
import os
import sys
from datetime import datetime

try:
    import plotly.graph_objects as go
except ImportError:
    print("ERROR: Plotly is missing. Run: pip install plotly")
    sys.exit(1)

RESULTS_DIR = "results"

if not os.path.exists(RESULTS_DIR):
    print(f"ERROR: {RESULTS_DIR} not found.")
    sys.exit(1)

runs = sorted([d for d in os.listdir(RESULTS_DIR) if os.path.isdir(os.path.join(RESULTS_DIR, d))])
if not runs:
    print("ERROR: No benchmark data found.")
    sys.exit(1)

print(f"=> Analyzing {len(runs)} historical runs...")

# Data structure: db[group][scenario][parser][run_timestamp] = time_in_ns
history_db = {}
latest_db = {}
latest_run = runs[-1]

def parse_run_to_date(run_str):
    # Converts "2026-04-23_18-26-05" to a readable format or datetime object
    try:
        dt = datetime.strptime(run_str, "%Y-%m-%d_%H-%M-%S")
        return dt.strftime("%b %d, %H:%M")
    except:
        return run_str

# 1. Extract Data Across ALL Runs
for run in runs:
    backup_dir = os.path.join(RESULTS_DIR, run, "raw_criterion_backup")
    if not os.path.exists(backup_dir):
        continue
        
    run_label = parse_run_to_date(run)

    for root, dirs, files in os.walk(backup_dir):
        if "estimates.json" in files:
            with open(os.path.join(root, "estimates.json")) as f:
                data = json.load(f)
                point_est = data["mean"]["point_estimate"]

            rel_path = os.path.relpath(root, backup_dir)
            parts = rel_path.split(os.sep)

            # Standard flat parsing logic based on your run_benchmark.sh
            if len(parts) < 3:
                continue
                
            group = parts[0]
            
            if len(parts) == 4:
                raw_parser = parts[1]
                scenario = parts[2]
            elif len(parts) == 3:
                func_name = parts[1]
                if "_" in func_name:
                    raw_parser, scenario = func_name.split("_", 1)
                else:
                    raw_parser = func_name
                    scenario = "Default"
            else:
                continue

            raw_p = raw_parser.lower()
            if "serde" in raw_p: parser = "Serde JSON"
            elif "exlex" in raw_p: parser = "Exlex (DOD)"
            elif "toml_edit" in raw_p or "tomledit" in raw_p: parser = "TOML Edit"
            elif "toml" in raw_p: parser = "TOML"
            elif "ini" in raw_p: parser = "INI"
            elif "sonic" in raw_p: parser = "Sonic-RS (SIMD)"
            elif "simd" in raw_p: parser = "SIMD-JSON"
            elif "quick" in raw_p: parser = "Quick-XML"
            elif "figment" in raw_p: parser = "Figment (Layer)"
            else: parser = raw_parser.capitalize()

            # Populate Historical DB
            if group not in history_db: history_db[group] = {}
            if scenario not in history_db[group]: history_db[group][scenario] = {}
            if parser not in history_db[group][scenario]: history_db[group][scenario][parser] = {}
            
            history_db[group][scenario][parser][run_label] = point_est
            
            # Populate Latest DB for the bar charts
            if run == latest_run:
                if group not in latest_db: latest_db[group] = {}
                if scenario not in latest_db[group]: latest_db[group][scenario] = {}
                latest_db[group][scenario][parser] = point_est

# 2. Colors (Okabe-Ito)
COLOR_MAP = {
    "Exlex (DOD)": "#00e5ff", "Serde JSON": "#E69F00", "Sonic-RS (SIMD)": "#009E73",
    "SIMD-JSON": "#56B4E9", "TOML": "#999999", "TOML Edit": "#F0E442",
    "INI": "#D55E00", "Quick-XML": "#CC79A7", "Figment (Layer)": "#0072B2",
}

html_graphs = []

# --- GENERATE HISTORICAL LINE CHARTS ---
html_graphs.append("<h2 style='color:#00e5ff; border-bottom: 1px solid #333; padding-bottom: 10px;'>Part 1: Historical Regression Tracking</h2>")

for group in sorted(history_db.keys()):
    scenarios = history_db[group]
    
    # We will plot one line chart per group, averaging the scenarios, OR plot a specific important scenario.
    # To keep it clean, let's plot a line chart for each Scenario within the group if it has Exlex.
    for scenario, parsers in scenarios.items():
        if "Exlex (DOD)" not in parsers:
            continue # Only track history where Exlex is involved
            
        fig = go.Figure()
        
        # Plot Exlex and Industry Standards over time
        for p in ["Exlex (DOD)", "Serde JSON", "TOML Edit"]:
            if p in parsers:
                x_vals = list(parsers[p].keys()) # Timestamps
                y_vals = list(parsers[p].values()) # Nanoseconds
                
                fig.add_trace(go.Scatter(
                    x=x_vals, y=y_vals, 
                    mode='lines+markers',
                    name=p,
                    line=dict(width=3, color=COLOR_MAP.get(p, "#ffffff")),
                    marker=dict(size=8)
                ))

        fig.update_layout(
            title=f"Trend: {group.replace('_', ' ')} -> {scenario}",
            template="plotly_dark",
            yaxis_title="Time (ns) ↓ Lower is Better",
            xaxis_title="Benchmark Run (Time)",
            font=dict(family="system-ui", size=14),
            margin=dict(t=60, b=40, l=40, r=40),
            legend=dict(orientation="h", yanchor="bottom", y=1.02, xanchor="right", x=1),
        )
        html_graphs.append('<div class="graph-container">' + fig.to_html(full_html=False, include_plotlyjs=False) + '</div>')

# --- GENERATE LATEST BAR CHARTS ---
html_graphs.append("<h2 style='color:#00e5ff; border-bottom: 1px solid #333; padding-bottom: 10px; margin-top: 50px;'>Part 2: Latest Snapshot Matrix</h2>")

for group in sorted(latest_db.keys()):
    scenarios = latest_db[group]
    parsers_in_group = set()
    for s in scenarios.values(): parsers_in_group.update(s.keys())

    fig = go.Figure()
    sorted_parsers = sorted(list(parsers_in_group))
    if "Exlex (DOD)" in sorted_parsers:
        sorted_parsers.remove("Exlex (DOD)")
        sorted_parsers.insert(0, "Exlex (DOD)")

    for p in sorted_parsers:
        x_vals = sorted(list(scenarios.keys()))
        y_vals = [scenarios[s].get(p, 0) for s in x_vals]
        fig.add_trace(go.Bar(name=p, x=x_vals, y=y_vals, marker_color=COLOR_MAP.get(p, "#ffffff")))

    fig.update_layout(
        title=f"Latest: {group.replace('_', ' ')}",
        barmode="group",
        template="plotly_dark",
        yaxis_title="Time (ns) ↓ Lower is Better",
        xaxis_title="Topology / Scenario",
        font=dict(family="system-ui", size=14),
        margin=dict(t=60, b=40, l=40, r=40),
        legend=dict(orientation="h", yanchor="bottom", y=1.02, xanchor="right", x=1),
    )
    html_graphs.append('<div class="graph-container">' + fig.to_html(full_html=False, include_plotlyjs=False) + '</div>')

# 3. Compile HTML
FINAL_HTML = f"""
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Exlex Endgame Telemetry</title>
    <script src="https://cdn.plot.ly/plotly-2.32.0.min.js"></script>
    <style>
        body {{ background-color: #111111; color: #ffffff; font-family: system-ui, sans-serif; margin: 0; padding: 20px; }}
        h1 {{ text-align: center; margin-bottom: 5px; color: #00e5ff; font-weight: 800; text-transform: uppercase; letter-spacing: 2px; }}
        h3 {{ text-align: center; margin-top: 0; margin-bottom: 40px; color: #888; font-weight: 400; }}
        .graph-container {{ max-width: 1400px; margin: 0 auto 30px auto; background: #1e1e1e; border-radius: 8px; padding: 15px; box-shadow: 0 10px 20px rgba(0,0,0,0.5); }}
    </style>
</head>
<body>
    <h1>Exlex Historical Regression Telemetry</h1>
    <h3>Comprehensive Historical Analysis up to {parse_run_to_date(latest_run)}</h3>
    {"".join(html_graphs)}
</body>
</html>
"""

output_file = "HistoricalRegression.html"
with open(output_file, "w") as f:
    f.write(FINAL_HTML)

print(f"=> Endgame Telemetry Complete! Open {output_file}")