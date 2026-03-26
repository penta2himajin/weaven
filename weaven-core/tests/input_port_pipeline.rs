/// Input-Port-side pipeline tests (§6.2, §6.3 steps 4–6).
///
/// The pipeline ordering per §6.3:
///   1-3. Connection-side: Transform → Filter → Redirect (tested in fire_propagation.rs)
///   4-6. Input-Port-side: Transform → Filter → Redirect  ← this file
///
/// Entity-specific rules (immunity, absorption) live in the Input-Port pipeline.
/// World rules (type effectiveness, distance attenuation) live in the Connection pipeline.

use weaven_core::*;

const S0: StateId = StateId(0);
const S1: StateId = StateId(1);
const PORT_IN:  PortId = PortId(0);
const PORT_OUT: PortId = PortId(1);
const SIGTYPE: SignalTypeId = SignalTypeId(0);

fn fire_signal(intensity: f64) -> Signal {
    let mut p = std::collections::BTreeMap::new();
    p.insert("intensity".to_string(), intensity);
    Signal { signal_type: SIGTYPE, payload: p }
}

fn make_sm_with_input_pipeline(id: SmId, input_pipeline: Vec<PipelineStep>) -> SmDef {
    SmDef {
        id,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(id.0 * 10),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("intensity") > 0.0)),
            effects: vec![],
        }],
        input_ports: vec![Port {
            id: PORT_IN,
            kind: PortKind::Input,
            signal_type: SIGTYPE,
            input_pipeline,
            influence_radius: None,
        }],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

// ── Input-Port Transform ────────────────────────────────────────────────────

/// Input-Port-side Transform: halve intensity before guard sees it.
/// Simulates an entity-level damage reduction (e.g. armor).
#[test]
fn test_input_port_transform_halves_intensity() {
    let src = SmId(1);
    let tgt = SmId(2);
    let mut world = World::new();

    // Source SM that emits a signal
    world.register_sm(SmDef {
        id: src,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(10),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("fire") > 0.0)),
            effects: vec![Box::new(|_| {
                let mut p = std::collections::BTreeMap::new();
                p.insert("intensity".to_string(), 10.0);
                vec![EffectOutput::Signal(PORT_OUT, Signal { signal_type: SIGTYPE, payload: p })]
            })],
        }],
        input_ports: vec![],
        output_ports: vec![Port::new(PORT_OUT, PortKind::Output, SIGTYPE)],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });

    // Target SM: Input-Port Transform halves intensity (entity-level armor)
    world.register_sm(make_sm_with_input_pipeline(tgt, vec![
        PipelineStep::Transform(Box::new(|mut sig| {
            let v = sig.payload.get("intensity").copied().unwrap_or(0.0);
            sig.payload.insert("intensity".to_string(), v * 0.5);
            sig
        })),
    ]));

    world.connect(Connection {
        id: ConnectionId(1),
        source_sm: src, source_port: PORT_OUT,
        target_sm: tgt, target_port: PORT_IN,
        delay_ticks: 0,
        pipeline: vec![],
    });

    // Activate source
    if let Some(i) = world.instances.get_mut(&src) { i.context.set("fire", 1.0); }
    world.activate(src);
    tick(&mut world);

    // Target received intensity 5.0 (10 * 0.5) — guard fires (5 > 0)
    assert_eq!(world.instances[&tgt].active_state, S1, "target should transition");
    assert_eq!(world.instances[&tgt].context.get("intensity"), 5.0,
        "intensity should be halved by Input-Port Transform");
}

// ── Input-Port Filter (immunity) ────────────────────────────────────────────

/// Input-Port-side Filter: entity immune to fire signals.
/// The signal is blocked at the entity boundary — guard never sees it.
#[test]
fn test_input_port_filter_immunity() {
    let src = SmId(1);
    let tgt = SmId(2);
    let mut world = World::new();

    world.register_sm(SmDef {
        id: src,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(10),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("fire") > 0.0)),
            effects: vec![Box::new(|_| {
                let mut p = std::collections::BTreeMap::new();
                p.insert("intensity".to_string(), 5.0);
                vec![EffectOutput::Signal(PORT_OUT, Signal { signal_type: SIGTYPE, payload: p })]
            })],
        }],
        input_ports: vec![],
        output_ports: vec![Port::new(PORT_OUT, PortKind::Output, SIGTYPE)],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });

    // Target has fire immunity — Input-Port Filter blocks all signals
    world.register_sm(make_sm_with_input_pipeline(tgt, vec![
        PipelineStep::Filter(Box::new(|_sig| false)), // immune: block everything
    ]));

    world.connect(Connection {
        id: ConnectionId(1),
        source_sm: src, source_port: PORT_OUT,
        target_sm: tgt, target_port: PORT_IN,
        delay_ticks: 0,
        pipeline: vec![],
    });

    if let Some(i) = world.instances.get_mut(&src) { i.context.set("fire", 1.0); }
    world.activate(src);
    tick(&mut world);

    // Source transitioned but target blocked — stays S0
    assert_eq!(world.instances[&src].active_state, S1, "source transitioned");
    assert_eq!(world.instances[&tgt].active_state, S0,
        "target immune — signal blocked by Input-Port Filter");
}

// ── Input-Port Redirect ─────────────────────────────────────────────────────

const PORT_ALT_IN: PortId = PortId(2);
const S_ALT: StateId = StateId(2);

/// Input-Port-side Redirect: blocked signal routed to an alternate port.
/// Simulates "Flash Fire" ability: fire immunity + triggers a power-up.
#[test]
fn test_input_port_redirect_flash_fire() {
    let src = SmId(1);
    let tgt = SmId(2);
    let mut world = World::new();

    world.register_sm(SmDef {
        id: src,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(10),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("fire") > 0.0)),
            effects: vec![Box::new(|_| {
                let mut p = std::collections::BTreeMap::new();
                p.insert("intensity".to_string(), 5.0);
                vec![EffectOutput::Signal(PORT_OUT, Signal { signal_type: SIGTYPE, payload: p })]
            })],
        }],
        input_ports: vec![],
        output_ports: vec![Port::new(PORT_OUT, PortKind::Output, SIGTYPE)],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });

    // Target: Flash Fire ability
    // Input pipeline: Filter blocks fire → Redirect to PORT_ALT_IN (power-up trigger)
    world.register_sm(SmDef {
        id: tgt,
        states: [S0, S1, S_ALT].into_iter().collect(),
        initial_state: S0,
        transitions: vec![
            // Normal damage path (blocked by filter)
            Transition {
                id: TransitionId(20),
                source: S0, target: S1, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("intensity") > 0.0)),
                effects: vec![],
            },
            // Power-up path (triggered by redirect)
            Transition {
                id: TransitionId(21),
                source: S0, target: S_ALT, priority: 20,
                guard: Some(Box::new(|ctx, _| ctx.get("boosted") > 0.0)),
                effects: vec![],
            },
        ],
        input_ports: vec![
            Port {
                id: PORT_IN,
                kind: PortKind::Input,
                signal_type: SIGTYPE,
                input_pipeline: vec![
                    PipelineStep::Filter(Box::new(|_| false)),      // block fire
                    PipelineStep::Redirect(PORT_ALT_IN),            // redirect to power-up
                ],
                influence_radius: None,
            },
            Port::new(PORT_ALT_IN, PortKind::Input, SIGTYPE),
        ],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });

    world.connect(Connection {
        id: ConnectionId(1),
        source_sm: src, source_port: PORT_OUT,
        target_sm: tgt, target_port: PORT_IN,
        delay_ticks: 0,
        pipeline: vec![],
    });

    if let Some(i) = world.instances.get_mut(&src) { i.context.set("fire", 1.0); }
    world.activate(src);

    // Pre-set boosted context so the alt transition's guard fires when redirect arrives
    if let Some(i) = world.instances.get_mut(&tgt) { i.context.set("boosted", 1.0); }
    tick(&mut world);

    // Target should go to S_ALT (power-up) not S1 (damage)
    assert_eq!(world.instances[&tgt].active_state, S_ALT,
        "Flash Fire: fire blocked, power-up triggered via redirect");
}

// ── Connection + Input-Port pipeline chaining (§6.3 full ordering) ──────────

/// Full §6.3 pipeline: Connection-side Transform then Input-Port-side Transform.
/// Both reduce intensity; the target only fires if the final value is positive.
#[test]
fn test_pipeline_full_ordering_connection_then_input_port() {
    let src = SmId(1);
    let tgt = SmId(2);
    let mut world = World::new();

    world.register_sm(SmDef {
        id: src,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(10),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("fire") > 0.0)),
            effects: vec![Box::new(|_| {
                let mut p = std::collections::BTreeMap::new();
                p.insert("intensity".to_string(), 4.0);
                vec![EffectOutput::Signal(PORT_OUT, Signal { signal_type: SIGTYPE, payload: p })]
            })],
        }],
        input_ports: vec![],
        output_ports: vec![Port::new(PORT_OUT, PortKind::Output, SIGTYPE)],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });

    // Connection-side: -1 (world rule)
    // Input-Port-side: -2 (entity rule)
    // 4 - 1 - 2 = 1 > 0 → guard fires
    world.register_sm(make_sm_with_input_pipeline(tgt, vec![
        PipelineStep::Transform(Box::new(|mut sig| {
            let v = sig.payload.get("intensity").copied().unwrap_or(0.0);
            sig.payload.insert("intensity".to_string(), v - 2.0);
            sig
        })),
    ]));

    world.connect(Connection {
        id: ConnectionId(1),
        source_sm: src, source_port: PORT_OUT,
        target_sm: tgt, target_port: PORT_IN,
        delay_ticks: 0,
        pipeline: vec![
            PipelineStep::Transform(Box::new(|mut sig| {
                let v = sig.payload.get("intensity").copied().unwrap_or(0.0);
                sig.payload.insert("intensity".to_string(), v - 1.0);
                sig
            })),
        ],
    });

    if let Some(i) = world.instances.get_mut(&src) { i.context.set("fire", 1.0); }
    world.activate(src);
    tick(&mut world);

    assert_eq!(world.instances[&tgt].active_state, S1, "target transitions (intensity 1 > 0)");
    assert_eq!(world.instances[&tgt].context.get("intensity"), 1.0,
        "4 - 1 (conn) - 2 (port) = 1");
}

/// When chained transforms reduce intensity to 0, Input-Port Filter blocks the signal.
#[test]
fn test_pipeline_chain_exhausts_and_blocks() {
    let src = SmId(1);
    let tgt = SmId(2);
    let mut world = World::new();

    world.register_sm(SmDef {
        id: src,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(10),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("fire") > 0.0)),
            effects: vec![Box::new(|_| {
                let mut p = std::collections::BTreeMap::new();
                p.insert("intensity".to_string(), 3.0);
                vec![EffectOutput::Signal(PORT_OUT, Signal { signal_type: SIGTYPE, payload: p })]
            })],
        }],
        input_ports: vec![],
        output_ports: vec![Port::new(PORT_OUT, PortKind::Output, SIGTYPE)],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });

    // Connection: -1, Input-Port: -2, Filter: blocks if <= 0
    // 3 - 1 - 2 = 0 → filter blocks
    world.register_sm(SmDef {
        id: tgt,
        states: [S0, S1].into_iter().collect(),
        initial_state: S0,
        transitions: vec![Transition {
            id: TransitionId(20),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("intensity") > 0.0)),
            effects: vec![],
        }],
        input_ports: vec![Port {
            id: PORT_IN,
            kind: PortKind::Input,
            signal_type: SIGTYPE,
            input_pipeline: vec![
                PipelineStep::Transform(Box::new(|mut sig| {
                    let v = sig.payload.get("intensity").copied().unwrap_or(0.0);
                    sig.payload.insert("intensity".to_string(), v - 2.0);
                    sig
                })),
                PipelineStep::Filter(Box::new(|sig| {
                    sig.payload.get("intensity").copied().unwrap_or(0.0) > 0.0
                })),
            ],
            influence_radius: None,
        }],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: weaven_core::ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    });

    world.connect(Connection {
        id: ConnectionId(1),
        source_sm: src, source_port: PORT_OUT,
        target_sm: tgt, target_port: PORT_IN,
        delay_ticks: 0,
        pipeline: vec![
            PipelineStep::Transform(Box::new(|mut sig| {
                let v = sig.payload.get("intensity").copied().unwrap_or(0.0);
                sig.payload.insert("intensity".to_string(), v - 1.0);
                sig
            })),
        ],
    });

    if let Some(i) = world.instances.get_mut(&src) { i.context.set("fire", 1.0); }
    world.activate(src);
    tick(&mut world);

    assert_eq!(world.instances[&src].active_state, S1, "source fires");
    assert_eq!(world.instances[&tgt].active_state, S0,
        "target blocked — 3-1-2=0, Input-Port Filter rejects");
}
