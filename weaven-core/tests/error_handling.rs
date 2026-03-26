/// Error handling tests (§11.5).
///
/// Three error classes:
///   1. StaleSignal        — in-flight signal targeting a despawned SM
///   2. ElapseInvalidState — elapse_fn returned a state not in def.states
///   3. CascadeDepthExceeded — max_cascade_depth reached

use weaven_core::*;
use weaven_core::error::{WeavenDiagnostic, CascadeOverflowPolicy, CascadeOverflowAction};

const SM_A:   SmId   = SmId(1);
const SM_B:   SmId   = SmId(2);
const S0:     StateId = StateId(0);
const S1:     StateId = StateId(1);
const PORT_OUT: PortId = PortId(0);
const PORT_IN:  PortId = PortId(1);
const SIGTYPE: SignalTypeId = SignalTypeId(0);

fn sig(field: &str, val: f64) -> Signal {
    let mut p = std::collections::BTreeMap::new();
    p.insert(field.to_string(), val);
    Signal { signal_type: SIGTYPE, payload: p }
}

// ── 1. Stale Signal ────────────────────────────────────────────────────────

/// Signal queued for SM_B (delay=1). SM_B despawns before delivery.
/// The signal must be purged and StaleSignal diagnostic emitted.
#[test]
fn test_stale_signal_purged_on_despawn() {
    let mut world = World::new();

    world.register_sm(SmDef::new(SM_A, [S0, S1], S0,
        vec![Transition {
            id: TransitionId(10), source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("fire") > 0.0)),
            effects: vec![Box::new(|_| vec![
                EffectOutput::Signal(PORT_OUT, sig("hit", 1.0))
            ])],
        }],
        vec![], vec![Port::new(PORT_OUT, PortKind::Output, SIGTYPE)],
    ));

    world.register_sm(SmDef::new(SM_B, [S0, S1], S0,
        vec![Transition {
            id: TransitionId(20), source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("hit") > 0.0)),
            effects: vec![],
        }],
        vec![Port::new(PORT_IN, PortKind::Input, SIGTYPE)], vec![],
    ));

    world.connect(Connection {
        id: ConnectionId(1),
        source_sm: SM_A, source_port: PORT_OUT,
        target_sm: SM_B, target_port: PORT_IN,
        delay_ticks: 2, // delayed 2 ticks — still in transit when SM_B despawns
        pipeline: vec![],
    });

    // Tick 1: SM_A fires → signal enqueued with delay=1
    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("fire", 1.0); }
    world.activate(SM_A);
    tick(&mut world);
    assert!(!world.signal_queue.is_empty(), "signal in queue with delay=1");

    // SM_B despawns before the signal arrives
    world.request_despawn(vec![SM_B]);

    // Tick 2: despawn fires, signal should be purged, diagnostic emitted
    let out = tick(&mut world);

    assert!(world.signal_queue.is_empty(), "stale signal purged");
    let stale: Vec<_> = out.diagnostics.stale_signals().collect();
    assert_eq!(stale.len(), 1, "one StaleSignal diagnostic");
    if let WeavenDiagnostic::StaleSignal { target_sm, .. } = &stale[0] {
        assert_eq!(*target_sm, SM_B);
    }
}

/// Multiple in-flight signals to the same despawned SM are all purged.
#[test]
fn test_stale_signal_multiple_purged() {
    let mut world = World::new();
    world.register_sm(SmDef::new(SM_B, [S0], S0, vec![],
        vec![Port::new(PORT_IN, PortKind::Input, SIGTYPE)], vec![]));

    // Inject two delayed signals targeting SM_B directly
    world.inject_signal(SM_B, PORT_IN, sig("x", 1.0));
    world.inject_signal(SM_B, PORT_IN, sig("y", 2.0));
    // Manually set delays so they survive Phase 4 (delay > 0)
    for qs in world.signal_queue.iter_mut() {
        qs.delay = 2;
    }

    world.request_despawn(vec![SM_B]);
    let out = tick(&mut world);

    assert!(world.signal_queue.is_empty(), "all stale signals purged");
    assert_eq!(out.diagnostics.stale_signals().count(), 2,
        "two StaleSignal diagnostics");
}

// ── 2. Elapse Invalid State ────────────────────────────────────────────────

const PARENT_SM: SmId    = SmId(10);
const SUB_SM:    SmId    = SmId(11);
const PARENT_A:  StateId = StateId(100);
const PARENT_B:  StateId = StateId(101);
const VALID_SUB: StateId = StateId(200);
const BAD_STATE: StateId = StateId(999); // NOT in sub-SM's states

fn make_parent() -> SmDef {
    SmDef::new(PARENT_SM, [PARENT_A, PARENT_B], PARENT_A, vec![
        Transition {
            id: TransitionId(100), source: PARENT_A, target: PARENT_B, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("go") > 0.0)), effects: vec![],
        },
        Transition {
            id: TransitionId(101), source: PARENT_B, target: PARENT_A, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("back") > 0.0)), effects: vec![],
        },
    ], vec![], vec![])
}

fn enter_b(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("go", 1.0); }
    world.activate(PARENT_SM);
    tick(world);
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("go", 0.0); }
}

fn leave_b(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("back", 1.0); }
    world.activate(PARENT_SM);
    tick(world);
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("back", 0.0); }
}

/// elapse_fn returns a state not in def.states → fallback to frozen state + diagnostic.
#[test]
fn test_elapse_invalid_state_fallback() {
    let mut world = World::new();
    world.register_sm(make_parent());

    let mut sub = SmDef::new(SUB_SM, [VALID_SUB], VALID_SUB, vec![], vec![], vec![]);
    sub.elapse_capability = ElapseCapabilityRt::Deterministic;
    // Returns BAD_STATE which is not in {VALID_SUB}
    sub.elapse_fn = Some(Box::new(|_state, _ctx, _elapsed| {
        (BAD_STATE, Context::default())
    }));
    world.register_sm(sub);

    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Elapse,
        promoted_ports: vec![],
    });

    // Enter B → sub-SM activates at VALID_SUB
    enter_b(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, VALID_SUB);

    // Leave B → sub-SM frozen at VALID_SUB
    leave_b(&mut world);

    // Re-enter B → elapse_fn returns BAD_STATE → fallback to VALID_SUB + diagnostic
    enter_b(&mut world);

    assert_eq!(world.instances[&SUB_SM].active_state, VALID_SUB,
        "fell back to frozen state (VALID_SUB)");
}

/// Valid elapse_fn does NOT emit ElapseInvalidState.
#[test]
fn test_elapse_valid_state_no_diagnostic() {
    let mut world = World::new();
    world.register_sm(make_parent());

    let mut sub = SmDef::new(SUB_SM, [VALID_SUB], VALID_SUB, vec![], vec![], vec![]);
    sub.elapse_capability = ElapseCapabilityRt::Deterministic;
    // Returns VALID_SUB which IS in {VALID_SUB}
    sub.elapse_fn = Some(Box::new(|state, ctx, _elapsed| (state, ctx.clone())));
    world.register_sm(sub);

    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Elapse,
        promoted_ports: vec![],
    });

    enter_b(&mut world);
    leave_b(&mut world);
    let out = enter_b_with_output(&mut world);

    let invalid: Vec<_> = out.diagnostics.items.iter()
        .filter(|d| matches!(d, WeavenDiagnostic::ElapseInvalidState { .. }))
        .collect();
    assert!(invalid.is_empty(), "no ElapseInvalidState for valid return");
}

fn enter_b_with_output(world: &mut World) -> TickOutput {
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("go", 1.0); }
    world.activate(PARENT_SM);
    let out = tick(world);
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("go", 0.0); }
    out
}

// ── 3. Cascade Depth Exceeded ──────────────────────────────────────────────

fn make_loop_sm(id: SmId, next_id: SmId, port_out: PortId, port_in: PortId) -> SmDef {
    // SM that on every signal immediately re-emits → infinite cascade
    SmDef::new(id, [S0], S0, vec![
        Transition {
            id: TransitionId(id.0 * 100), source: S0, target: S0, priority: 10,
            guard: Some(Box::new(move |ctx, _| ctx.get("ping") > 0.0)),
            effects: vec![Box::new(move |_| vec![
                EffectOutput::Signal(port_out, sig("ping", 1.0))
            ])],
        }
    ],
    vec![Port::new(port_in, PortKind::Input, SIGTYPE)],
    vec![Port::new(port_out, PortKind::Output, SIGTYPE)])
}

/// DiscardAndContinue: overflow emits diagnostic, excess signals dropped.
#[test]
fn test_cascade_depth_exceeded_discard() {
    let mut world = World::new();
    world.max_cascade_depth = 3;
    world.cascade_overflow_policy = CascadeOverflowPolicy::DiscardAndContinue;

    let port_ab_out = PortId(0); let port_b_in = PortId(1);
    let port_ba_out = PortId(2); let port_a_in = PortId(3);

    world.register_sm(make_loop_sm(SM_A, SM_B, port_ab_out, port_a_in));
    world.register_sm(make_loop_sm(SM_B, SM_A, port_ba_out, port_b_in));

    world.connect(Connection { id: ConnectionId(1),
        source_sm: SM_A, source_port: port_ab_out,
        target_sm: SM_B, target_port: port_b_in,
        delay_ticks: 0, pipeline: vec![] });
    world.connect(Connection { id: ConnectionId(2),
        source_sm: SM_B, source_port: port_ba_out,
        target_sm: SM_A, target_port: port_a_in,
        delay_ticks: 0, pipeline: vec![] });

    // Kick off cascade
    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("ping", 1.0); }
    world.activate(SM_A);
    let out = tick(&mut world);

    let overflows: Vec<_> = out.diagnostics.cascade_overflows().collect();
    assert!(!overflows.is_empty(), "CascadeDepthExceeded diagnostic emitted");
    if let WeavenDiagnostic::CascadeDepthExceeded { depth_reached, action, .. } = &overflows[0] {
        assert_eq!(*depth_reached, 3);
        assert_eq!(*action, CascadeOverflowAction::DiscardAndContinue);
    }
    // With DiscardAndContinue, queue should be empty after the tick
    assert!(world.signal_queue.is_empty(), "signals discarded");
}

/// DeferToNextTick: overflow signals preserved for next tick.
#[test]
fn test_cascade_depth_exceeded_defer() {
    let sm_c = SmId(3);
    let mut world = World::new();
    world.max_cascade_depth = 2;
    world.cascade_overflow_policy = CascadeOverflowPolicy::DeferToNextTick;

    let port_out = PortId(0); let port_in = PortId(1);

    world.register_sm(make_loop_sm(SM_A, sm_c, port_out, port_in));
    world.connect(Connection { id: ConnectionId(1),
        source_sm: SM_A, source_port: port_out,
        target_sm: SM_A, target_port: port_in, // self-loop
        delay_ticks: 0, pipeline: vec![] });

    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("ping", 1.0); }
    world.activate(SM_A);
    let out = tick(&mut world);

    let overflows: Vec<_> = out.diagnostics.cascade_overflows().collect();
    assert!(!overflows.is_empty(), "overflow detected");
    if let WeavenDiagnostic::CascadeDepthExceeded { action, .. } = &overflows[0] {
        assert_eq!(*action, CascadeOverflowAction::DeferToNextTick);
    }
    // With DeferToNextTick, some signals survive in the queue for next tick
    assert!(!world.signal_queue.is_empty(),
        "deferred signals preserved for next tick");
}

/// No cascade overflow for normal depth: diagnostics empty.
#[test]
fn test_no_cascade_overflow_normal_operation() {
    let mut world = World::new();
    world.max_cascade_depth = 64;

    world.register_sm(SmDef::new(SM_A, [S0, S1], S0, vec![
        Transition {
            id: TransitionId(10), source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("trigger") > 0.0)),
            effects: vec![Box::new(|_| vec![
                EffectOutput::Signal(PORT_OUT, sig("ping", 1.0))
            ])],
        }
    ], vec![], vec![Port::new(PORT_OUT, PortKind::Output, SIGTYPE)]));

    world.register_sm(SmDef::new(SM_B, [S0, S1], S0, vec![
        Transition {
            id: TransitionId(20), source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("ping") > 0.0)),
            effects: vec![],
        }
    ], vec![Port::new(PORT_IN, PortKind::Input, SIGTYPE)], vec![]));

    world.connect(Connection { id: ConnectionId(1),
        source_sm: SM_A, source_port: PORT_OUT,
        target_sm: SM_B, target_port: PORT_IN,
        delay_ticks: 0, pipeline: vec![] });

    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("trigger", 1.0); }
    world.activate(SM_A);
    let out = tick(&mut world);

    assert!(out.diagnostics.cascade_overflows().next().is_none(),
        "no cascade overflow for 1-hop cascade");
    assert!(out.diagnostics.is_empty(), "no diagnostics at all");
}
