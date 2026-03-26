/// Weaven Schema (§12.3) tests — JSON load + compile + execute.

use weaven_core::*;
use weaven_core::schema::{load_schema, compile_schema};

// ── Basic SM round-trip ───────────────────────────────────────────────────

#[test]
fn test_schema_load_and_compile_simple_sm() {
    let json = r#"{
        "state_machines": [{
            "id": 1,
            "states": [0, 1],
            "initial_state": 0,
            "transitions": [{
                "id": 10,
                "source": 0,
                "target": 1,
                "priority": 10,
                "guard": { "BinOp": {
                    "op": "Gt",
                    "left": { "CtxField": "hp" },
                    "right": { "Num": 0.0 }
                }},
                "effects": []
            }],
            "input_ports": [],
            "output_ports": []
        }]
    }"#;

    let schema = load_schema(json).expect("schema should parse");
    let compiled = compile_schema(&schema);
    assert_eq!(compiled.sm_defs.len(), 1);

    let def = &compiled.sm_defs[0];
    assert_eq!(def.id, SmId(1));
    assert_eq!(def.states.len(), 2);
    assert_eq!(def.transitions.len(), 1);
}

/// Compiled guard fires correctly based on context value.
#[test]
fn test_schema_guard_fires_on_context() {
    let json = r#"{
        "state_machines": [{
            "id": 1,
            "states": [0, 1],
            "initial_state": 0,
            "transitions": [{
                "id": 10, "source": 0, "target": 1, "priority": 10,
                "guard": { "BinOp": {
                    "op": "Gt",
                    "left": { "CtxField": "hp" },
                    "right": { "Num": 0.0 }
                }},
                "effects": []
            }],
            "input_ports": [], "output_ports": []
        }]
    }"#;

    let schema = load_schema(json).unwrap();
    let compiled = compile_schema(&schema);

    let mut world = World::new();
    world.register_sm(compiled.sm_defs.into_iter().next().unwrap());

    // hp = 0 → guard false → no transition
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(0), "hp=0 guard false");

    // hp = 50 → guard true → S1
    if let Some(i) = world.instances.get_mut(&SmId(1)) { i.context.set("hp", 50.0); }
    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(1), "hp=50 guard true");
}

/// Schema with SetContext effect mutates context.
#[test]
fn test_schema_set_context_effect() {
    let json = r#"{
        "state_machines": [{
            "id": 1,
            "states": [0, 1],
            "initial_state": 0,
            "transitions": [{
                "id": 10, "source": 0, "target": 1, "priority": 10,
                "guard": { "BinOp": {
                    "op": "Gt",
                    "left": { "CtxField": "trigger" },
                    "right": { "Num": 0.0 }
                }},
                "effects": [{ "SetContext": {
                    "field": "damage_dealt",
                    "expr": { "BinOp": {
                        "op": "Mul",
                        "left": { "CtxField": "attack" },
                        "right": { "Num": 1.5 }
                    }}
                }}]
            }],
            "input_ports": [], "output_ports": []
        }]
    }"#;

    let schema = load_schema(json).unwrap();
    let compiled = compile_schema(&schema);

    let mut world = World::new();
    world.register_sm(compiled.sm_defs.into_iter().next().unwrap());

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("trigger", 1.0);
        i.context.set("attack", 10.0);
    }
    world.activate(SmId(1));
    tick(&mut world);

    assert_eq!(world.instances[&SmId(1)].active_state, StateId(1));
    assert_eq!(world.instances[&SmId(1)].context.get("damage_dealt"), 15.0,
        "10 * 1.5 = 15");
}

/// Schema with Connection between two SMs.
#[test]
fn test_schema_connection_routes_signal() {
    let json = r#"{
        "state_machines": [
            {
                "id": 1, "states": [0, 1], "initial_state": 0,
                "transitions": [{
                    "id": 10, "source": 0, "target": 1, "priority": 10,
                    "guard": { "BinOp": {
                        "op": "Gt", "left": {"CtxField": "fire"}, "right": {"Num": 0.0}
                    }},
                    "effects": [{ "Signal": {
                        "port": 1,
                        "payload": { "intensity": {"Num": 5.0} }
                    }}]
                }],
                "input_ports": [],
                "output_ports": [{ "id": 1, "kind": "Output", "signal_type": 0 }]
            },
            {
                "id": 2, "states": [0, 1], "initial_state": 0,
                "transitions": [{
                    "id": 20, "source": 0, "target": 1, "priority": 10,
                    "guard": { "BinOp": {
                        "op": "Gt", "left": {"CtxField": "intensity"}, "right": {"Num": 0.0}
                    }},
                    "effects": []
                }],
                "input_ports": [{ "id": 0, "kind": "Input", "signal_type": 0 }],
                "output_ports": []
            }
        ],
        "connections": [{
            "id": 1, "source_sm": 1, "source_port": 1,
            "target_sm": 2, "target_port": 0, "delay_ticks": 0
        }]
    }"#;

    let schema = load_schema(json).unwrap();
    let compiled = compile_schema(&schema);

    let mut world = World::new();
    for def in compiled.sm_defs { world.register_sm(def); }
    for conn in compiled.connections { world.connect(conn); }

    if let Some(i) = world.instances.get_mut(&SmId(1)) { i.context.set("fire", 1.0); }
    world.activate(SmId(1));
    tick(&mut world);

    assert_eq!(world.instances[&SmId(1)].active_state, StateId(1), "SM1 fired");
    assert_eq!(world.instances[&SmId(2)].active_state, StateId(1), "SM2 received signal");
}

/// Schema with Named Table and table lookup in guard.
#[test]
fn test_schema_named_table_lookup() {
    let json = r#"{
        "state_machines": [{
            "id": 1, "states": [0, 1], "initial_state": 0,
            "transitions": [{
                "id": 10, "source": 0, "target": 1, "priority": 10,
                "guard": { "BinOp": {
                    "op": "Gt",
                    "left": { "TableLookup": {
                        "table": "elementDamage",
                        "keys": [{"Str": "Fire"}]
                    }},
                    "right": { "Num": 1.0 }
                }},
                "effects": []
            }],
            "input_ports": [], "output_ports": []
        }],
        "named_tables": [{
            "name": "elementDamage",
            "entries": { "Fire": 1.5, "Water": 0.5 }
        }]
    }"#;

    let schema = load_schema(json).unwrap();
    let compiled = compile_schema(&schema);

    let mut world = World::new();
    world.tables = compiled.table_registry;
    world.register_sm(compiled.sm_defs.into_iter().next().unwrap());

    world.activate(SmId(1));
    // Guard: table.elementDamage["Fire"] = 1.5 > 1.0 → true → transition fires
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(1),
        "table lookup 1.5 > 1.0 → guard true");
}

/// Schema with Connection-side pipeline (Transform).
#[test]
fn test_schema_connection_pipeline_transform() {
    let json = r#"{
        "state_machines": [
            {
                "id": 1, "states": [0, 1], "initial_state": 0,
                "transitions": [{
                    "id": 10, "source": 0, "target": 1, "priority": 10,
                    "guard": {"BinOp": {"op": "Gt", "left": {"CtxField": "fire"}, "right": {"Num": 0.0}}},
                    "effects": [{"Signal": {"port": 1,
                        "payload": {"intensity": {"Num": 10.0}}
                    }}]
                }],
                "input_ports": [],
                "output_ports": [{"id": 1, "kind": "Output", "signal_type": 0}]
            },
            {
                "id": 2, "states": [0, 1], "initial_state": 0,
                "transitions": [{
                    "id": 20, "source": 0, "target": 1, "priority": 10,
                    "guard": {"BinOp": {"op": "Gt", "left": {"CtxField": "intensity"}, "right": {"Num": 0.0}}},
                    "effects": []
                }],
                "input_ports": [{"id": 0, "kind": "Input", "signal_type": 0}],
                "output_ports": []
            }
        ],
        "connections": [{
            "id": 1, "source_sm": 1, "source_port": 1,
            "target_sm": 2, "target_port": 0, "delay_ticks": 0,
            "pipeline": [
                {"Transform": {"intensity": {"BinOp": {
                    "op": "Mul",
                    "left": {"SigField": "intensity"},
                    "right": {"Num": 0.5}
                }}}}
            ]
        }]
    }"#;

    let schema = load_schema(json).unwrap();
    let compiled = compile_schema(&schema);

    let mut world = World::new();
    for def in compiled.sm_defs { world.register_sm(def); }
    for conn in compiled.connections { world.connect(conn); }

    if let Some(i) = world.instances.get_mut(&SmId(1)) { i.context.set("fire", 1.0); }
    world.activate(SmId(1));
    tick(&mut world);

    assert_eq!(world.instances[&SmId(2)].active_state, StateId(1), "SM2 fired");
    assert_eq!(world.instances[&SmId(2)].context.get("intensity"), 5.0,
        "10 * 0.5 = 5 after pipeline transform");
}

/// Schema with HitStop effect.
#[test]
fn test_schema_hitstop_effect() {
    let json = r#"{
        "state_machines": [{
            "id": 1, "states": [0, 1], "initial_state": 0,
            "transitions": [{
                "id": 10, "source": 0, "target": 1, "priority": 10,
                "guard": {"BinOp": {"op": "Gt", "left": {"CtxField": "hit"}, "right": {"Num": 0.0}}},
                "effects": [{"HitStop": {"frames": 5}}]
            }],
            "input_ports": [], "output_ports": []
        }]
    }"#;

    let schema = load_schema(json).unwrap();
    let compiled = compile_schema(&schema);

    let mut world = World::new();
    world.register_sm(compiled.sm_defs.into_iter().next().unwrap());

    if let Some(i) = world.instances.get_mut(&SmId(1)) { i.context.set("hit", 1.0); }
    world.activate(SmId(1));
    let out = tick(&mut world);

    assert_eq!(world.instances[&SmId(1)].active_state, StateId(1));
    assert!(world.hit_stop_frames > 0, "HitStop applied");
    assert!(out.system_commands.iter().any(|c| matches!(c, SystemCommand::HitStop { .. })));
}

/// Schema: CollectionCount guard — fire transition when active buff count > 2.
#[test]
fn test_schema_collection_count_guard() {
    let json = r#"{
        "state_machines": [{
            "id": 1,
            "states": [0, 1],
            "initial_state": 0,
            "transitions": [{
                "id": 10, "source": 0, "target": 1, "priority": 10,
                "guard": { "BinOp": {
                    "op": "Gt",
                    "left": { "CollectionCount": {
                        "array_field": "buffs",
                        "predicate": { "BinOp": {
                            "op": "Gt",
                            "left": { "SigField": "active" },
                            "right": { "Num": 0.0 }
                        }}
                    }},
                    "right": { "Num": 2.0 }
                }},
                "effects": []
            }],
            "input_ports": [], "output_ports": []
        }]
    }"#;

    let schema = load_schema(json).unwrap();
    let compiled = compile_schema(&schema);

    let mut world = World::new();
    world.register_sm(compiled.sm_defs.into_iter().next().unwrap());

    // Set up 3 active buffs
    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set_array("buffs", vec![
            [("active", 1.0)].iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            [("active", 1.0)].iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            [("active", 1.0)].iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        ]);
    }

    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(1),
        "3 active buffs > 2 → guard fires");
}

/// Schema: CollectionAny guard — fire if any enemy has hp <= 0.
#[test]
fn test_schema_collection_any_guard() {
    let json = r#"{
        "state_machines": [{
            "id": 1,
            "states": [0, 1],
            "initial_state": 0,
            "transitions": [{
                "id": 10, "source": 0, "target": 1, "priority": 10,
                "guard": { "CollectionAny": {
                    "array_field": "enemies",
                    "predicate": { "BinOp": {
                        "op": "Lte",
                        "left": { "SigField": "hp" },
                        "right": { "Num": 0.0 }
                    }}
                }},
                "effects": []
            }],
            "input_ports": [], "output_ports": []
        }]
    }"#;

    let schema = load_schema(json).unwrap();
    let compiled = compile_schema(&schema);

    let mut world = World::new();
    world.register_sm(compiled.sm_defs.into_iter().next().unwrap());

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set_array("enemies", vec![
            [("hp", 50.0)].iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            [("hp", 0.0)].iter().map(|(k, v)| (k.to_string(), *v)).collect(), // dead
        ]);
    }

    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].active_state, StateId(1),
        "any enemy hp <= 0 → guard fires");
}

/// Schema: CollectionSum effect — write total damage from projectiles to context.
#[test]
fn test_schema_collection_sum_effect() {
    let json = r#"{
        "state_machines": [{
            "id": 1,
            "states": [0, 1],
            "initial_state": 0,
            "transitions": [{
                "id": 10, "source": 0, "target": 1, "priority": 10,
                "guard": { "BinOp": {
                    "op": "Gt",
                    "left": { "CtxField": "trigger" },
                    "right": { "Num": 0.0 }
                }},
                "effects": [{ "SetContext": {
                    "field": "total_damage",
                    "expr": { "CollectionSum": {
                        "array_field": "hits",
                        "sum_field": "dmg"
                    }}
                }}]
            }],
            "input_ports": [], "output_ports": []
        }]
    }"#;

    let schema = load_schema(json).unwrap();
    let compiled = compile_schema(&schema);

    let mut world = World::new();
    world.register_sm(compiled.sm_defs.into_iter().next().unwrap());

    if let Some(i) = world.instances.get_mut(&SmId(1)) {
        i.context.set("trigger", 1.0);
        i.context.set_array("hits", vec![
            [("dmg", 10.0)].iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            [("dmg", 15.0)].iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            [("dmg", 5.0)].iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        ]);
    }

    world.activate(SmId(1));
    tick(&mut world);
    assert_eq!(world.instances[&SmId(1)].context.get("total_damage"), 30.0,
        "10+15+5=30 total damage");
}
