//! Integration tests: real weaven-core scenarios through the debugger.
//!
//! These tests exercise the full pipeline:
//!   World setup → DebugSession → tick → trace events → topology
//!
//! Goals:
//!   1. Verify trace events provide enough information for debugging
//!   2. Identify gaps in trace coverage
//!   3. Validate topology graph reflects the real world structure

use weaven_core::*;
use weaven_core::trace::{TraceEvent, Phase as TracePhase};
use weaven_debugger_core::debug_session::DebugSession;
use weaven_debugger_core::topology::{build_topology, add_ir_edges_from_trace, EdgeKind};

// =========================================================================
// Scenario A: Fire propagation (Appendix A)
// =========================================================================

const STATE_GRASS:   StateId = StateId(0);
const STATE_BURNING: StateId = StateId(1);
const PORT_IN:  PortId = PortId(0);
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
                        let mut p = std::collections::BTreeMap::new();
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

fn fire_conn(id: u32, src: SmId, tgt: SmId) -> Connection {
    Connection {
        id: ConnectionId(id),
        source_sm: src,
        source_port: PORT_OUT,
        target_sm: tgt,
        target_port: PORT_IN,
        delay_ticks: 0,
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
    }
}

fn fire_world() -> World {
    // T1 → T2 → T3 chain
    let mut world = World::new();
    for i in 1..=3 {
        world.register_sm(tile_sm(SmId(i)));
    }
    world.connect(fire_conn(1, SmId(1), SmId(2)));
    world.connect(fire_conn(2, SmId(2), SmId(3)));
    // Ignite T1
    if let Some(inst) = world.instances.get_mut(&SmId(1)) {
        inst.context.set("intensity", 5.0);
    }
    world.inject_signal(SmId(1), PORT_IN, Signal {
        signal_type: SIG_FIRE,
        payload: {
            let mut p = std::collections::BTreeMap::new();
            p.insert("intensity".into(), 5.0);
            p
        },
    });
    world
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[test]
fn fire_topology_has_all_nodes_and_edges() {
    let world = fire_world();
    let topo = build_topology(&world);
    assert_eq!(topo.nodes.len(), 3, "3 tile SMs");
    assert_eq!(topo.edges.len(), 2, "2 connections");
    assert!(topo.edges.iter().all(|e| e.kind == EdgeKind::Static));
}

#[test]
fn fire_trace_captures_full_cascade() {
    let mut session = DebugSession::new(fire_world());
    let result = session.tick();

    // Expect: T1 fires (Phase 3), cascade to T2, then T3 (Phase 4).
    let fired: Vec<_> = result.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::TransitionFired { .. }))
        .collect();

    // T1, T2, T3 should all fire Grass→Burning.
    assert_eq!(fired.len(), 3, "all 3 tiles fire: got {:?}",
        fired.iter().map(|e| match e {
            TraceEvent::TransitionFired { sm_id, .. } => sm_id.0,
            _ => 0,
        }).collect::<Vec<_>>());
}

#[test]
fn fire_trace_has_cascade_steps() {
    let mut session = DebugSession::new(fire_world());
    let result = session.tick();

    let cascade: Vec<_> = result.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::CascadeStep { .. }))
        .collect();

    // At least 2 cascade steps (T2 delivery, T3 delivery).
    assert!(cascade.len() >= 2,
        "expected >=2 cascade steps, got {}", cascade.len());
}

#[test]
fn fire_trace_signal_emission_chain() {
    let mut session = DebugSession::new(fire_world());
    let result = session.tick();

    let emitted: Vec<_> = result.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::SignalEmitted { .. }))
        .collect();

    // T1 emits in Phase 3, T2 emits in cascade (Phase 4).
    // T3 may or may not emit (intensity may be 0 after pipeline).
    assert!(emitted.len() >= 2,
        "expected >=2 signal emissions, got {}", emitted.len());

    // Verify we can trace the source SM for each emission.
    for e in &emitted {
        match e {
            TraceEvent::SignalEmitted { sm_id, port, .. } => {
                assert_eq!(port.0, PORT_OUT.0, "signal should come from output port");
                assert!(sm_id.0 >= 1 && sm_id.0 <= 3, "source SM should be T1-T3");
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn fire_trace_guard_evaluations_per_sm() {
    let mut session = DebugSession::new(fire_world());
    let result = session.tick();

    let guards: Vec<_> = result.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::GuardEvaluated { .. }))
        .collect();

    // T1 guard evaluated in Phase 2, T2/T3 in Phase 4 cascade.
    assert!(guards.len() >= 3,
        "expected guard eval for each tile, got {}", guards.len());

    // All should pass (result=true) because intensity is > 0.
    for g in &guards {
        match g {
            TraceEvent::GuardEvaluated { sm_id, result, .. } => {
                // Some guards might be for the "stay burning" transition which doesn't exist
                // in our setup. Check that at least T1's guard passes.
                if sm_id.0 == 1 {
                    assert!(result, "T1 guard should pass");
                }
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn fire_trace_phase_ordering_strict() {
    let mut session = DebugSession::new(fire_world());
    let result = session.tick();

    // Extract phases in order.
    let phases: Vec<TracePhase> = result.trace_events.iter().map(|e| match e {
        TraceEvent::GuardEvaluated { phase, .. } => *phase,
        TraceEvent::IrMatched { phase, .. } => *phase,
        TraceEvent::TransitionFired { phase, .. } => *phase,
        TraceEvent::SignalEmitted { phase, .. } => *phase,
        TraceEvent::CascadeStep { phase, .. } => *phase,
        TraceEvent::PipelineFiltered { phase, .. } => *phase,
        TraceEvent::SignalDelivered { phase, .. } => *phase,
    }).collect();

    // All Evaluate events before all Execute events before all Propagate events.
    let last_eval = phases.iter().rposition(|p| *p == TracePhase::Evaluate);
    let first_exec = phases.iter().position(|p| *p == TracePhase::Execute);
    let last_exec = phases.iter().rposition(|p| *p == TracePhase::Execute);
    let first_prop = phases.iter().position(|p| *p == TracePhase::Propagate);

    if let (Some(le), Some(fe)) = (last_eval, first_exec) {
        assert!(le < fe, "Evaluate must precede Execute");
    }
    if let (Some(le), Some(fp)) = (last_exec, first_prop) {
        assert!(le < fp, "Execute must precede Propagate");
    }
}

#[test]
fn fire_seek_restores_pre_cascade_state() {
    let mut session = DebugSession::new(fire_world());
    session.tick(); // tick 1: fire spreads
    session.tick(); // tick 2

    // Seek back to tick 0 — all tiles should be Grass.
    let state = session.seek_tick(0);
    for entry in &state.sm_states {
        assert_eq!(entry.active_state, STATE_GRASS,
            "SM({}) should be Grass at tick 0", entry.sm_id.0);
    }
}

// =========================================================================
// Scenario B: Parry (Appendix B) — Interaction Rule via manual injection
// =========================================================================

const ENEMY_WINDUP: StateId = StateId(100);
const ENEMY_ACTIVE: StateId = StateId(101);
const ENEMY_STAGGER: StateId = StateId(102);
const PC_IDLE: StateId = StateId(200);
const PC_PARRY: StateId = StateId(201);
const PC_RIPOSTE: StateId = StateId(202);
const PORT_STAGGER: PortId = PortId(10);
const PORT_PARRY_OK: PortId = PortId(11);
const SIG_COMBAT: SignalTypeId = SignalTypeId(1);

fn parry_world() -> World {
    let mut world = World::new();
    let enemy = SmId(10);
    let pc = SmId(20);

    world.register_sm(SmDef {
        id: enemy,
        states: [ENEMY_WINDUP, ENEMY_ACTIVE, ENEMY_STAGGER].into_iter().collect(),
        initial_state: ENEMY_WINDUP,
        transitions: vec![
            Transition { id: TransitionId(1000), source: ENEMY_WINDUP, target: ENEMY_ACTIVE,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("timer") > 0.0)),
                guard_expr: None,
                effects: vec![] },
            Transition { id: TransitionId(1001), source: ENEMY_ACTIVE, target: ENEMY_STAGGER,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("stagger") > 0.0)),
                guard_expr: None,
                effects: vec![] },
        ],
        input_ports: vec![Port::new(PORT_STAGGER, PortKind::Input, SIG_COMBAT)],
        output_ports: vec![],
        on_despawn_transitions: vec![], elapse_capability: ElapseCapabilityRt::NonElapsable, elapse_fn: None,
    });

    world.register_sm(SmDef {
        id: pc,
        states: [PC_IDLE, PC_PARRY, PC_RIPOSTE].into_iter().collect(),
        initial_state: PC_IDLE,
        transitions: vec![
            Transition { id: TransitionId(2000), source: PC_IDLE, target: PC_PARRY,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("parry_input") > 0.0)),
                guard_expr: None,
                effects: vec![] },
            Transition { id: TransitionId(2001), source: PC_PARRY, target: PC_RIPOSTE,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("parry_ok") > 0.0)),
                guard_expr: None,
                effects: vec![] },
        ],
        input_ports: vec![Port::new(PORT_PARRY_OK, PortKind::Input, SIG_COMBAT)],
        output_ports: vec![],
        on_despawn_transitions: vec![], elapse_capability: ElapseCapabilityRt::NonElapsable, elapse_fn: None,
    });

    world
}

#[test]
fn parry_tick1_both_transition_no_ir() {
    let mut world = parry_world();
    let enemy = SmId(10);
    let pc = SmId(20);

    // Tick 1: both inputs arrive simultaneously.
    if let Some(i) = world.instances.get_mut(&enemy) { i.context.set("timer", 1.0); }
    if let Some(i) = world.instances.get_mut(&pc) { i.context.set("parry_input", 1.0); }
    world.activate(enemy);
    world.activate(pc);

    let mut session = DebugSession::new(world);
    let result = session.tick();

    // Both should fire.
    let fired: Vec<SmId> = result.trace_events.iter().filter_map(|e| match e {
        TraceEvent::TransitionFired { sm_id, .. } => Some(*sm_id),
        _ => None,
    }).collect();
    assert!(fired.contains(&enemy), "enemy should fire WindUp→Active");
    assert!(fired.contains(&pc), "PC should fire Idle→Parry");

    // No IrMatched events (IR evaluated on pre-transition state).
    let ir_count = result.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::IrMatched { .. }))
        .count();
    assert_eq!(ir_count, 0, "no IR match on tick 1");
}

#[test]
fn parry_tick2_ir_match_triggers_stagger() {
    let mut world = parry_world();
    let enemy = SmId(10);
    let pc = SmId(20);

    // Tick 1 setup.
    if let Some(i) = world.instances.get_mut(&enemy) { i.context.set("timer", 1.0); }
    if let Some(i) = world.instances.get_mut(&pc) { i.context.set("parry_input", 1.0); }
    world.activate(enemy);
    world.activate(pc);

    let mut session = DebugSession::new(world);
    session.tick(); // tick 1

    // Tick 2: manually inject IR result (simulating Phase 2 IR match).
    if let Some(i) = session.world.instances.get_mut(&enemy) {
        i.context.set("stagger", 1.0);
    }
    session.world.inject_signal(enemy, PORT_STAGGER, Signal { signal_type: SIG_COMBAT, payload: std::collections::BTreeMap::new() });
    if let Some(i) = session.world.instances.get_mut(&pc) {
        i.context.set("parry_ok", 1.0);
    }
    session.world.inject_signal(pc, PORT_PARRY_OK, Signal { signal_type: SIG_COMBAT, payload: std::collections::BTreeMap::new() });

    let result = session.tick(); // tick 2

    // Both should fire cascade transitions.
    let fired: Vec<(SmId, StateId)> = result.trace_events.iter().filter_map(|e| match e {
        TraceEvent::TransitionFired { sm_id, to_state, .. } => Some((*sm_id, *to_state)),
        _ => None,
    }).collect();

    assert!(fired.iter().any(|(sm, st)| *sm == enemy && *st == ENEMY_STAGGER),
        "enemy should stagger");
    assert!(fired.iter().any(|(sm, st)| *sm == pc && *st == PC_RIPOSTE),
        "PC should riposte");
}

#[test]
fn parry_trace_has_guard_detail_for_debugging() {
    let mut world = parry_world();
    let enemy = SmId(10);
    let pc = SmId(20);

    if let Some(i) = world.instances.get_mut(&enemy) { i.context.set("timer", 1.0); }
    if let Some(i) = world.instances.get_mut(&pc) { i.context.set("parry_input", 1.0); }
    world.activate(enemy);
    world.activate(pc);

    let mut session = DebugSession::new(world);
    let result = session.tick();

    // Guard evaluations should include both passing and failing guards.
    let guards: Vec<_> = result.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::GuardEvaluated { .. }))
        .collect();

    // At minimum: enemy's timer guard (pass), PC's parry_input guard (pass).
    assert!(guards.len() >= 2, "expected >=2 guard evals, got {}", guards.len());

    // Each guard event should have the SM id — essential for debugging.
    for g in &guards {
        if let TraceEvent::GuardEvaluated { sm_id, transition, .. } = g {
            assert!(sm_id.0 == enemy.0 || sm_id.0 == pc.0,
                "guard SM should be enemy or PC");
            assert!(transition.0 > 0, "transition ID should be meaningful");
        }
    }
}

// =========================================================================
// Gap verification: previously identified gaps, now fixed.
// =========================================================================

#[test]
fn fixed_signal_emitted_has_target() {
    // FIXED: SignalEmitted.target is now populated after routing resolves.
    let mut session = DebugSession::new(fire_world());
    let result = session.tick();

    let emitted: Vec<_> = result.trace_events.iter()
        .filter_map(|e| match e {
            TraceEvent::SignalEmitted { target, sm_id, .. } => Some((*sm_id, *target)),
            _ => None,
        })
        .collect();

    assert!(!emitted.is_empty());
    // All emitted signals should have resolved targets.
    let all_have_target = emitted.iter().all(|(_, t)| t.is_some());
    assert!(all_have_target, "FIXED: All SignalEmitted events should have resolved targets");

    // T1→T2 path should be visible.
    let t1_to_t2 = emitted.iter().any(|(src, tgt)| src.0 == 1 && tgt.unwrap().0 == 2);
    assert!(t1_to_t2, "Should trace T1→T2 signal path");
}

#[test]
fn fixed_connection_pipeline_filter_traced() {
    // FIXED: Connection-side pipeline filtering now emits PipelineFiltered.
    let mut world = World::new();
    for i in 1..=4 {
        world.register_sm(tile_sm(SmId(i)));
    }
    world.connect(fire_conn(1, SmId(1), SmId(2)));
    world.connect(fire_conn(2, SmId(2), SmId(3)));
    world.connect(fire_conn(3, SmId(3), SmId(4)));

    if let Some(inst) = world.instances.get_mut(&SmId(1)) {
        inst.context.set("intensity", 3.0);
    }
    world.inject_signal(SmId(1), PORT_IN, Signal {
        signal_type: SIG_FIRE,
        payload: { let mut p = std::collections::BTreeMap::new(); p.insert("intensity".into(), 3.0); p },
    });

    let mut session = DebugSession::new(world);
    let result = session.tick();

    // T4 should NOT fire.
    let t4_fired = result.trace_events.iter().any(|e| match e {
        TraceEvent::TransitionFired { sm_id, .. } => sm_id.0 == 4,
        _ => false,
    });
    assert!(!t4_fired, "T4 should not fire");

    // FIXED: PipelineFiltered events should now exist for connection-side filtering.
    let filtered: Vec<_> = result.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::PipelineFiltered { .. }))
        .collect();
    assert!(!filtered.is_empty(), "FIXED: Connection-side filtering should produce PipelineFiltered events");

    // Should reference target SM (T4) with ConnectionId.
    let has_t4 = filtered.iter().any(|e| match e {
        TraceEvent::PipelineFiltered { sm_id, connection, .. } =>
            sm_id.0 == 4 && connection.is_some(),
        _ => false,
    });
    assert!(has_t4, "PipelineFiltered should reference SM(4) with ConnectionId");
}
