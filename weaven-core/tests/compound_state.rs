/// Compound State tests (§4): sub-SM lifecycle, SuspendPolicy, Port Promotion.
///
/// Setup pattern used throughout:
///   Parent SM: StateA(0) → StateB(1) → StateA(0)
///   Sub-SM:    SubIdle(10) → SubActive(11)
///
/// When parent enters StateB, sub-SM is activated.
/// When parent exits StateB, sub-SM is handled per SuspendPolicy.

use weaven_core::*;

// ── IDs ────────────────────────────────────────────────────────────────────
const PARENT_A: StateId = StateId(0);
const PARENT_B: StateId = StateId(1);

const SUB_IDLE:   StateId = StateId(10);
const SUB_ACTIVE: StateId = StateId(11);

const PARENT_SM: SmId = SmId(1);
const SUB_SM:    SmId = SmId(2);

const PORT_ENTER_B:   PortId = PortId(0); // trigger: parent A→B
const PORT_LEAVE_B:   PortId = PortId(1); // trigger: parent B→A
const PORT_SUB_FIRE:  PortId = PortId(2); // trigger: sub Idle→Active

const SIGTYPE: SignalTypeId = SignalTypeId(0);

fn sig(key: &str, val: f64) -> Signal {
    let mut p = std::collections::BTreeMap::new();
    p.insert(key.to_string(), val);
    Signal { signal_type: SIGTYPE, payload: p }
}

fn make_parent_sm() -> SmDef {
    SmDef {
        id: PARENT_SM,
        states: [PARENT_A, PARENT_B].into_iter().collect(),
        initial_state: PARENT_A,
        transitions: vec![
            Transition {
                id: TransitionId(10),
                source: PARENT_A, target: PARENT_B, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("enter_b") > 0.0)),
                effects: vec![],
            },
            Transition {
                id: TransitionId(11),
                source: PARENT_B, target: PARENT_A, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("leave_b") > 0.0)),
                effects: vec![],
            },
        ],
        input_ports: vec![
            Port::new(PORT_ENTER_B, PortKind::Input, SIGTYPE),
            Port::new(PORT_LEAVE_B, PortKind::Input, SIGTYPE),
        ],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn make_sub_sm() -> SmDef {
    SmDef {
        id: SUB_SM,
        states: [SUB_IDLE, SUB_ACTIVE].into_iter().collect(),
        initial_state: SUB_IDLE,
        transitions: vec![
            Transition {
                id: TransitionId(20),
                source: SUB_IDLE, target: SUB_ACTIVE, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("sub_fire") > 0.0)),
                effects: vec![],
            },
        ],
        input_ports: vec![
            Port::new(PORT_SUB_FIRE, PortKind::Input, SIGTYPE),
        ],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

/// Trigger parent A→B and verify sub-SM is activated.
fn enter_b(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("enter_b", 1.0); }
    world.activate(PARENT_SM);
    tick(world);
    // Clear trigger to avoid re-firing
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("enter_b", 0.0); }
}

/// Trigger parent B→A.
fn leave_b(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("leave_b", 1.0); }
    world.activate(PARENT_SM);
    tick(world);
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("leave_b", 0.0); }
}

/// Fire sub-SM Idle→Active.
fn fire_sub(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&SUB_SM) { i.context.set("sub_fire", 1.0); }
    world.activate(SUB_SM);
    tick(world);
    if let Some(i) = world.instances.get_mut(&SUB_SM) { i.context.set("sub_fire", 0.0); }
}

// ── Tests: activation ──────────────────────────────────────────────────────

/// Sub-SM is activated when parent enters the compound state.
#[test]
fn test_sub_sm_activated_on_parent_entry() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    world.register_sm(make_sub_sm());
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B,
        parent_sm:    PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Freeze,
        promoted_ports: vec![],
    });

    enter_b(&mut world);

    assert_eq!(world.instances[&PARENT_SM].active_state, PARENT_B, "parent→B");
    assert!(world.active_set.contains(&SUB_SM), "sub-SM should be in Active Set after entry");
    assert_eq!(world.instances[&SUB_SM].active_state, SUB_IDLE, "sub-SM starts at initial state");
}

/// Sub-SM is NOT active before parent enters the compound state.
#[test]
fn test_sub_sm_not_active_before_parent_entry() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    world.register_sm(make_sub_sm());
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Freeze,
        promoted_ports: vec![],
    });

    // No entry yet — sub-SM should be dormant
    assert!(!world.active_set.contains(&SUB_SM), "sub-SM should be dormant initially");
}

// ── Tests: SuspendPolicy::Freeze ───────────────────────────────────────────

/// Freeze: sub-SM state is preserved across parent exit/re-entry.
#[test]
fn test_freeze_preserves_sub_sm_state() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    world.register_sm(make_sub_sm());
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Freeze,
        promoted_ports: vec![],
    });

    // Enter B → sub-SM starts Idle
    enter_b(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, SUB_IDLE);

    // Advance sub-SM to Active
    fire_sub(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, SUB_ACTIVE, "sub→Active");

    // Exit B → sub-SM frozen at Active
    leave_b(&mut world);
    assert!(!world.active_set.contains(&SUB_SM), "sub-SM dormant after freeze");
    assert_eq!(world.frozen_snapshots[&SUB_SM].active_state, SUB_ACTIVE, "snapshot preserved");

    // Re-enter B → sub-SM resumes at Active
    enter_b(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, SUB_ACTIVE, "resumed at Active");
    assert!(world.active_set.contains(&SUB_SM), "sub-SM active again");
}

// ── Tests: SuspendPolicy::Discard ──────────────────────────────────────────

/// Discard: sub-SM is reset to initial state on each entry.
#[test]
fn test_discard_resets_sub_sm_on_exit() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    world.register_sm(make_sub_sm());
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Discard,
        promoted_ports: vec![],
    });

    // Enter B, advance sub-SM
    enter_b(&mut world);
    fire_sub(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, SUB_ACTIVE);

    // Exit B → sub-SM discarded (reset to initial)
    leave_b(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, SUB_IDLE, "reset to Idle");
    assert!(world.frozen_snapshots.is_empty(), "no snapshot for Discard");

    // Re-enter B → starts fresh at Idle
    enter_b(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, SUB_IDLE, "fresh start");
}

// ── Tests: Port Promotion (§4.4) ───────────────────────────────────────────

/// Port Promotion: a sub-SM event can trigger a parent-level transition.
///
/// Setup:
///   Sub-SM fires Idle→Active and emits on PORT_SUB_EMIT.
///   This port is promoted to the parent SM's scope.
///   A parent-level Transition on PARENT_B reads the promoted signal
///   and advances PARENT_B → PARENT_A.
#[test]
fn test_port_promotion_sub_event_triggers_parent_transition() {
    const PORT_SUB_EMIT: PortId = PortId(3);
    const PARENT_C: StateId = StateId(2); // new state to avoid confusion

    // Sub-SM that emits on PORT_SUB_EMIT when it fires
    let sub_emitting = SmDef {
        id: SUB_SM,
        states: [SUB_IDLE, SUB_ACTIVE].into_iter().collect(),
        initial_state: SUB_IDLE,
        transitions: vec![Transition {
            id: TransitionId(20),
            source: SUB_IDLE, target: SUB_ACTIVE, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("sub_fire") > 0.0)),
            effects: vec![Box::new(|_ctx| {
                let p = std::collections::BTreeMap::new();
                vec![EffectOutput::Signal(PORT_SUB_EMIT, Signal { signal_type: SIGTYPE, payload: p })]
            })],
        }],
        input_ports: vec![Port::new(PORT_SUB_FIRE, PortKind::Input, SIGTYPE)],
        output_ports: vec![Port::new(PORT_SUB_EMIT, PortKind::Output, SIGTYPE)],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    };

    // Parent SM with a promoted-port-driven transition: B → C
    const PORT_PROMOTED_IN: PortId = PortId(4);
    let parent_with_promotion = SmDef {
        id: PARENT_SM,
        states: [PARENT_A, PARENT_B, PARENT_C].into_iter().collect(),
        initial_state: PARENT_A,
        transitions: vec![
            Transition {
                id: TransitionId(10),
                source: PARENT_A, target: PARENT_B, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("enter_b") > 0.0)),
                effects: vec![],
            },
            // Parent transition driven by promoted port signal
            Transition {
                id: TransitionId(12),
                source: PARENT_B, target: PARENT_C, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("promoted_fired") > 0.0)),
                effects: vec![],
            },
        ],
        input_ports: vec![
            Port::new(PORT_ENTER_B, PortKind::Input, SIGTYPE),
            Port::new(PORT_PROMOTED_IN, PortKind::Input, SIGTYPE),
        ],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    };

    let mut world = World::new();
    world.register_sm(parent_with_promotion);
    world.register_sm(sub_emitting);

    // Connect promoted sub-SM output to parent's promoted input port
    world.connect(Connection {
        id: ConnectionId(1),
        source_sm: SUB_SM,    source_port: PORT_SUB_EMIT,
        target_sm: PARENT_SM, target_port: PORT_PROMOTED_IN,
        delay_ticks: 0,
        pipeline: vec![
            // Write promoted_fired into parent context so the guard can read it
            PipelineStep::Transform(Box::new(|mut sig| {
                sig.payload.insert("promoted_fired".to_string(), 1.0);
                sig
            })),
        ],
    });

    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Discard,
        promoted_ports: vec![(SUB_SM, PORT_SUB_EMIT)],
    });

    // Enter B → sub-SM activates
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("enter_b", 1.0); }
    world.activate(PARENT_SM);
    tick(&mut world);
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("enter_b", 0.0); }

    assert_eq!(world.instances[&PARENT_SM].active_state, PARENT_B);

    // Fire sub-SM → emits on PORT_SUB_EMIT → cascades through Connection → parent guard fires
    if let Some(i) = world.instances.get_mut(&SUB_SM) { i.context.set("sub_fire", 1.0); }
    world.activate(SUB_SM);
    tick(&mut world);

    // Sub-SM emitted → Connection pipeline wrote "promoted_fired=1" into parent context
    // Phase 4 cascade: parent PARENT_B → PARENT_C
    assert_eq!(world.instances[&PARENT_SM].active_state, PARENT_C,
        "parent advanced to C via promoted port signal");
    assert_eq!(world.instances[&SUB_SM].active_state, SUB_ACTIVE,
        "sub-SM reached Active");
}

// ── Tests: multiple parallel sub-SMs ──────────────────────────────────────

const SUB_SM_2: SmId = SmId(3);

fn make_sub_sm_2() -> SmDef {
    const S0: StateId = StateId(20);
    const S1: StateId = StateId(21);
    const PORT: PortId = PortId(5);
    SmDef {
        id: SUB_SM_2,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(30),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("sub2_fire") > 0.0)),
            effects: vec![],
        }],
        input_ports: vec![Port::new(PORT, PortKind::Input, SIGTYPE)],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

/// Multiple parallel sub-SMs both activate on parent entry.
#[test]
fn test_multiple_parallel_sub_sms_activate() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    world.register_sm(make_sub_sm());
    world.register_sm(make_sub_sm_2());
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM, SUB_SM_2],
        suspend_policy: SuspendPolicyRt::Discard,
        promoted_ports: vec![],
    });

    enter_b(&mut world);

    assert!(world.active_set.contains(&SUB_SM),   "sub-SM 1 active");
    assert!(world.active_set.contains(&SUB_SM_2), "sub-SM 2 active");
    assert_eq!(world.instances[&SUB_SM].active_state,   SUB_IDLE, "sub1 at initial");
    assert_eq!(world.instances[&SUB_SM_2].active_state, StateId(20), "sub2 at initial");
}

/// On parent exit with Freeze, both parallel sub-SMs are frozen independently.
#[test]
fn test_multiple_sub_sms_freeze_independently() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    world.register_sm(make_sub_sm());
    world.register_sm(make_sub_sm_2());
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM, SUB_SM_2],
        suspend_policy: SuspendPolicyRt::Freeze,
        promoted_ports: vec![],
    });

    enter_b(&mut world);

    // Advance only sub-SM 1
    fire_sub(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, SUB_ACTIVE);
    assert_eq!(world.instances[&SUB_SM_2].active_state, StateId(20)); // sub2 untouched

    leave_b(&mut world);

    // sub1 frozen at Active, sub2 frozen at initial
    assert_eq!(world.frozen_snapshots[&SUB_SM].active_state, SUB_ACTIVE);
    assert_eq!(world.frozen_snapshots[&SUB_SM_2].active_state, StateId(20));

    // Re-enter: both resume correctly
    enter_b(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state,   SUB_ACTIVE, "sub1 resumed");
    assert_eq!(world.instances[&SUB_SM_2].active_state, StateId(20), "sub2 resumed");
}
