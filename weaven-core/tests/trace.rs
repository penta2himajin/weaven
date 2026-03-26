// Integration test: TraceEvent collection (debugger §3).
//
// Verifies that the `trace` feature produces correct events
// for Guard evaluation, Transition firing, Signal emission,
// and Phase 4 cascade steps.
#![cfg(feature = "trace")]

use weaven_core::*;
use weaven_core::trace::{TraceEvent, Phase as TracePhase};

const S_IDLE:    StateId = StateId(0);
const S_ACTIVE:  StateId = StateId(1);
const S_DONE:    StateId = StateId(2);
const P_IN:      PortId  = PortId(0);
const P_OUT:     PortId  = PortId(1);
const SIG_TYPE:  SignalTypeId = SignalTypeId(0);

/// Build a simple SM: Idle --(signal received)--> Active --(emit signal)--> Done
fn simple_sm(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [S_IDLE, S_ACTIVE, S_DONE].into_iter().collect(),
        initial_state: S_IDLE,
        transitions: vec![
            Transition {
                id: TransitionId(id.0 * 10),
                source: S_IDLE,
                target: S_ACTIVE,
                priority: 10,
                guard: Some(Box::new(|ctx, _sig| ctx.get("trigger") > 0.0)),
                effects: vec![
                    Box::new(|_ctx| {
                        let mut payload = std::collections::BTreeMap::new();
                        payload.insert("trigger".to_string(), 1.0);
                        vec![EffectOutput::Signal(P_OUT, Signal { signal_type: SIG_TYPE, payload })]
                    }),
                ],
            },
            Transition {
                id: TransitionId(id.0 * 10 + 1),
                source: S_ACTIVE,
                target: S_DONE,
                priority: 10,
                guard: Some(Box::new(|ctx, _sig| ctx.get("trigger") > 0.0)),
                effects: vec![],
            },
        ],
        input_ports: vec![Port::new(P_IN, PortKind::Input, SIG_TYPE)],
        output_ports: vec![Port::new(P_OUT, PortKind::Output, SIG_TYPE)],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn setup_world() -> World {
    let sm_a = SmId(1);
    let sm_b = SmId(2);
    let mut world = World::new();

    world.register_sm(simple_sm(sm_a));
    world.register_sm(simple_sm(sm_b));

    // Connection: A.out → B.in
    world.connections.push(Connection {
        id: ConnectionId(1),
        source_sm: sm_a,
        source_port: P_OUT,
        target_sm: sm_b,
        target_port: P_IN,
        pipeline: vec![],
        delay_ticks: 0,
    });

    // Inject initial signal to SM A
    world.signal_queue.push_back(QueuedSignal {
        target_sm: sm_a,
        target_port: P_IN,
        signal: Signal {
            signal_type: SIG_TYPE,
            payload: {
                let mut p = std::collections::BTreeMap::new();
                p.insert("trigger".to_string(), 1.0);
                p
            },
        },
        delay: 0,
        source_conn: None,
    });
    world.active_set.insert(sm_a);

    world
}

#[test]
fn trace_events_guard_evaluated() {
    let mut world = setup_world();
    let out = tick(&mut world);

    let guard_events: Vec<_> = out.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::GuardEvaluated { .. }))
        .collect();

    // At minimum SM A's transitions should have been evaluated
    assert!(!guard_events.is_empty(), "Expected GuardEvaluated events");

    // Check that at least one guard evaluated to true (the one that fires)
    let any_true = guard_events.iter().any(|e| match e {
        TraceEvent::GuardEvaluated { result, .. } => *result,
        _ => false,
    });
    assert!(any_true, "Expected at least one guard to pass");
}

#[test]
fn trace_events_transition_fired() {
    let mut world = setup_world();
    let out = tick(&mut world);

    let fired_events: Vec<_> = out.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::TransitionFired { .. }))
        .collect();

    // SM A should fire Idle→Active in Phase 3, SM B fires in cascade (Phase 4)
    assert!(fired_events.len() >= 1, "Expected at least one TransitionFired, got {}", fired_events.len());

    // Verify SM A's transition
    let sm_a_fired = fired_events.iter().any(|e| match e {
        TraceEvent::TransitionFired { sm_id, from_state, to_state, .. } =>
            *sm_id == SmId(1) && *from_state == S_IDLE && *to_state == S_ACTIVE,
        _ => false,
    });
    assert!(sm_a_fired, "SM A should fire Idle→Active");
}

#[test]
fn trace_events_signal_emitted() {
    let mut world = setup_world();
    let out = tick(&mut world);

    let emit_events: Vec<_> = out.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::SignalEmitted { .. }))
        .collect();

    // SM A emits a signal when transitioning Idle→Active
    assert!(!emit_events.is_empty(), "Expected SignalEmitted events");

    let sm_a_emitted = emit_events.iter().any(|e| match e {
        TraceEvent::SignalEmitted { sm_id, port, .. } =>
            *sm_id == SmId(1) && *port == P_OUT,
        _ => false,
    });
    assert!(sm_a_emitted, "SM A should emit signal on P_OUT");
}

#[test]
fn trace_events_cascade_step() {
    let mut world = setup_world();
    let out = tick(&mut world);

    let cascade_events: Vec<_> = out.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::CascadeStep { .. }))
        .collect();

    // There should be cascade steps (signal delivery to SM B)
    assert!(!cascade_events.is_empty(), "Expected CascadeStep events");

    // First cascade step should be depth 1
    match &cascade_events[0] {
        TraceEvent::CascadeStep { depth, phase, .. } => {
            assert_eq!(*depth, 1);
            assert_eq!(*phase, TracePhase::Propagate);
        }
        _ => panic!("Expected CascadeStep"),
    }
}

#[test]
fn trace_events_phase_ordering() {
    let mut world = setup_world();
    let out = tick(&mut world);

    // Verify phase ordering: Evaluate events come before Execute, Execute before Propagate
    let phases: Vec<TracePhase> = out.trace_events.iter().map(|e| match e {
        TraceEvent::GuardEvaluated { phase, .. } => *phase,
        TraceEvent::IrMatched { phase, .. } => *phase,
        TraceEvent::TransitionFired { phase, .. } => *phase,
        TraceEvent::SignalEmitted { phase, .. } => *phase,
        TraceEvent::CascadeStep { phase, .. } => *phase,
        TraceEvent::PipelineFiltered { phase, .. } => *phase,
    }).collect();

    // Find last Evaluate event and first Execute event
    let last_eval = phases.iter().rposition(|p| *p == TracePhase::Evaluate);
    let first_exec = phases.iter().position(|p| *p == TracePhase::Execute);
    if let (Some(le), Some(fe)) = (last_eval, first_exec) {
        assert!(le < fe, "All Evaluate events should precede Execute events");
    }

    // Find last Execute event and first Propagate event
    let last_exec = phases.iter().rposition(|p| *p == TracePhase::Execute);
    let first_prop = phases.iter().position(|p| *p == TracePhase::Propagate);
    if let (Some(le), Some(fp)) = (last_exec, first_prop) {
        assert!(le < fp, "All Execute events should precede Propagate events");
    }
}

#[test]
fn trace_events_empty_when_no_feature() {
    // This test only compiles with trace feature, so it validates that
    // trace_events is populated (not empty) when the feature IS on.
    let mut world = setup_world();
    let out = tick(&mut world);
    assert!(!out.trace_events.is_empty(), "trace_events should be populated with trace feature");
}

// =========================================================================
// Gap 1: SignalEmitted.target must be populated after routing
// =========================================================================

#[test]
fn gap1_signal_emitted_has_target() {
    // SM A emits → routed to SM B via Connection.
    // The SignalEmitted event should have target = Some(SmId(2)).
    let mut world = setup_world();
    let out = tick(&mut world);

    let emitted_with_target: Vec<_> = out.trace_events.iter()
        .filter_map(|e| match e {
            TraceEvent::SignalEmitted { sm_id, target, .. } => Some((*sm_id, *target)),
            _ => None,
        })
        .filter(|(_, target)| target.is_some())
        .collect();

    assert!(!emitted_with_target.is_empty(),
        "At least one SignalEmitted should have a resolved target");

    // SM A (SmId(1)) emits to SM B (SmId(2)).
    let a_to_b = emitted_with_target.iter()
        .any(|(src, tgt)| src.0 == 1 && tgt.unwrap().0 == 2);
    assert!(a_to_b, "Should trace SM(1) → SM(2) signal path");
}

// =========================================================================
// Gap 2: Connection-side PipelineFiltered must be traced
// =========================================================================

const S_GRASS:   StateId = StateId(10);
const S_BURNING: StateId = StateId(11);
const P_FIRE_IN:  PortId = PortId(10);
const P_FIRE_OUT: PortId = PortId(11);
const SIG_FIRE: SignalTypeId = SignalTypeId(1);

fn tile_sm(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [S_GRASS, S_BURNING].into_iter().collect(),
        initial_state: S_GRASS,
        transitions: vec![
            Transition {
                id: TransitionId(id.0 * 10),
                source: S_GRASS,
                target: S_BURNING,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("intensity") > 0.0)),
                effects: vec![
                    Box::new(|ctx| {
                        let v = ctx.get("intensity");
                        if v <= 0.0 { return vec![]; }
                        let mut p = std::collections::BTreeMap::new();
                        p.insert("intensity".into(), v);
                        vec![EffectOutput::Signal(P_FIRE_OUT, Signal { signal_type: SIG_FIRE, payload: p })]
                    }),
                ],
            },
        ],
        input_ports: vec![Port::new(P_FIRE_IN, PortKind::Input, SIG_FIRE)],
        output_ports: vec![Port::new(P_FIRE_OUT, PortKind::Output, SIG_FIRE)],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

#[test]
fn gap2_connection_pipeline_filter_traced() {
    // T1 → T2 → T3, intensity=2. Pipeline: intensity -= 1, filter intensity > 0.
    // T1 fires (intensity=2), emits to T2 (intensity=1 after transform).
    // T2 fires (intensity=1), emits to T3 (intensity=0 after transform → FILTERED).
    // The filter event for T3 should appear as PipelineFiltered.
    let mut world = World::new();
    for i in 1..=3 {
        world.register_sm(tile_sm(SmId(i)));
    }
    for i in 1..=2 {
        world.connections.push(Connection {
            id: ConnectionId(i),
            source_sm: SmId(i),
            source_port: P_FIRE_OUT,
            target_sm: SmId(i + 1),
            target_port: P_FIRE_IN,
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
        });
    }

    // Ignite T1.
    if let Some(inst) = world.instances.get_mut(&SmId(1)) {
        inst.context.set("intensity", 2.0);
    }
    world.inject_signal(SmId(1), P_FIRE_IN, Signal {
        signal_type: SIG_FIRE,
        payload: { let mut p = std::collections::BTreeMap::new(); p.insert("intensity".into(), 2.0); p },
    });

    let out = tick(&mut world);

    // T3 should NOT fire (intensity filtered to 0).
    let t3_fired = out.trace_events.iter().any(|e| match e {
        TraceEvent::TransitionFired { sm_id, .. } => sm_id.0 == 3,
        _ => false,
    });
    assert!(!t3_fired, "T3 should not fire");

    // There should be a PipelineFiltered event for the Connection-side filter.
    let conn_filtered: Vec<_> = out.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::PipelineFiltered { .. }))
        .collect();
    assert!(!conn_filtered.is_empty(),
        "Connection-side pipeline filtering should produce a PipelineFiltered trace event");

    // The filtered event should reference the target SM (T3) and connection.
    let has_t3_filter = conn_filtered.iter().any(|e| match e {
        TraceEvent::PipelineFiltered { sm_id, connection, .. } =>
            sm_id.0 == 3 && connection.is_some(),
        _ => false,
    });
    assert!(has_t3_filter, "PipelineFiltered should reference SM(3) with ConnectionId");
}

// =========================================================================
// Gap 3: GuardEvaluated should include context snapshot
// =========================================================================

#[test]
fn gap3_guard_evaluated_has_context_snapshot() {
    // When a guard is evaluated, the trace should capture key context values
    // so the debugger can show WHY the guard passed or failed.
    let mut world = setup_world();
    let out = tick(&mut world);

    let guard_events: Vec<_> = out.trace_events.iter()
        .filter(|e| matches!(e, TraceEvent::GuardEvaluated { .. }))
        .collect();

    assert!(!guard_events.is_empty());

    // At least one guard should have a non-empty context snapshot.
    let has_context = guard_events.iter().any(|e| match e {
        TraceEvent::GuardEvaluated { context_snapshot, .. } => {
            context_snapshot.as_ref().map_or(false, |ctx| !ctx.is_empty())
        }
        _ => false,
    });
    assert!(has_context,
        "GuardEvaluated should include context snapshot for debugging");
}
