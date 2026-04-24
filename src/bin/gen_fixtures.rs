use std::fs;
use std::path::Path;

/// In-memory representation of a configuration section to guarantee
/// structural equivalence across all format outputs.
struct ConfigNode {
    name: String,
    props: Vec<(String, String)>,
    children: Vec<ConfigNode>,
}

impl ConfigNode {
    /// Generates a Wide structure: 1 root node with N children, each having very few props.
    /// Tests directory/child lookup scalability.
    fn generate_wide(children_count: usize, props_per_child: usize, prefix: &str) -> ConfigNode {
        let mut children = Vec::with_capacity(children_count);
        for i in 0..children_count {
            let name = format!("{}_child_{}", prefix, i);
            let mut props = Vec::with_capacity(props_per_child);
            for p in 0..props_per_child {
                props.push((format!("key_{}_{}", name, p), format!("val_{}", p)));
            }
            children.push(ConfigNode {
                name,
                props,
                children: Vec::new(),
            });
        }

        ConfigNode {
            name: format!("{}_root", prefix),
            props: vec![("root_key".to_string(), "root_val".to_string())],
            children,
        }
    }

    /// Generates a Lopsided structure: One massive section, alongside many empty/small sections.
    /// Tests hash array contiguous memory performance and worst-case linear scanning.
    fn generate_lopsided(
        heavy_props: usize,
        empty_sections: usize,
        prefix: &str,
    ) -> Vec<ConfigNode> {
        let mut nodes = Vec::with_capacity(empty_sections + 1);

        // The heavy node
        let mut heavy_props_vec = Vec::with_capacity(heavy_props);
        for p in 0..heavy_props {
            heavy_props_vec.push((format!("heavy_key_{}", p), "heavy_value".to_string()));
        }
        nodes.push(ConfigNode {
            name: format!("{}_heavy", prefix),
            props: heavy_props_vec,
            children: Vec::new(),
        });

        // The empty nodes
        for i in 0..empty_sections {
            nodes.push(ConfigNode {
                name: format!("{}_empty_{}", prefix, i),
                props: Vec::new(),
                children: Vec::new(),
            });
        }
        nodes
    }
    /// Recursively builds the matrix topologies
    fn generate(
        depth_left: usize,
        breadth: usize,
        props_count: usize,
        prefix: &str,
        level: usize,
    ) -> Vec<ConfigNode> {
        if depth_left == 0 {
            return Vec::new();
        }

        let mut nodes = Vec::with_capacity(breadth);
        for i in 0..breadth {
            let name = format!("{}_L{}_N{}", prefix, level, i);

            let mut props = Vec::with_capacity(props_count);
            for p in 0..props_count {
                props.push((
                    format!("key_{}_{}", name, p),
                    format!("value_{}_{}_{}", name, p, "x".repeat(10)), // 10-byte payload
                ));
            }

            nodes.push(ConfigNode {
                name: name.clone(),
                props,
                children: Self::generate(depth_left - 1, breadth, props_count, prefix, level + 1),
            });
        }
        nodes
    }
    fn generate_exact_scale(prop_count: usize) -> ConfigNode {
        let mut props = Vec::with_capacity(prop_count);
        for p in 0..prop_count {
            props.push((format!("key_{}", p), format!("val_{}", p)));
        }
        ConfigNode {
            name: format!("scale_{}", prop_count),
            props,
            children: Vec::new(),
        }
    }
    // --- Format Serializers ---

    fn to_exlex(&self, buffer: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        buffer.push_str(&format!("{}sect \"{}\" {{\n", pad, self.name));
        for (k, v) in &self.props {
            buffer.push_str(&format!("{}  \"{}\": \"{}\"\n", pad, k, v));
        }
        for child in &self.children {
            child.to_exlex(buffer, indent + 1);
        }
        buffer.push_str(&format!("{}}}\n", pad));
    }

    fn to_json(&self, buffer: &mut String, indent: usize, is_last: bool) {
        let pad = "  ".repeat(indent);
        buffer.push_str(&format!("{}\"{}\": {{\n", pad, self.name));

        let total_items = self.props.len() + self.children.len();
        let mut current = 0;

        for (k, v) in &self.props {
            current += 1;
            let comma = if current < total_items { "," } else { "" };
            buffer.push_str(&format!("{}  \"{}\": \"{}\"{}\n", pad, k, v, comma));
        }

        for (i, child) in self.children.iter().enumerate() {
            current += 1;
            let comma = if current < total_items { "," } else { "" };
            child.to_json(
                buffer,
                indent + 1,
                i == self.children.len() - 1 && current == total_items,
            );
            if current < total_items {
                buffer.push_str(",\n");
            } else {
                buffer.push_str("\n");
            }
        }
        buffer.push_str(&format!("{}}}", pad));
    }

    fn to_toml(&self, buffer: &mut String, parent_path: &str) {
        let current_path = if parent_path.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", parent_path, self.name)
        };

        buffer.push_str(&format!("[{}]\n", current_path));
        for (k, v) in &self.props {
            buffer.push_str(&format!("{} = \"{}\"\n", k, v));
        }
        buffer.push_str("\n");

        for child in &self.children {
            child.to_toml(buffer, &current_path);
        }
    }

    fn to_ini(&self, buffer: &mut String, parent_path: &str) {
        // INI lacks real nesting, standard practice is dot-separated section names
        let current_path = if parent_path.is_empty() {
            self.name.clone()
        } else {
            format!("{}.{}", parent_path, self.name)
        };

        buffer.push_str(&format!("[{}]\n", current_path));
        for (k, v) in &self.props {
            buffer.push_str(&format!("{}={}\n", k, v));
        }
        buffer.push_str("\n");

        for child in &self.children {
            child.to_ini(buffer, &current_path);
        }
    }
    fn to_xml(&self, buffer: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        buffer.push_str(&format!("{}<{}>\n", pad, self.name));
        for (k, v) in &self.props {
            // Valid XML keys can't easily start with numbers or have weird chars,
            // but our generated keys (e.g. "key_...") are safe.
            buffer.push_str(&format!("{}  <{}>{}</{}>\n", pad, k, v, k));
        }
        for child in &self.children {
            child.to_xml(buffer, indent + 1);
        }
        buffer.push_str(&format!("{}</{}>\n", pad, self.name));
    }
}

fn write_matrix(name: &str, roots: &[ConfigNode]) {
    println!("Generating matrix: {}", name);

    let mut exlex_buf = String::new();
    let mut json_buf = String::from("{\n");
    let mut toml_buf = String::new();
    let mut ini_buf = String::new();

    for (i, root) in roots.iter().enumerate() {
        root.to_exlex(&mut exlex_buf, 0);
        exlex_buf.push('\n');
        let is_last = i == roots.len() - 1;
        root.to_json(&mut json_buf, 1, is_last);
        if !is_last {
            json_buf.push_str(",\n");
        } else {
            json_buf.push_str("\n");
        }

        root.to_toml(&mut toml_buf, "");
        root.to_ini(&mut ini_buf, "");
    }
    json_buf.push_str("}\n");

    let base_path = format!("fixtures/{}", name);
    fs::write(format!("{}.exlex", base_path), exlex_buf).unwrap();
    fs::write(format!("{}.json", base_path), json_buf).unwrap();
    fs::write(format!("{}.toml", base_path), toml_buf).unwrap();
    fs::write(format!("{}.ini", base_path), ini_buf).unwrap();
    let mut xml_buf = String::from("<config>\n");
    for root in roots.iter() {
        root.to_xml(&mut xml_buf, 1);
    }
    xml_buf.push_str("</config>\n");
    std::fs::write(format!("{}.xml", base_path), xml_buf).unwrap();
}

fn main() {
    println!("=> Matrix Fixture Generator Initialized.");
    let _ = std::fs::create_dir_all(std::path::Path::new("fixtures"));

    // --- BASE TOPOLOGIES ---
    let flat_sparse = ConfigNode::generate(1, 10, 2, "fs", 0);
    let flat_dense = ConfigNode::generate(1, 5, 100, "fd", 0);
    let deep_sparse = ConfigNode::generate(8, 1, 2, "ds", 0);
    let deep_dense = ConfigNode::generate(6, 1, 50, "dd", 0);
    let wide_sparse = vec![ConfigNode::generate_wide(100, 1, "ws")];
    let bushy_normal = ConfigNode::generate(3, 5, 10, "bn", 0);
    let lopsided_dense = ConfigNode::generate_lopsided(200, 50, "ld");

    // --- LOOKUP BENCHMARK FIXTURES ---
    write_matrix("flat_sparse_first", &flat_sparse);
    write_matrix("flat_dense_last", &flat_dense);
    write_matrix("flat_dense_missing", &flat_dense);

    // Note: The Collision fixture requires artificially injected keys that yield the same 64-bit
    // FxHash output. A 64-bit brute forcer (birthday paradox) takes ~4 billion ops.
    // This is handled dynamically in benches/common/collision_util.rs during bench execution
    // to avoid stalling this generator script for 30 seconds.
    write_matrix("flat_dense_collision", &flat_dense);

    write_matrix("deep_sparse_path6", &deep_sparse);
    write_matrix("deep_dense_pathdeep", &deep_dense);
    write_matrix("wide_sparse_random", &wide_sparse);
    write_matrix("bushy_normal_middle", &bushy_normal);
    write_matrix("bushy_normal_sequential", &bushy_normal);
    write_matrix("lopsided_dense_last", &lopsided_dense);
    write_matrix("lopsided_dense_missing", &lopsided_dense);

    // --- MUTATION BENCHMARK FIXTURES ---
    // Isolating mutation targets prevents OS/filesystem cache anomalies when
    // Criterion repeatedly saves output strings.
    write_matrix("mut_update_existing", &bushy_normal);
    write_matrix("mut_update_new", &bushy_normal);
    write_matrix("mut_update_twice", &flat_dense);
    write_matrix("mut_delete_then_access", &flat_sparse);
    write_matrix("mut_add_section", &bushy_normal);
    write_matrix("mut_add_nested_section", &deep_sparse);
    write_matrix("mut_delete_section", &bushy_normal);
    write_matrix("mut_move_section", &bushy_normal);
    write_matrix("mut_mass_update_save", &lopsided_dense);
    write_matrix("mut_roundtrip_verify", &bushy_normal);

    // --- ASYMPTOTIC SCALING FIXTURES ---
    // Generate config files scaling from 5 to 100 properties to find the HashMap crossover
    for count in (5..=100).step_by(5) {
        let node = ConfigNode::generate_exact_scale(count);
        // This will create files like "scale_005.exlex", "scale_010.exlex", etc.
        let name = format!("scale_{:03}", count);
        write_matrix(&name, &[node]);
    }
    println!("=> Generation complete. 21 dimensions mapped to fixtures/");
}
