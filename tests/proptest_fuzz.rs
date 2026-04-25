// tests/proptest_fuzz.rs
use exlex::{Exlex, ExlexArena};
use proptest::prelude::*;
use std::collections::HashMap;

// ============================================================================
// SHADOW AST - A bulletproof reference implementation of your Config Model
// ============================================================================

/// Represents a validated, strictly correct configuration structure.
/// We use HashMaps to implicitly enforce unique keys/sections, which
/// mirrors how a user mathematically expects unique property storage to work.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfigAST {
    properties: HashMap<String, String>,
    sections: HashMap<String, ConfigAST>,
}

impl Default for ConfigAST {
    fn default() -> Self {
        Self {
            properties: HashMap::new(),
            sections: HashMap::new(),
        }
    }
}

impl ConfigAST {
    /// Serializes the Shadow AST into your `exlex` custom syntax format.
    fn to_exlex_string(&self, indent: usize) -> String {
        let mut out = String::new();
        let pad = " ".repeat(indent);

        for (k, v) in &self.properties {
            out.push_str(&format!("{}\"{}\": \"{}\"\n", pad, k, v));
        }

        for (name, child) in &self.sections {
            out.push_str(&format!("{}sect \"{}\" {{\n", pad, name));
            out.push_str(&child.to_exlex_string(indent + 4));
            out.push_str(&format!("{}}}\n", pad));
        }

        out
    }

    /// Recursively walks the parsed Exlex Data-Oriented structure and asserts
    /// that every single value matches our generated Shadow AST exactly.
    fn verify(&self, parsed: &Exlex, sect_id: exlex::ExlexSection) {
        for (k, v) in &self.properties {
            let parsed_val = parsed
                .get_property(k, sect_id)
                .unwrap_or_else(|_| panic!("Failed to find generated property '{}'", k));
            assert_eq!(parsed_val, v.as_str(), "Value mismatch for key '{}'", k);
        }

        for (name, child) in &self.sections {
            let child_id = parsed
                .get_child(name, sect_id)
                .unwrap_or_else(|_| panic!("Failed to find generated section '{}'", name));
            child.verify(parsed, child_id);
        }
    }
}

// ============================================================================
// STRATEGIES - The RNG Chaos Engines
// ============================================================================

/// Generates valid Exlex identifiers (keys, values, section names).
/// Omitting double quotes `"` and newlines `\n` to prevent breaking the grammar.
fn valid_exlex_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_\\- ]+".prop_filter("Must not be empty", |s| !s.is_empty())
}

/// Recursively generates a deeply nested AST configuration
fn config_ast_strategy(depth: u32, max_breadth: usize) -> impl Strategy<Value = ConfigAST> {
    let props =
        proptest::collection::hash_map(valid_exlex_string(), valid_exlex_string(), 0..max_breadth);

    if depth > 0 {
        let sects = proptest::collection::hash_map(
            valid_exlex_string(),
            config_ast_strategy(depth - 1, max_breadth),
            0..3, // Up to 3 sub-sections per level
        );
        (props, sects)
            .prop_map(|(properties, sections)| ConfigAST {
                properties,
                sections,
            })
            .boxed()
    } else {
        (props, Just(HashMap::new()))
            .prop_map(|(properties, sections)| ConfigAST {
                properties,
                sections,
            })
            .boxed()
    }
}

/// The operations that the Arena Mutator can perform
#[derive(Debug, Clone)]
enum MutOp {
    UpsertProperty(String, String),
    DeleteProperty(String),
    AddSection(String),
}

fn mut_ops_strategy() -> impl Strategy<Value = Vec<MutOp>> {
    proptest::collection::vec(
        prop_oneof![
            (valid_exlex_string(), valid_exlex_string())
                .prop_map(|(k, v)| MutOp::UpsertProperty(k, v)),
            valid_exlex_string().prop_map(MutOp::DeleteProperty),
            valid_exlex_string().prop_map(MutOp::AddSection),
        ],
        0..30, // Execute up to 30 chaotic mutations per test
    )
}

// ============================================================================
// THE TEST SUITE
// ============================================================================

proptest! {
    // ----------------------------------------------------------------------
    // 1. PURE FUZZING: Garbage Collection & Panic Assurance
    // ----------------------------------------------------------------------
    // Throws completely random bytes and characters at the zero-copy engine.
    // It is expected to return `Err(ExlexError)`, but it MUST NEVER PANIC.
    // This stress-tests out-of-bounds slice indexing and memchr safety.
    #[test]
    fn fuzz_parser_does_not_panic(garbage in "\\PC*") {
        let _ = Exlex::init_reader(&garbage, None, None, None, None);
    }

    // ----------------------------------------------------------------------
    // 2. PARSER ROUNDTRIP: Structural Integrity Check
    // ----------------------------------------------------------------------
    // Generates a massive, deeply nested syntax tree, serializes it to a string,
    // parses it using Exlex, and then mathematically proves that every single
    // property and nested section was successfully preserved in the SOA vectors.
    #[test]
    fn proptest_parser_structural_roundtrip(ast in config_ast_strategy(3, 10)) {
        let config_str = ast.to_exlex_string(0);

        // Parse the generated string
        match Exlex::init_reader(&config_str, None, None, None, None) {
            Ok(parsed) => {
                // Traverse the SOA and verify 100% exact match against AST
                ast.verify(&parsed, parsed.get_root());
            }
            Err(e) => {
                panic!("Valid generated syntax failed to parse! Error: {:?}\nConfig:\n{}", e, config_str);
            }
        }
    }

    // ----------------------------------------------------------------------
    // 3. MUTATOR PIPELINE ROUNDTRIP: Arena Update Verification
    // ----------------------------------------------------------------------
    // This is the absolute hardest test. It creates an initial config,
    // initializes your `ExlexMutator`, randomly inserts, updates, and deletes
    // keys/sections, saves it to a buffer, re-parses it entirely, and checks
    // if the new parse perfectly matches a shadow-mutated Rust HashMap.
    #[test]
    fn proptest_mutator_engine(
        ast in config_ast_strategy(1, 5),
        mutations in mut_ops_strategy()
    ) {
        let initial_config_str = ast.to_exlex_string(0);
        let parsed = Exlex::init_reader(&initial_config_str, None, None, None, None).unwrap();

        let mut shadow_ast = ast.clone();

        let mut arena = ExlexArena(String::with_capacity(1024));
        let mut buffer = String::with_capacity(1024);

        let mut mutator = parsed.init_mutator(&mut arena, &mut buffer).unwrap();
        let root_id = parsed.get_root();

        // Apply chaos to both the Mutator and the Shadow AST
        for op in &mutations {
            match op {
                MutOp::UpsertProperty(k, v) => {
                    // Update Shadow
                    shadow_ast.properties.insert(k.clone(), v.clone());
                    // Update Exlex
                    mutator.update_prop(k, v, root_id);
                }
                MutOp::DeleteProperty(k) => {
                    // Attempt delete on Exlex (Ignore error if it wasn't there)
                    if mutator.delete_property(k, root_id).is_ok() {
                        shadow_ast.properties.remove(k);
                    } else {
                        // If Exlex says it wasn't there, ensure Shadow agrees
                        assert!(!shadow_ast.properties.contains_key(k));
                    }
                }
                MutOp::AddSection(name) => {
                    // To bypass the &'a str lifetime restriction of your `new_section`
                    // we leak the string in testing to make it outlive the `parsed` object.
                    // This perfectly simulates a user passing a static or scoped string.
                    let leaked_name: &'static str = Box::leak(name.clone().into_boxed_str());

                    if mutator.new_section(leaked_name, root_id).is_ok() {
                        if !shadow_ast.sections.contains_key(name) {
                            shadow_ast.sections.insert(name.clone(), ConfigAST::default());
                        }
                    }
                }
            }
        }

        // Finalize the write buffer
        mutator.save();
        let new_config_str = buffer.clone();

        // Reparse the newly outputted buffer!
        match Exlex::init_reader(&new_config_str, None, None, None, None) {
            Ok(new_parsed) => {
                shadow_ast.verify(&new_parsed, new_parsed.get_root());
            }
            Err(e) => {
                panic!("Mutator outputted invalid syntax! \
                       Error: {:?}\n\
                       Initial:\n{}\n\
                       Output:\n{}", e, initial_config_str, new_config_str);
            }
        }
    }
}
