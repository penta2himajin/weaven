//! Golden file generator — produces JSON fixtures from real Rust serde output.
//!
//! These fixtures are consumed by TS tests to verify that normalizers
//! handle the actual Rust serialization format correctly.
//!
//! Run: cargo test -p weaven-debugger-core --test golden_fixtures -- --ignored
//!
//! Output: weaven-debugger-core/tests/fixtures/*.json

use std::collections::BTreeMap;
use weaven_core::*;
use weaven_debugger_core::debug_session::DebugSession;
use weaven_debugger_core::topology::build_topology;

const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

fn write_fixture(name: &str, value: &impl serde::Serialize) {
    let json = serde_json::to_string_pretty(value).unwrap();
    let path = format!("{}/{}", FIXTURE_DIR, name);
    std::fs::create_dir_all(FIXTURE_DIR).unwrap();
    std::fs::write(&path, &json).unwrap();
    eprintln!("Wrote {}", path);
}

// --- Fire propagation scenario ---

const STATE_GRASS: StateId = StateId(0);
const STATE_BURNING: StateId = StateId(1);
const PORT_IN: PortId = PortId(0);
const PORT_OUT: PortId = PortId(1);
const SIG_FIRE: SignalTypeId = SignalTypeId(0);

fn tile_sm(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [STATE_GRASS, STATE_BURNING].into_iter().collect(),
        initial_state: STATE_GRASS,
        transitions: vec![
            Transition {
                id: TransitionId(id.0 * 10),
                source: STATE_GRASS,
                target: STATE_BURNING,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("intensity") > 0.0)),
                guard_expr: None,
                effects: vec![
                    Box::new(|ctx| {
                        let v = ctx.get("intensity");
                        if v <= 0.0 { return vec![]; }
                        let mut p = BTreeMap::new();
                        p.insert("intensity".into(), v);
                        vec![EffectOutput::Signal(PORT_OUT, Signal { signal_type: SIG_FIRE, payload: p })]
                    }),
                ],
            },
        ],
        input_ports: vec![Port::new(PORT_IN, PortKind::Input, SIG_FIRE)],
        output_ports: vec![Port::new(PORT_OUT, PortKind::Output, SIG_FIRE)],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn fire_world() -> World {
    let mut world = World::new();
    for i in 1..=3 {
        world.register_sm(tile_sm(SmId(i)));
    }
    world.connections.push(Connection {
        id: ConnectionId(1), source_sm: SmId(1), source_port: PORT_OUT,
        target_sm: SmId(2), target_port: PORT_IN, delay_ticks: 0,
        pipeline: vec![
            PipelineStep::Transform(Box::new(|mut sig| {
                let v = sig.payload.get("intensity").copied().unwrap_or(0.0);
                sig.payload.insert("intensity".into(), v - 1.0);
                sig
            })),
            PipelineStep::Filter(Box::new(|sig| {
                sig.payload.get("intensity").copied().unwrap_or(0.0) > 0.0
            })),
        ],
    });
    world.connections.push(Connection {
        id: ConnectionId(2), source_sm: SmId(2), source_port: PORT_OUT,
        target_sm: SmId(3), target_port: PORT_IN, delay_ticks: 0,
        pipeline: vec![
            PipelineStep::Transform(Box::new(|mut sig| {
                let v = sig.payload.get("intensity").copied().unwrap_or(0.0);
                sig.payload.insert("intensity".into(), v - 1.0);
                sig
            })),
            PipelineStep::Filter(Box::new(|sig| {
                sig.payload.get("intensity").copied().unwrap_or(0.0) > 0.0
            })),
        ],
    });

    if let Some(inst) = world.instances.get_mut(&SmId(1)) {
        inst.context.set("intensity", 3.0);
    }
    world.inject_signal(SmId(1), PORT_IN, Signal {
        signal_type: SIG_FIRE,
        payload: { let mut p = BTreeMap::new(); p.insert("intensity".into(), 3.0); p },
    });
    world
}

#[test]
#[ignore] // Run explicitly: cargo test -p weaven-debugger-core --test golden_fixtures -- --ignored
fn generate_fire_tick_result() {
    let mut session = DebugSession::new(fire_world());
    let result = session.tick();
    write_fixture("fire_tick_result.json", &result);
}

#[test]
#[ignore]
fn generate_fire_topology() {
    let world = fire_world();
    let topo = build_topology(&world);
    write_fixture("fire_topology.json", &topo);
}

// --- Parry scenario ---

fn parry_world() -> World {
    let mut world = World::new();
    let enemy = SmId(10);
    let pc = SmId(20);

    world.register_sm(SmDef {
        id: enemy,
        states: [StateId(0), StateId(1), StateId(2)].into_iter().collect(),
        initial_state: StateId(0),
        transitions: vec![
            Transition { id: TransitionId(100), source: StateId(0), target: StateId(1),
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("timer") > 0.0)),
                guard_expr: None,
                effects: vec![] },
            Transition { id: TransitionId(101), source: StateId(1), target: StateId(2),
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("stagger") > 0.0)),
                guard_expr: None,
                effects: vec![] },
        ],
        input_ports: vec![Port::new(PortId(10), PortKind::Input, SignalTypeId(1))],
        output_ports: vec![],
        on_despawn_transitions: vec![], elapse_capability: ElapseCapabilityRt::NonElapsable, elapse_fn: None,
    });

    world.register_sm(SmDef {
        id: pc,
        states: [StateId(10), StateId(11), StateId(12)].into_iter().collect(),
        initial_state: StateId(10),
        transitions: vec![
            Transition { id: TransitionId(200), source: StateId(10), target: StateId(11),
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("parry") > 0.0)),
                guard_expr: None,
                effects: vec![] },
        ],
        input_ports: vec![Port::new(PortId(11), PortKind::Input, SignalTypeId(1))],
        output_ports: vec![],
        on_despawn_transitions: vec![], elapse_capability: ElapseCapabilityRt::NonElapsable, elapse_fn: None,
    });

    // Activate both with context
    if let Some(i) = world.instances.get_mut(&enemy) { i.context.set("timer", 1.0); }
    if let Some(i) = world.instances.get_mut(&pc) { i.context.set("parry", 1.0); }
    world.activate(enemy);
    world.activate(pc);

    world
}

#[test]
#[ignore]
fn generate_parry_tick_result() {
    let mut session = DebugSession::new(parry_world());
    let result = session.tick();
    write_fixture("parry_tick_result.json", &result);
}

#[test]
#[ignore]
fn generate_parry_topology() {
    let world = parry_world();
    let topo = build_topology(&world);
    write_fixture("parry_topology.json", &topo);
}
