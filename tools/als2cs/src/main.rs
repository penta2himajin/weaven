//! als2cs — Generate C# type definitions and validators from Alloy (.als) models.
//!
//! This is a lightweight replacement for `oxidtr generate --target cs`.
//! It parses a subset of Alloy used by Weaven models and emits:
//!   - Models.cs (types, enums, records)
//!   - Validators.cs (invariant checking functions)
//!
//! Usage:
//!   als2cs <input.als> --output <dir>

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 || args[2] != "--output" {
        eprintln!("Usage: als2cs <input.als> --output <dir>");
        std::process::exit(1);
    }
    let input_path = &args[1];
    let output_dir = &args[3];

    let source = fs::read_to_string(input_path).expect("Cannot read input file");
    let model = parse_alloy(&source);
    let (models_cs, validators_cs) = generate_cs(&model, input_path);

    fs::create_dir_all(output_dir).expect("Cannot create output directory");
    fs::write(Path::new(output_dir).join("Models.cs"), models_cs).unwrap();
    fs::write(Path::new(output_dir).join("Validators.cs"), validators_cs).unwrap();

    println!("Generated Models.cs and Validators.cs in {}", output_dir);
}

// ─── Alloy AST ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct AlloyModel {
    sigs: Vec<Sig>,
    facts: Vec<Fact>,
}

#[derive(Debug, Clone)]
struct Sig {
    name: String,
    is_abstract: bool,
    is_one: bool,
    extends: Option<String>,
    fields: Vec<Field>,
}

#[derive(Debug, Clone)]
struct Field {
    name: String,
    multiplicity: Multiplicity,
    ty: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Multiplicity {
    One,
    Lone,
    Set,
    Seq,
}

#[derive(Debug, Clone)]
struct Fact {
    name: String,
    body: String,
}

// ─── Parser ─────────────────────────────────────────────────────────────────

fn parse_alloy(source: &str) -> AlloyModel {
    let mut sigs = Vec::new();
    let mut facts = Vec::new();

    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Skip comments and blank lines
        if line.starts_with("--") || line.is_empty() || line.starts_with("//") {
            i += 1;
            continue;
        }

        // Skip predicates, assertions, checks, runs
        if line.starts_with("pred ")
            || line.starts_with("assert ")
            || line.starts_with("check ")
            || line.starts_with("run ")
        {
            // Skip to end of block
            if line.contains('{') {
                let mut depth = 0;
                while i < lines.len() {
                    for ch in lines[i].chars() {
                        if ch == '{' { depth += 1; }
                        if ch == '}' { depth -= 1; }
                    }
                    i += 1;
                    if depth <= 0 { break; }
                }
            } else {
                i += 1;
            }
            continue;
        }

        // Parse sig
        if line.contains("sig ") && !line.starts_with("fact") {
            let (sig, consumed) = parse_sig(&lines, i);
            sigs.push(sig);
            i += consumed;
            continue;
        }

        // Parse fact
        if line.starts_with("fact ") {
            let (fact, consumed) = parse_fact(&lines, i);
            facts.push(fact);
            i += consumed;
            continue;
        }

        i += 1;
    }

    AlloyModel { sigs, facts }
}

fn parse_sig(lines: &[&str], start: usize) -> (Sig, usize) {
    let line = lines[start].trim();

    // Remove inline comments
    let line = if let Some(idx) = line.find("--") {
        line[..idx].trim()
    } else {
        line
    };

    let is_abstract = line.contains("abstract ");
    let is_one = line.contains("one sig ");

    // Extract name and extends
    let sig_idx = line.find("sig ").unwrap();
    let after_sig = &line[sig_idx + 4..];
    let name_end = after_sig.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(after_sig.len());
    let name = after_sig[..name_end].trim().to_string();

    let extends = if let Some(ext_idx) = after_sig.find("extends ") {
        let ext_name_start = ext_idx + 8;
        let ext_rest = &after_sig[ext_name_start..];
        let ext_name_end = ext_rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(ext_rest.len());
        Some(ext_rest[..ext_name_end].trim().to_string())
    } else {
        None
    };

    let mut fields = Vec::new();
    let mut consumed = 1;

    // Check for fields in braces
    if line.contains('{') && line.ends_with('}') {
        // Single-line sig with no fields: sig Foo {}
        // or inline fields: sig Foo { field: one Bar }
        if let Some(brace_start) = line.find('{') {
            let brace_end = line.rfind('}').unwrap();
            let inner = line[brace_start + 1..brace_end].trim();
            if !inner.is_empty() {
                parse_fields(inner, &mut fields);
            }
        }
    } else if line.contains('{') {
        // Multi-line sig
        let mut depth = 0;
        let mut field_lines: Vec<&str> = Vec::new();
        let mut j = start;
        while j < lines.len() {
            let l = lines[j].trim();
            for ch in l.chars() {
                if ch == '{' { depth += 1; }
                if ch == '}' { depth -= 1; }
            }
            if j > start {
                let cleaned = if let Some(idx) = l.find("--") { l[..idx].trim() } else { l };
                let cleaned = cleaned.trim_end_matches('}').trim_end_matches(',').trim();
                if !cleaned.is_empty() {
                    parse_fields(cleaned, &mut fields);
                }
            }
            j += 1;
            if depth <= 0 { break; }
        }
        consumed = j - start;
    }

    (Sig { name, is_abstract, is_one, extends, fields }, consumed)
}

fn parse_fields(text: &str, fields: &mut Vec<Field>) {
    // Parse "fieldName: multiplicity Type" or comma-separated
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() { continue; }
        if let Some(colon) = part.find(':') {
            let name = part[..colon].trim().to_string();
            let rest = part[colon + 1..].trim();

            let (mult, ty) = if rest.starts_with("one ") {
                (Multiplicity::One, rest[4..].trim())
            } else if rest.starts_with("lone ") {
                (Multiplicity::Lone, rest[5..].trim())
            } else if rest.starts_with("set ") {
                (Multiplicity::Set, rest[4..].trim())
            } else if rest.starts_with("seq ") {
                (Multiplicity::Seq, rest[4..].trim())
            } else {
                (Multiplicity::One, rest)
            };

            fields.push(Field {
                name,
                multiplicity: mult,
                ty: ty.to_string(),
            });
        }
    }
}

fn parse_fact(lines: &[&str], start: usize) -> (Fact, usize) {
    let line = lines[start].trim();
    let name_start = 5; // "fact "
    let name_end = line[name_start..].find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(line.len() - name_start);
    let name = line[name_start..name_start + name_end].trim().to_string();

    let mut body = String::new();
    let mut depth = 0;
    let mut consumed = 0;
    let mut j = start;
    while j < lines.len() {
        let l = lines[j];
        for ch in l.chars() {
            if ch == '{' { depth += 1; }
            if ch == '}' { depth -= 1; }
        }
        body.push_str(l);
        body.push('\n');
        j += 1;
        if depth <= 0 && j > start { break; }
    }
    consumed = j - start;

    (Fact { name, body }, consumed)
}

// ─── C# Code Generator ─────────────────────────────────────────────────────

fn generate_cs(model: &AlloyModel, input_path: &str) -> (String, String) {
    // Build parent→children map for enum detection
    let mut children_of: BTreeMap<String, Vec<&Sig>> = BTreeMap::new();
    for sig in &model.sigs {
        if let Some(ref parent) = sig.extends {
            children_of.entry(parent.clone()).or_default().push(sig);
        }
    }

    // Classify sigs
    let mut enums: Vec<(&Sig, Vec<&Sig>)> = Vec::new();
    let mut id_types: Vec<&Sig> = Vec::new();
    let mut record_types: Vec<&Sig> = Vec::new();
    let mut abstract_unions: Vec<(&Sig, Vec<&Sig>)> = Vec::new();

    for sig in &model.sigs {
        if sig.extends.is_some() { continue; } // handled as child

        let children = children_of.get(&sig.name);

        if sig.is_abstract && children.map_or(false, |c| c.iter().all(|s| s.is_one)) {
            // Enum: abstract sig with all `one sig` children
            enums.push((sig, children.unwrap().clone()));
        } else if sig.is_abstract && children.is_some() {
            // Discriminated union: abstract sig with non-one children
            abstract_unions.push((sig, children.unwrap().clone()));
        } else if sig.fields.is_empty() && !sig.is_abstract {
            // ID type: leaf sig with no fields
            id_types.push(sig);
        } else if !sig.fields.is_empty() {
            // Record: sig with fields
            record_types.push(sig);
        }
        // else: empty non-abstract sig (like WorldSnapshot) → record with no params
    }

    // Also include empty non-abstract sigs without children as empty records
    let mut empty_records: Vec<&Sig> = Vec::new();
    for sig in &model.sigs {
        if sig.extends.is_some() { continue; }
        if sig.fields.is_empty()
            && !sig.is_abstract
            && !children_of.contains_key(&sig.name)
        {
            // Could be ID type or empty record — heuristic:
            // If it has no fields and maps to a primitive (SmId, Tick, etc.), it's an ID type
            // Otherwise it's an empty record
            // We already classified these above in id_types
        }
    }

    // Determine which ID types map to which primitives
    // Filter out sigs that shouldn't be ID types (no semantic ID name pattern)
    let id_types: Vec<&Sig> = id_types.into_iter().filter(|s| {
        let name = s.name.as_str();
        name.ends_with("Id") || matches!(name, "Tick" | "Label" | "EvalValue" | "IRGroupName" | "TableKey")
    }).collect();

    let id_type_map = build_id_type_map(&id_types);

    // ── Generate Models.cs ──────────────────────────────────────────────

    let mut models = String::new();
    models.push_str(&format!(
        "// <auto-generated>\n// Generated from {} by als2cs.\n// DO NOT EDIT — re-run `als2cs` to regenerate.\n// </auto-generated>\n\nusing System;\nusing System.Collections.Generic;\n\nnamespace Weaven.Generated\n{{\n",
        input_path
    ));

    // ID types
    if !id_types.is_empty() {
        models.push_str("    // ── Identity types ──────────────────────────────────────────────────\n\n");
        for sig in &id_types {
            let cs_prim = id_type_map.get(&sig.name as &str).copied().unwrap_or("uint");
            models.push_str(&format!("    public readonly record struct {}({} Value);\n", sig.name, cs_prim));
        }
        models.push('\n');
    }

    // Enums
    if !enums.is_empty() {
        models.push_str("    // ── Enumerations ────────────────────────────────────────────────────\n\n");
        for (parent, children) in &enums {
            models.push_str(&format!("    public enum {}\n    {{\n", parent.name));
            for child in children {
                models.push_str(&format!("        {},\n", child.name));
            }
            models.push_str("    }\n\n");
        }
    }

    // Record types (with fields)
    let fact_names: BTreeSet<&str> = model.facts.iter().map(|f| f.name.as_str()).collect();

    for sig in &record_types {
        // Check for invariants
        let invariants = find_invariants_for_sig(&sig.name, &model.facts);
        for inv in &invariants {
            models.push_str(&format!("    /// <summary>Invariant: {}</summary>\n", inv));
        }

        let params = sig.fields.iter().map(|f| {
            let cs_type = alloy_type_to_cs(&f.ty, &f.multiplicity, &id_type_map);
            let cs_name = pascal_case(&f.name);
            format!("{} {}", cs_type, cs_name)
        }).collect::<Vec<_>>().join(",\n        ");

        models.push_str(&format!("    public sealed record {}(\n        {}\n    );\n\n", sig.name, params));
    }

    // Abstract unions (discriminated union via abstract record + sealed subclasses)
    for (parent, children) in &abstract_unions {
        models.push_str(&format!("    public abstract record {};\n\n", parent.name));
        for child in children {
            if child.fields.is_empty() {
                // No additional fields beyond parent
                models.push_str(&format!("    public sealed record {}(\n", child.name));
                // Include parent fields
                let params = parent.fields.iter().map(|f| {
                    let cs_type = alloy_type_to_cs(&f.ty, &f.multiplicity, &id_type_map);
                    let cs_name = pascal_case(&f.name);
                    format!("{} {}", cs_type, cs_name)
                }).collect::<Vec<_>>().join(",\n        ");
                models.push_str(&format!("        {}\n    ) : {};\n\n", params, parent.name));
            } else {
                // Parent fields + child-specific fields
                let mut all_params = Vec::new();
                for f in &parent.fields {
                    let cs_type = alloy_type_to_cs(&f.ty, &f.multiplicity, &id_type_map);
                    let cs_name = pascal_case(&f.name);
                    all_params.push(format!("{} {}", cs_type, cs_name));
                }
                for f in &child.fields {
                    let cs_type = alloy_type_to_cs(&f.ty, &f.multiplicity, &id_type_map);
                    let cs_name = pascal_case(&f.name);
                    all_params.push(format!("{} {}", cs_type, cs_name));
                }
                let params = all_params.join(",\n        ");
                models.push_str(&format!("    public sealed record {}(\n        {}\n    ) : {};\n\n", child.name, params, parent.name));
            }
        }
    }

    // Empty records (no fields, not ID types, not enums)
    for sig in &model.sigs {
        if sig.extends.is_some() { continue; }
        if sig.is_abstract { continue; }
        if sig.fields.is_empty()
            && !id_type_map.contains_key(sig.name.as_str())
            && !children_of.contains_key(&sig.name)
        {
            models.push_str(&format!("    public sealed record {}();\n\n", sig.name));
        }
    }

    models.push_str("}\n");

    // ── Generate Validators.cs ──────────────────────────────────────────

    let mut validators = String::new();
    validators.push_str(&format!(
        "// <auto-generated>\n// Generated from {} by als2cs.\n// DO NOT EDIT — re-run `als2cs` to regenerate.\n// </auto-generated>\n\nusing System;\nusing System.Collections.Generic;\nusing System.Linq;\nusing Weaven.Generated;\n\nnamespace Weaven.Generated.Validation\n{{\n    public static class Validators\n    {{\n",
        input_path
    ));

    for fact in &model.facts {
        if let Some(validator) = generate_validator(fact, &model.sigs) {
            validators.push_str(&validator);
        }
    }

    validators.push_str("    }\n}\n");

    (models, validators)
}

fn build_id_type_map<'a>(id_types: &[&'a Sig]) -> BTreeMap<&'a str, &'static str> {
    let mut map = BTreeMap::new();
    for sig in id_types {
        let prim = match sig.name.as_str() {
            "SmId" | "StateId" | "TransitionId" | "PortId" | "ConnectionId" | "InteractionRuleId" => "uint",
            "Tick" => "ulong",
            "Label" | "IRGroupName" | "TableKey" => "string",
            "EvalValue" => "double",
            _ => "uint", // default
        };
        map.insert(sig.name.as_str(), prim);
    }
    map
}

fn alloy_type_to_cs(ty: &str, mult: &Multiplicity, id_map: &BTreeMap<&str, &str>) -> String {
    let base = match ty {
        "Int" => "int".to_string(),
        "Bool" => "bool".to_string(),
        _ => ty.to_string(),
    };

    match mult {
        Multiplicity::One => base,
        Multiplicity::Lone => {
            if base == "int" || base == "bool" || base == "double" || base == "uint" || base == "ulong"
                || id_map.values().any(|v| *v == base.as_str())
                || id_map.contains_key(base.as_str())
            {
                format!("{}?", base)
            } else {
                format!("{}?", base)
            }
        }
        Multiplicity::Set => format!("IReadOnlySet<{}>", base),
        Multiplicity::Seq => format!("IReadOnlyList<{}>", base),
    }
}

fn pascal_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}

fn find_invariants_for_sig(sig_name: &str, facts: &[Fact]) -> Vec<String> {
    let mut invariants = Vec::new();
    for fact in facts {
        if fact.name.is_empty() { continue; }
        // Skip phase-constraint facts and non-structural ones
        if fact.name.ends_with("Phase") || fact.name.ends_with("NonNeg") { continue; }

        // Check if the fact is "for all x: SigName" — the primary quantified type
        // Must match ": SigName |" or ": SigName\n" (the quantified variable's type)
        let pattern1 = format!(": {} |", sig_name);
        let pattern2 = format!(": {}\n", sig_name);
        let pattern3 = format!(": {} ", sig_name);
        if fact.body.contains(&pattern1) || fact.body.contains(&pattern2) {
            invariants.push(fact.name.clone());
        } else if fact.body.contains(&pattern3)
            && fact.body.contains(&format!("all"))
            // Avoid matching sub-strings like "GraphNode" in "TopologyGraph"
            && !fact.body.contains(&format!("all g: TopologyGraph"))
        {
            invariants.push(fact.name.clone());
        }
    }
    invariants
}

fn generate_validator(fact: &Fact, sigs: &[Sig]) -> Option<String> {
    let name = &fact.name;
    let body = &fact.body;

    // Map known fact patterns to C# validators
    match name.as_str() {
        "NoSelfLoop" => Some(format!(
            "        /// <summary>Invariant: NoSelfLoop — edge source != edge target.</summary>\n\
             \x20       public static bool ValidateNoSelfLoop(GraphEdge edge)\n\
             \x20           => edge.EdgeSource != edge.EdgeTarget;\n\n"
        )),
        "EdgesReferenceGraphNodes" => Some(format!(
            "        /// <summary>Invariant: EdgesReferenceGraphNodes — every edge endpoint is in the node set.</summary>\n\
             \x20       public static bool ValidateEdgesReferenceGraphNodes(TopologyGraph graph)\n\
             \x20           => graph.Edges.All(e =>\n\
             \x20               graph.Nodes.Contains(e.EdgeSource) &&\n\
             \x20               graph.Nodes.Contains(e.EdgeTarget));\n\n"
        )),
        "UniqueSmPerNode" => Some(format!(
            "        /// <summary>Invariant: UniqueSmPerNode — each SM appears in at most one node.</summary>\n\
             \x20       public static bool ValidateUniqueSmPerNode(TopologyGraph graph)\n\
             \x20       {{\n\
             \x20           var seen = new HashSet<SmId>();\n\
             \x20           foreach (var node in graph.Nodes)\n\
             \x20           {{\n\
             \x20               if (!seen.Add(node.Sm)) return false;\n\
             \x20           }}\n\
             \x20           return true;\n\
             \x20       }}\n\n"
        )),
        "NoCyclicEvalTree" => Some(format!(
            "        /// <summary>Invariant: NoCyclicEvalTree — no cycles in the expression tree.</summary>\n\
             \x20       public static bool ValidateNoCyclicEvalTree(EvalTreeNode root)\n\
             \x20       {{\n\
             \x20           var visited = new HashSet<EvalTreeNode>(ReferenceEqualityComparer.Instance);\n\
             \x20           return CheckNoCycle(root, visited);\n\
             \x20       }}\n\n\
             \x20       private static bool CheckNoCycle(EvalTreeNode node, HashSet<EvalTreeNode> visited)\n\
             \x20       {{\n\
             \x20           if (!visited.Add(node)) return false;\n\
             \x20           foreach (var child in node.Children)\n\
             \x20           {{\n\
             \x20               if (!CheckNoCycle(child, visited)) return false;\n\
             \x20           }}\n\
             \x20           visited.Remove(node);\n\
             \x20           return true;\n\
             \x20       }}\n\n"
        )),
        "CursorRange" => Some(format!(
            "        /// <summary>Invariant: CursorRange — current &lt;= maxTick.</summary>\n\
             \x20       public static bool ValidateCursorRange(TickCursor cursor)\n\
             \x20           => cursor.Current <= cursor.MaxTick;\n\n"
        )),
        "MaxTickNonNeg" => Some(format!(
            "        /// <summary>Invariant: MaxTickNonNeg — maxTick &gt;= 0.</summary>\n\
             \x20       public static bool ValidateMaxTickNonNeg(TickCursor cursor)\n\
             \x20           => cursor.MaxTick >= 0;\n\n"
        )),
        "SnapshotNonEmpty" => Some(format!(
            "        /// <summary>Invariant: SnapshotNonEmpty — at least one snapshot.</summary>\n\
             \x20       public static bool ValidateSnapshotNonEmpty(DebugSession session)\n\
             \x20           => session.Snapshots.Count > 0;\n\n"
        )),
        _ => None, // Skip non-structural facts (phase constraints, etc.)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_sig() {
        let src = "sig SmId {}";
        let model = parse_alloy(src);
        assert_eq!(model.sigs.len(), 1);
        assert_eq!(model.sigs[0].name, "SmId");
        assert!(!model.sigs[0].is_abstract);
        assert!(model.sigs[0].fields.is_empty());
    }

    #[test]
    fn test_parse_abstract_sig() {
        let src = "abstract sig Phase {}";
        let model = parse_alloy(src);
        assert_eq!(model.sigs.len(), 1);
        assert!(model.sigs[0].is_abstract);
    }

    #[test]
    fn test_parse_one_sig_extends() {
        let src = "one sig PhaseInput extends Phase {}";
        let model = parse_alloy(src);
        assert_eq!(model.sigs.len(), 1);
        assert!(model.sigs[0].is_one);
        assert_eq!(model.sigs[0].extends.as_deref(), Some("Phase"));
    }

    #[test]
    fn test_parse_sig_with_fields() {
        let src = "sig GraphNode {\n  sm: one SmId,\n  activeState: lone StateId\n}";
        let model = parse_alloy(src);
        assert_eq!(model.sigs.len(), 1);
        assert_eq!(model.sigs[0].fields.len(), 2);
        assert_eq!(model.sigs[0].fields[0].name, "sm");
        assert_eq!(model.sigs[0].fields[0].multiplicity, Multiplicity::One);
        assert_eq!(model.sigs[0].fields[1].name, "activeState");
        assert_eq!(model.sigs[0].fields[1].multiplicity, Multiplicity::Lone);
    }

    #[test]
    fn test_parse_fact() {
        let src = "fact NoSelfLoop {\n  no e: GraphEdge | e.edgeSource = e.edgeTarget\n}";
        let model = parse_alloy(src);
        assert_eq!(model.facts.len(), 1);
        assert_eq!(model.facts[0].name, "NoSelfLoop");
    }

    #[test]
    fn test_pascal_case() {
        assert_eq!(pascal_case("smId"), "SmId");
        assert_eq!(pascal_case("activeState"), "ActiveState");
        assert_eq!(pascal_case("edge_source"), "EdgeSource");
    }

    #[test]
    fn test_alloy_type_to_cs() {
        let map = BTreeMap::new();
        assert_eq!(alloy_type_to_cs("Int", &Multiplicity::One, &map), "int");
        assert_eq!(alloy_type_to_cs("Bool", &Multiplicity::One, &map), "bool");
        assert_eq!(alloy_type_to_cs("SmId", &Multiplicity::Lone, &map), "SmId?");
        assert_eq!(alloy_type_to_cs("GraphNode", &Multiplicity::Set, &map), "IReadOnlySet<GraphNode>");
        assert_eq!(alloy_type_to_cs("TraceEvent", &Multiplicity::Seq, &map), "IReadOnlyList<TraceEvent>");
    }

    #[test]
    fn test_generate_from_debugger_model() {
        let source = std::fs::read_to_string("../../models/weaven-debugger.als")
            .expect("Cannot read weaven-debugger.als");
        let model = parse_alloy(&source);

        // Should find key sigs
        let sig_names: Vec<&str> = model.sigs.iter().map(|s| s.name.as_str()).collect();
        assert!(sig_names.contains(&"SmId"));
        assert!(sig_names.contains(&"Phase"));
        assert!(sig_names.contains(&"TraceEvent"));
        assert!(sig_names.contains(&"GraphEdge"));
        assert!(sig_names.contains(&"DebugSession"));

        // Should find key facts
        let fact_names: Vec<&str> = model.facts.iter().map(|f| f.name.as_str()).collect();
        assert!(fact_names.contains(&"NoSelfLoop"));
        assert!(fact_names.contains(&"SnapshotNonEmpty"));

        // Generate and verify output is non-empty
        let (models_cs, validators_cs) = generate_cs(&model, "models/weaven-debugger.als");
        assert!(models_cs.contains("public readonly record struct SmId"));
        assert!(models_cs.contains("public enum Phase"));
        assert!(models_cs.contains("public abstract record TraceEvent"));
        assert!(models_cs.contains("public sealed record GraphEdge"));
        assert!(validators_cs.contains("ValidateNoSelfLoop"));
        assert!(validators_cs.contains("ValidateSnapshotNonEmpty"));
    }
}
