/// Tests for Active Set management (§7.2) and Appendix B Parry combat scenario.

use weaven_core::*;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn empty_connection_vec() -> Vec<PipelineStep> { vec![] }

fn make_two_state_sm(id: SmId, trigger_key: &'static str) -> SmDef {
    const S0: StateId = StateId(0);
    const S1: StateId = StateId(1);
    SmDef {
        id,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![
            Transition {
                id: TransitionId(id.0 * 10),
                source: S0,
                target: S1,
                priority: 10,
                guard: Some(Box::new(move |ctx, _| ctx.get(trigger_key) > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        input_ports: vec![],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

// ---------------------------------------------------------------------------
// Active Set tests
// ---------------------------------------------------------------------------

/// An SM that never fires should be removed from the Active Set after one tick.
#[test]
fn test_dormant_sm_leaves_active_set() {
    let sm_id = SmId(1);
    let mut world = World::new();
    world.register_sm(make_two_state_sm(sm_id, "trigger"));
    world.activate(sm_id); // manually activate; no signals pending

    assert!(world.active_set.contains(&sm_id), "should start active");
    tick(&mut world);
    assert!(!world.active_set.contains(&sm_id), "should be dormant after non-firing tick");
}

/// An SM that fires a transition stays active (it might fire again next tick).
#[test]
fn test_firing_sm_stays_active() {
    let sm_id = SmId(1);
    let mut world = World::new();
    world.register_sm(make_two_state_sm(sm_id, "trigger"));
    if let Some(i) = world.instances.get_mut(&sm_id) {
        i.context.set("trigger", 1.0);
    }
    world.activate(sm_id);

    tick(&mut world);
    // SM fired S0→S1; it stays active because fired_this_tick contains it.
    assert!(world.active_set.contains(&sm_id), "should stay active after firing");
}

/// An SM with a deferred signal (delay>0) stays active until delivered.
#[test]
fn test_sm_with_deferred_signal_stays_active() {
    const PORT_IN: PortId = PortId(0);
    const SIGTYPE: SignalTypeId = SignalTypeId(0);
    const S0: StateId = StateId(0);
    const S1: StateId = StateId(1);

    let src_id = SmId(1);
    let tgt_id = SmId(2);

    // src fires and emits through a delay=1 connection
    let mut world = World::new();
    world.register_sm(SmDef {
        id: src_id,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(10),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("fire") > 0.0)),
            guard_expr: None,
            effects: vec![Box::new(|_ctx| {
                let mut p = std::collections::BTreeMap::new();
                p.insert("v".to_string(), 1.0);
                vec![EffectOutput::Signal(PortId(1), Signal { signal_type: SIGTYPE, payload: p })]
            })],
        }],
        input_ports: vec![],
        output_ports: vec![Port::new(PortId(1), PortKind::Output, SIGTYPE)],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });
    world.register_sm(SmDef {
        id: tgt_id,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(20),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("v") > 0.0)),
            guard_expr: None,
            effects: vec![],
        }],
        input_ports: vec![Port::new(PORT_IN, PortKind::Input, SIGTYPE)],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });
    world.connect(Connection {
        id: ConnectionId(1),
        source_sm: src_id, source_port: PortId(1),
        target_sm: tgt_id,  target_port: PORT_IN,
        delay_ticks: 1, pipeline: empty_connection_vec(),
    });

    if let Some(i) = world.instances.get_mut(&src_id) { i.context.set("fire", 1.0); }
    world.activate(src_id);

    // Tick 1: src fires S0→S1, emits signal with delay=1.
    // tgt does NOT need to be in active_set — Phase 4 delivers deferred signals
    // from the queue directly, regardless of active_set membership (§7.2).
    tick(&mut world);
    assert_eq!(world.instances[&src_id].active_state, S1, "src fired");

    // The deferred signal must be sitting in the queue (delay now decremented to 0).
    let pending_for_tgt = world.signal_queue.iter().any(|qs| qs.target_sm == tgt_id);
    assert!(pending_for_tgt, "deferred signal should be queued for tgt");

    // Tick 2: delay=0 signal is ready, Phase 4 delivers it, tgt fires S0→S1.
    tick(&mut world);
    assert_eq!(world.instances[&tgt_id].active_state, S1, "tgt fired after delay");
}

/// State diff only reports SMs that actually changed state.
#[test]
fn test_tick_output_diff_only_changed() {
    let sm_a = SmId(1); // will fire
    let sm_b = SmId(2); // will not fire

    let mut world = World::new();
    world.register_sm(make_two_state_sm(sm_a, "go"));
    world.register_sm(make_two_state_sm(sm_b, "go")); // same trigger key, but won't set it

    if let Some(i) = world.instances.get_mut(&sm_a) { i.context.set("go", 1.0); }
    world.activate(sm_a);
    world.activate(sm_b);

    let out = tick(&mut world);
    assert!(out.state_changes.contains_key(&sm_a), "sm_a changed");
    assert!(!out.state_changes.contains_key(&sm_b), "sm_b did not change");
}

// ---------------------------------------------------------------------------
// Appendix B: Parry scenario
// ---------------------------------------------------------------------------
//
// Setup:
//   Enemy AttackSM: WindUp(0) → ActiveFrame(1) — fires when timer expires
//   PC    ParrySM:  Idle(0)   → Parry(1)        — fires when input arrives
//
// Tick N: both SMs transition simultaneously.
//   Phase 2: Interaction Rule evaluates PRE-transition states.
//             Enemy=WindUp, PC=Idle → no match.
//   Phase 3: Enemy→ActiveFrame, PC→Parry.
//
// Tick N+1:
//   Phase 2: Enemy=ActiveFrame, PC=Parry, proximity→ MATCH.
//             Interaction Rule enqueues StaggerIn→Enemy, ParrySuccessIn→PC.
//   Phase 3: Enemy→Staggered, PC→Riposte.
//
// Interaction Rules are not yet implemented as a system, so we simulate them
// as a manual pre-tick injection step (Phase 2 stub).

const ENEMY_WINDUP:      StateId = StateId(0);
const ENEMY_ACTIVEFRAME: StateId = StateId(1);
const ENEMY_STAGGERED:   StateId = StateId(2);

const PC_IDLE:    StateId = StateId(10);
const PC_PARRY:   StateId = StateId(11);
const PC_RIPOSTE: StateId = StateId(12);

const PORT_STAGGER_IN:       PortId = PortId(0);
const PORT_PARRY_SUCCESS_IN: PortId = PortId(1);

const SIGTYPE_COMBAT: SignalTypeId = SignalTypeId(0);

fn combat_signal() -> Signal {
    Signal { signal_type: SIGTYPE_COMBAT, payload: std::collections::BTreeMap::new() }
}

fn make_enemy_sm(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [ENEMY_WINDUP, ENEMY_ACTIVEFRAME, ENEMY_STAGGERED].into_iter().collect(),
        initial_state: ENEMY_WINDUP,
        transitions: vec![
            // WindUp → ActiveFrame when timer expires (simulated via "timer_expired" context)
            Transition {
                id: TransitionId(id.0 * 100 + 0),
                source: ENEMY_WINDUP, target: ENEMY_ACTIVEFRAME,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("timer_expired") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
            // ActiveFrame → Staggered on StaggerIn signal
            Transition {
                id: TransitionId(id.0 * 100 + 1),
                source: ENEMY_ACTIVEFRAME, target: ENEMY_STAGGERED,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("stagger") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        input_ports: vec![
            Port::new(PORT_STAGGER_IN, PortKind::Input, SIGTYPE_COMBAT),
        ],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

fn make_pc_sm(id: SmId) -> SmDef {
    SmDef {
        id,
        states: [PC_IDLE, PC_PARRY, PC_RIPOSTE].into_iter().collect(),
        initial_state: PC_IDLE,
        transitions: vec![
            // Idle → Parry on player input
            Transition {
                id: TransitionId(id.0 * 100 + 0),
                source: PC_IDLE, target: PC_PARRY,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("parry_input") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
            // Parry → Riposte on ParrySuccess signal
            Transition {
                id: TransitionId(id.0 * 100 + 1),
                source: PC_PARRY, target: PC_RIPOSTE,
                priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("parry_success") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        input_ports: vec![
            Port::new(PORT_PARRY_SUCCESS_IN, PortKind::Input, SIGTYPE_COMBAT),
        ],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

/// Simulate the Interaction Rule evaluation (Phase 2 stub).
/// In the full implementation this runs inside phase2_evaluate against pre-transition states.
/// Here we call it manually BEFORE tick() on Tick N+1.
fn evaluate_interaction_rule_parry(world: &mut World, enemy_id: SmId, pc_id: SmId) {
    let enemy_state = world.instances[&enemy_id].active_state;
    let pc_state    = world.instances[&pc_id].active_state;

    // Rule: enemy=ActiveFrame AND pc=Parry → stagger enemy, parry success for PC
    if enemy_state == ENEMY_ACTIVEFRAME && pc_state == PC_PARRY {
        // Deliver StaggerIn to Enemy
        if let Some(i) = world.instances.get_mut(&enemy_id) {
            i.context.set("stagger", 1.0);
        }
        world.inject_signal(enemy_id, PORT_STAGGER_IN, combat_signal());

        // Deliver ParrySuccessIn to PC
        if let Some(i) = world.instances.get_mut(&pc_id) {
            i.context.set("parry_success", 1.0);
        }
        world.inject_signal(pc_id, PORT_PARRY_SUCCESS_IN, combat_signal());
    }
}

/// Core Appendix B scenario: 1-tick delay is structural and deterministic.
#[test]
fn test_parry_one_tick_delay() {
    let enemy_id = SmId(1);
    let pc_id    = SmId(2);

    let mut world = World::new();
    world.register_sm(make_enemy_sm(enemy_id));
    world.register_sm(make_pc_sm(pc_id));

    // ── Tick N ──────────────────────────────────────────────────────────────
    // Both transitions are triggered simultaneously.
    // Phase 2 evaluates PRE-transition states: Enemy=WindUp, PC=Idle → no IR match.
    if let Some(i) = world.instances.get_mut(&enemy_id) { i.context.set("timer_expired", 1.0); }
    if let Some(i) = world.instances.get_mut(&pc_id)    { i.context.set("parry_input",   1.0); }
    world.activate(enemy_id);
    world.activate(pc_id);

    tick(&mut world);

    // Post Tick N: both have advanced.
    assert_eq!(world.instances[&enemy_id].active_state, ENEMY_ACTIVEFRAME,
        "Tick N: Enemy→ActiveFrame");
    assert_eq!(world.instances[&pc_id].active_state, PC_PARRY,
        "Tick N: PC→Parry");

    // Interaction Rule did NOT fire this tick (evaluated against pre-transition states).
    // Neither stagger nor parry_success was delivered.
    assert_eq!(world.instances[&enemy_id].active_state, ENEMY_ACTIVEFRAME,
        "Enemy not yet Staggered after Tick N");
    assert_eq!(world.instances[&pc_id].active_state, PC_PARRY,
        "PC not yet Riposte after Tick N");

    // ── Tick N+1 ────────────────────────────────────────────────────────────
    // Interaction Rule now evaluates: Enemy=ActiveFrame, PC=Parry → MATCH.
    // (In full impl this is Phase 2; here we call the stub before tick().)
    evaluate_interaction_rule_parry(&mut world, enemy_id, pc_id);

    tick(&mut world);

    assert_eq!(world.instances[&enemy_id].active_state, ENEMY_STAGGERED,
        "Tick N+1: Enemy→Staggered");
    assert_eq!(world.instances[&pc_id].active_state, PC_RIPOSTE,
        "Tick N+1: PC→Riposte");
}

/// If the parry input arrives one tick too late (after enemy is already in ActiveFrame),
/// the player still gets the parry window on the next tick.
/// This is by design — Appendix B notes designers widen the Parry window to compensate.
#[test]
fn test_parry_late_input_still_works_next_tick() {
    let enemy_id = SmId(1);
    let pc_id    = SmId(2);

    let mut world = World::new();
    world.register_sm(make_enemy_sm(enemy_id));
    world.register_sm(make_pc_sm(pc_id));

    // Tick N: only enemy transitions; player hasn't pressed parry yet.
    if let Some(i) = world.instances.get_mut(&enemy_id) { i.context.set("timer_expired", 1.0); }
    world.activate(enemy_id);
    tick(&mut world);
    assert_eq!(world.instances[&enemy_id].active_state, ENEMY_ACTIVEFRAME);
    assert_eq!(world.instances[&pc_id].active_state,    PC_IDLE);

    // Tick N+1: player presses parry. IR sees Enemy=ActiveFrame, PC=Idle → no match yet.
    if let Some(i) = world.instances.get_mut(&pc_id) { i.context.set("parry_input", 1.0); }
    world.activate(pc_id);
    // IR stub: no match (PC still Idle at evaluation time)
    tick(&mut world);
    assert_eq!(world.instances[&pc_id].active_state, PC_PARRY, "PC→Parry");
    assert_eq!(world.instances[&enemy_id].active_state, ENEMY_ACTIVEFRAME, "Enemy still ActiveFrame");

    // Tick N+2: IR sees Enemy=ActiveFrame, PC=Parry → MATCH.
    evaluate_interaction_rule_parry(&mut world, enemy_id, pc_id);
    tick(&mut world);
    assert_eq!(world.instances[&enemy_id].active_state, ENEMY_STAGGERED, "Enemy staggered");
    assert_eq!(world.instances[&pc_id].active_state,    PC_RIPOSTE,      "PC riposte");
}

/// Guard: if neither condition matches, neither SM transitions.
#[test]
fn test_no_parry_no_match() {
    let enemy_id = SmId(1);
    let pc_id    = SmId(2);

    let mut world = World::new();
    world.register_sm(make_enemy_sm(enemy_id));
    world.register_sm(make_pc_sm(pc_id));

    // Only enemy advances to ActiveFrame; PC stays Idle.
    if let Some(i) = world.instances.get_mut(&enemy_id) { i.context.set("timer_expired", 1.0); }
    world.activate(enemy_id);
    tick(&mut world);

    // IR: Enemy=ActiveFrame, PC=Idle → no match.
    evaluate_interaction_rule_parry(&mut world, enemy_id, pc_id);
    // (No signals injected)

    tick(&mut world);
    // Enemy should NOT transition to Staggered (no stagger signal)
    assert_eq!(world.instances[&enemy_id].active_state, ENEMY_ACTIVEFRAME,
        "Enemy not staggered without Parry");
    assert_eq!(world.instances[&pc_id].active_state, PC_IDLE,
        "PC stays Idle");
}
