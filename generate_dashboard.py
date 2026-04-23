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

# 1. Find the absolute latest benchmark run safely
if not os.path.exists(RESULTS_DIR):
    print(f"ERROR: {RESULTS_DIR} not found. Run ./run_matrix.sh first.")
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

# 2. Extract and Normalize Data
db = {}

for root, dirs, files in os.walk(backup_dir):
    if "estimates.json" in files:
        with open(os.path.join(root, "estimates.json")) as f:
            data = json.load(f)
            point_est = data["mean"]["point_estimate"]  # In nanoseconds

        # Calculate relative path to determine Group, Scenario, and Parser
        rel_path = os.path.relpath(root, backup_dir)
        parts = rel_path.split(os.sep)

        group = parts[0]

        # Format A: Parse_Init / exlex / flat_sparse / new
        if len(parts) == 4:
            raw_parser = parts[1]
            scenario = parts[2]
        # Format B: Lookup_Flat / Exlex_Dense_Last / new
        elif len(parts) == 3:
            func_name = parts[1]
            if "_" in func_name:
                raw_parser, scenario = func_name.split("_", 1)
            else:
                raw_parser = func_name
                scenario = "Default"
        else:
            continue

        # Clean up parser names for the legend
        raw_p = raw_parser.lower()
        if "serde" in raw_p:
            parser = "Serde JSON"
        elif "exlex" in raw_p:
            parser = "Exlex (DOD)"
        elif "toml_edit" in raw_p or "tomledit" in raw_p:
            parser = "TOML Edit"
        elif "toml" in raw_p:
            parser = "TOML"
        elif "ini" in raw_p:
            parser = "INI"
        elif "sonic" in raw_p:
            parser = "Sonic-RS (SIMD)"
        elif "simd" in raw_p:
            parser = "SIMD-JSON"
        elif "quick" in raw_p:
            parser = "Quick-XML"
        elif "figment" in raw_p:
            parser = "Figment (Layer)"
        else:
            parser = raw_parser.capitalize()

        if group not in db:
            db[group] = {}
        if scenario not in db[group]:
            db[group][scenario] = {}
        db[group][scenario][parser] = point_est

# 3. Dedicated Color Palette (Exlex stands out)
COLOR_MAP = {
    "Exlex (DOD)": "#00e5ff",  # Neon Cyan
    "Serde JSON": "#ff3d00",  # Deep Orange
    "Sonic-RS (SIMD)": "#00e676",  # Neon Green
    "SIMD-JSON": "#1de9b6",  # Teal
    "TOML": "#ffea00",  # Yellow
    "TOML Edit": "#ffb300",  # Amber
    "INI": "#d50000",  # Red
    "Quick-XML": "#aa00ff",  # Purple
    "Figment (Layer)": "#3d5afe",  # Indigo
}

# 4. Generate the Plotly HTML blocks
html_graphs = []

# Sort groups alphabetically so the dashboard is ordered
for group in sorted(db.keys()):
    scenarios = db[group]

    # Identify all parsers that competed in this specific group
    parsers_in_group = set()
    for s in scenarios.values():
        parsers_in_group.update(s.keys())

    fig = go.Figure()

    # Ensure Exlex is plotted first, then sort the rest alphabetically
    sorted_parsers = sorted(list(parsers_in_group))
    if "Exlex (DOD)" in sorted_parsers:
        sorted_parsers.remove("Exlex (DOD)")
        sorted_parsers.insert(0, "Exlex (DOD)")

    for p in sorted_parsers:
        x_vals = sorted(list(scenarios.keys()))
        y_vals = [
            scenarios[s].get(p, 0) for s in x_vals
        ]  # 0 if that parser skipped that test

        fig.add_trace(
            go.Bar(name=p, x=x_vals, y=y_vals, marker_color=COLOR_MAP.get(p, "#ffffff"))
        )

    fig.update_layout(
        title=f"Benchmark Group: {group.replace('_', ' ')}",
        barmode="group",
        template="plotly_dark",
        yaxis_title="Time (Nanoseconds) ↓ Lower is Better",
        xaxis_title="Topology / Scenario",
        font=dict(family="system-ui", size=14),
        margin=dict(t=60, b=40, l=40, r=40),
        legend=dict(orientation="h", yanchor="bottom", y=1.02, xanchor="right", x=1),
    )

    # include_plotlyjs=False because we will inject the CDN script once at the top of the HTML
    html_graphs.append(fig.to_html(full_html=False, include_plotlyjs=False))

# 5. Compile the All-In-One HTML
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
        h3 {{ text-align: center; margin-top: 0; margin-bottom: 40px; color: #888; font-weight: 400; }}
        .graph-container {{ max-width: 1400px; margin: 0 auto 50px auto; background: #1e1e1e; border-radius: 8px; padding: 15px; box-shadow: 0 10px 20px rgba(0,0,0,0.5); }}
    </style>
</head>
<body>
    <h1>Exlex Performance Matrix</h1>
    <h3>Timestamp: {latest_run}</h3>
    {"".join([f'<div class="graph-container">{g}</div>' for g in html_graphs])}
</body>
</html>
"""

output_file = f"dashboard_{latest_run}.html"
with open(output_file, "w") as f:
    f.write(FINAL_HTML)

print(f"=> Telemetry Complete! Open {output_file} in your browser.")
