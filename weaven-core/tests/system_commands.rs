/// System Commands tests (§7.3).
///
/// HitStop:    pause tick advancement for N frames
/// SlowMotion: reduce tick rate by factor for duration_ticks ticks
/// TimeScale:  adjust global time delta

use weaven_core::*;

const SM_A: SmId     = SmId(1);
const S0:   StateId  = StateId(0);
const S1:   StateId  = StateId(1);
const S2:   StateId  = StateId(2);
const SIGTYPE: SignalTypeId = SignalTypeId(0);

fn make_sm_with_cmd_effect(id: SmId, cmd: SystemCommand) -> SmDef {
    SmDef {
        id,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(id.0 * 10),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("fire") > 0.0)),
            guard_expr: None,
            effects: vec![Box::new(move |_ctx| vec![EffectOutput::Cmd(cmd.clone())])],
        }],
        input_ports: vec![],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

// ── HitStop ────────────────────────────────────────────────────────────────

/// Transition effect emits HitStop → TickOutput contains it, world.hit_stop_frames set.
#[test]
fn test_hitstop_emitted_in_tick_output() {
    let mut world = World::new();
    world.register_sm(make_sm_with_cmd_effect(SM_A, SystemCommand::HitStop { frames: 3 }));

    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("fire", 1.0); }
    world.activate(SM_A);
    let out = tick(&mut world);

    assert_eq!(world.instances[&SM_A].active_state, S1, "transition fired");
    assert_eq!(world.hit_stop_frames, 2,  // decremented once in phase6
        "hit_stop_frames should be 3-1=2 after first tick");
    assert!(out.system_commands.iter().any(|c| matches!(c, SystemCommand::HitStop { frames: 3 })),
        "HitStop should appear in TickOutput");
}

/// HitStop accumulates across multiple simultaneous commands.
#[test]
fn test_hitstop_accumulates() {
    let sm_b = SmId(2);
    let mut world = World::new();
    world.register_sm(make_sm_with_cmd_effect(SM_A, SystemCommand::HitStop { frames: 2 }));
    world.register_sm(make_sm_with_cmd_effect(sm_b, SystemCommand::HitStop { frames: 3 }));

    for id in [SM_A, sm_b] {
        if let Some(i) = world.instances.get_mut(&id) { i.context.set("fire", 1.0); }
        world.activate(id);
    }
    tick(&mut world);

    // Both fire same tick: 2+3=5, then -1 = 4 remaining
    assert_eq!(world.hit_stop_frames, 4, "HitStop frames accumulate: 2+3-1=4");
}

/// HitStop counts down each tick.
#[test]
fn test_hitstop_counts_down() {
    let mut world = World::new();
    world.register_sm(make_sm_with_cmd_effect(SM_A, SystemCommand::HitStop { frames: 3 }));

    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("fire", 1.0); }
    world.activate(SM_A);
    tick(&mut world); // fires transition, hit_stop = 3-1 = 2

    assert_eq!(world.hit_stop_frames, 2);
    world.activate(SM_A); tick(&mut world); // hit_stop = 2-1 = 1
    assert_eq!(world.hit_stop_frames, 1);
    world.activate(SM_A); tick(&mut world); // hit_stop = 1-1 = 0
    assert_eq!(world.hit_stop_frames, 0, "HitStop cleared after countdown");
}

// ── SlowMotion ─────────────────────────────────────────────────────────────

/// SlowMotion sets factor and duration, counts down each tick, restores factor=1.0 on expiry.
#[test]
fn test_slow_motion_applied_and_expires() {
    let mut world = World::new();
    world.register_sm(make_sm_with_cmd_effect(
        SM_A,
        SystemCommand::SlowMotion { factor: 0.5, duration_ticks: 3 },
    ));

    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("fire", 1.0); }
    world.activate(SM_A);
    let out = tick(&mut world); // fires, slow_motion_remaining = 3-1 = 2

    assert_eq!(world.slow_motion_factor, 0.5, "slow motion factor set");
    assert_eq!(world.slow_motion_remaining, 2, "remaining = 3-1 = 2");
    assert!(out.system_commands.iter().any(|c| matches!(c,
        SystemCommand::SlowMotion { factor, duration_ticks: 3 } if *factor == 0.5
    )), "SlowMotion in TickOutput");

    world.activate(SM_A); tick(&mut world); // remaining = 2-1 = 1
    assert_eq!(world.slow_motion_remaining, 1);
    assert_eq!(world.slow_motion_factor, 0.5);

    world.activate(SM_A); tick(&mut world); // remaining = 1-1 = 0 → expired
    assert_eq!(world.slow_motion_remaining, 0);
    assert_eq!(world.slow_motion_factor, 1.0, "slow motion factor restored on expiry");
}

/// Multiple SlowMotion commands: last one wins (most recent overrides).
#[test]
fn test_slow_motion_override() {
    let sm_b = SmId(2);
    let mut world = World::new();
    world.register_sm(make_sm_with_cmd_effect(
        SM_A, SystemCommand::SlowMotion { factor: 0.5, duration_ticks: 10 },
    ));
    world.register_sm(make_sm_with_cmd_effect(
        sm_b, SystemCommand::SlowMotion { factor: 0.25, duration_ticks: 5 },
    ));

    for id in [SM_A, sm_b] {
        if let Some(i) = world.instances.get_mut(&id) { i.context.set("fire", 1.0); }
        world.activate(id);
    }
    tick(&mut world);

    // Both fire same tick; last command applied in Phase 6 order wins.
    // SmId ordering: SM_A(1) fires first, sm_b(2) fires second → sm_b's cmd is last.
    assert_eq!(world.slow_motion_factor, 0.25, "last SlowMotion wins");
    assert_eq!(world.slow_motion_remaining, 4, "sm_b duration=5, -1=4");
}

// ── TimeScale ──────────────────────────────────────────────────────────────

/// TimeScale is persistent — adjusts world.time_scale until explicitly changed.
#[test]
fn test_timescale_persists() {
    let mut world = World::new();
    assert_eq!(world.time_scale, 1.0, "initial time_scale = 1.0");

    world.register_sm(make_sm_with_cmd_effect(SM_A, SystemCommand::TimeScale(0.5)));

    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("fire", 1.0); }
    world.activate(SM_A);
    let out = tick(&mut world);

    assert_eq!(world.time_scale, 0.5, "time_scale updated to 0.5");
    assert!(out.system_commands.iter().any(|c| matches!(c, SystemCommand::TimeScale(s) if *s == 0.5)),
        "TimeScale in TickOutput");

    // TimeScale persists across subsequent ticks
    world.activate(SM_A); tick(&mut world);
    assert_eq!(world.time_scale, 0.5, "time_scale persists");
}

/// TimeScale can be reset to 1.0.
#[test]
fn test_timescale_reset() {
    let sm_b = SmId(2);
    let mut world = World::new();
    world.register_sm(make_sm_with_cmd_effect(SM_A, SystemCommand::TimeScale(0.1)));
    world.register_sm(make_sm_with_cmd_effect(sm_b, SystemCommand::TimeScale(1.0)));

    // First fire SM_A to set 0.1
    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("fire", 1.0); }
    world.activate(SM_A);
    tick(&mut world);
    assert_eq!(world.time_scale, 0.1);

    // Then fire sm_b to reset to 1.0
    if let Some(i) = world.instances.get_mut(&sm_b) { i.context.set("fire", 1.0); }
    world.activate(sm_b);
    tick(&mut world);
    assert_eq!(world.time_scale, 1.0, "time_scale reset to normal");
}

// ── Mixed commands + signals same effect ───────────────────────────────────

/// Effect emits both a Signal and a SystemCommand in the same transition.
#[test]
fn test_effect_emits_signal_and_command() {
    let sm_b = SmId(2);
    let mut world = World::new();

    // SM_A emits a signal to SM_B AND a HitStop command
    world.register_sm(SmDef {
        id: SM_A,
        states: [S0, S1, S2].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(10),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("fire") > 0.0)),
            guard_expr: None,
            effects: vec![Box::new(|_ctx| {
                let mut p = std::collections::BTreeMap::new();
                p.insert("hit".to_string(), 1.0);
                vec![
                    EffectOutput::Signal(PortId(0), Signal {
                        signal_type: SIGTYPE,
                        payload: p,
                    }),
                    EffectOutput::Cmd(SystemCommand::HitStop { frames: 2 }),
                ]
            })],
        }],
        input_ports: vec![],
        output_ports: vec![Port::new(PortId(0), PortKind::Output, SIGTYPE)],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });

    world.register_sm(SmDef {
        id: sm_b,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(20),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("hit") > 0.0)),
            guard_expr: None,
            effects: vec![],
        }],
        input_ports: vec![Port::new(PortId(1), PortKind::Input, SIGTYPE)],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });

    world.connect(Connection {
        id: ConnectionId(1),
        source_sm: SM_A, source_port: PortId(0),
        target_sm: sm_b, target_port: PortId(1),
        delay_ticks: 0,
        pipeline: vec![],
    });

    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("fire", 1.0); }
    world.activate(SM_A);
    let out = tick(&mut world);

    // Signal cascaded to SM_B
    assert_eq!(world.instances[&sm_b].active_state, S1, "SM_B received signal");
    // HitStop applied
    assert_eq!(world.hit_stop_frames, 1, "HitStop: 2-1=1 remaining");
    assert!(out.system_commands.iter().any(|c| matches!(c, SystemCommand::HitStop { .. })),
        "HitStop in TickOutput");
}
