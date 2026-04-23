use std::fs;
use std::path::Path;

/// Loads a fixture into a String.
/// CRITICAL: Call this OUTSIDE the `b.iter(|| ...)` block.
pub fn load_fixture(name: &str, ext: &str) -> String {
    let path = format!("fixtures/{}.{}", name, ext);
    fs::read_to_string(Path::new(&path)).unwrap_or_else(|_| {
        panic!(
            "CRITICAL: Failed to load fixture: {}. Run gen_fixtures first.",
            path
        )
    })
}
