/// Continuous Input/Output Port binding tests (§2.4.3, §2.4.4).
///
/// Continuous Input:  external values pulled into SM context each Phase 1.
/// Continuous Output: selected context fields published in TickOutput each Phase 6.

use weaven_core::*;
use std::sync::{Arc, Mutex};

const SM_A: SmId     = SmId(1);
const S0:   StateId  = StateId(0);
const S1:   StateId  = StateId(1);
const SIGTYPE: SignalTypeId = SignalTypeId(0);

fn make_speed_sm(id: SmId) -> SmDef {
    // SM that transitions S0→S1 when speed > 5.0 (read from continuous input)
    SmDef::new(
        id,
        [S0, S1],
        S0,
        vec![Transition {
            id: TransitionId(id.0 * 10),
            source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("speed") > 5.0)),
            effects: vec![],
        }],
        vec![], vec![],
    )
}

// ── Continuous Input Port ──────────────────────────────────────────────────

/// Phase 1 writes external value into SM context each tick.
#[test]
fn test_continuous_input_updates_context_each_tick() {
    let mut world = World::new();
    world.register_sm(make_speed_sm(SM_A));

    // Bind: external "speed" sensor → SM context field "speed"
    let speed = Arc::new(Mutex::new(3.0f64));
    let speed_clone = Arc::clone(&speed);
    world.bind_continuous_input(SM_A, "speed", move || *speed_clone.lock().unwrap());

    // Tick 1: speed=3 → guard false → stays S0
    tick(&mut world);
    assert_eq!(world.instances[&SM_A].active_state, S0, "speed=3, guard false");
    assert_eq!(world.instances[&SM_A].context.get("speed"), 3.0, "context updated");

    // Tick 2: speed=8 → guard true → S1
    *speed.lock().unwrap() = 8.0;
    tick(&mut world);
    assert_eq!(world.instances[&SM_A].active_state, S1, "speed=8, guard true");
    assert_eq!(world.instances[&SM_A].context.get("speed"), 8.0, "context updated");
}

/// Multiple continuous input bindings on the same SM (e.g. velocity.x, velocity.y).
#[test]
fn test_multiple_continuous_inputs_same_sm() {
    let mut world = World::new();
    world.register_sm(SmDef::new(
        SM_A,
        [S0, S1],
        S0,
        vec![Transition {
            id: TransitionId(10), source: S0, target: S1, priority: 10,
            // fires when magnitude² > 25 (i.e. |v| > 5)
            guard: Some(Box::new(|ctx, _| {
                ctx.get("vel_x") * ctx.get("vel_x") + ctx.get("vel_y") * ctx.get("vel_y") > 25.0
            })),
            effects: vec![],
        }],
        vec![], vec![],
    ));

    let vx = Arc::new(Mutex::new(0.0f64));
    let vy = Arc::new(Mutex::new(0.0f64));
    let vx2 = Arc::clone(&vx);
    let vy2 = Arc::clone(&vy);
    world.bind_continuous_input(SM_A, "vel_x", move || *vx2.lock().unwrap());
    world.bind_continuous_input(SM_A, "vel_y", move || *vy2.lock().unwrap());

    // vx=3, vy=4 → 9+16=25 → not > 25 → stays S0
    *vx.lock().unwrap() = 3.0;
    *vy.lock().unwrap() = 4.0;
    tick(&mut world);
    assert_eq!(world.instances[&SM_A].active_state, S0, "magnitude=5, guard false");

    // vx=4, vy=4 → 16+16=32 > 25 → S1
    *vx.lock().unwrap() = 4.0;
    tick(&mut world);
    assert_eq!(world.instances[&SM_A].active_state, S1, "magnitude>5, guard true");
}

/// Continuous input wakes dormant SM when value changes significantly.
#[test]
fn test_continuous_input_wakes_dormant_sm() {
    let mut world = World::new();
    world.register_sm(make_speed_sm(SM_A));

    let speed = Arc::new(Mutex::new(0.0f64));
    let speed_clone = Arc::clone(&speed);
    world.bind_continuous_input(SM_A, "speed", move || *speed_clone.lock().unwrap());

    // Run a tick so SM becomes dormant (no transition fires)
    tick(&mut world);
    world.active_set.remove(&SM_A); // force dormant

    // Change speed significantly → SM should re-enter Active Set in next Phase 1
    *speed.lock().unwrap() = 10.0;
    tick(&mut world);
    assert_eq!(world.instances[&SM_A].active_state, S1,
        "dormant SM woken by continuous input change, guard fired");
}

/// Continuous input preserves Guard purity (§2.4.3): guard reads context,
/// unaware the value originates externally.
#[test]
fn test_continuous_input_preserves_guard_purity() {
    let mut world = World::new();
    world.register_sm(make_speed_sm(SM_A));

    // Guard reads context.speed — does NOT care it came from a binding.
    // The binding writes the value in Phase 1; Phase 2 reads it as a plain context field.
    world.bind_continuous_input(SM_A, "speed", || 9.0); // constant 9.0

    tick(&mut world);
    // Guard: context.speed=9.0 > 5.0 → fires
    assert_eq!(world.instances[&SM_A].active_state, S1,
        "guard reads context.speed=9 regardless of its external origin");
}

// ── Continuous Output Port ─────────────────────────────────────────────────

/// Phase 6 publishes declared context fields in TickOutput.continuous_outputs.
#[test]
fn test_continuous_output_published_in_tick_output() {
    let mut world = World::new();
    world.register_sm(SmDef::new(SM_A, [S0], S0, vec![], vec![], vec![]));

    // Set context values
    if let Some(i) = world.instances.get_mut(&SM_A) {
        i.context.set("hp", 75.0);
        i.context.set("mana", 40.0);
        i.context.set("internal_flag", 99.0); // NOT declared → should not be exposed
    }

    // Declare: expose hp and mana, NOT internal_flag
    world.declare_continuous_output(SM_A, vec!["hp".to_string(), "mana".to_string()]);

    world.activate(SM_A);
    let out = tick(&mut world);

    let fields = out.continuous_outputs.get(&SM_A).expect("outputs for SM_A");
    assert_eq!(fields.get("hp"),   Some(&75.0), "hp exposed");
    assert_eq!(fields.get("mana"), Some(&40.0), "mana exposed");
    assert!(!fields.contains_key("internal_flag"), "internal_flag NOT exposed");
}

/// Multiple SMs declare continuous outputs independently.
#[test]
fn test_multiple_sms_continuous_output() {
    let sm_b = SmId(2);
    let mut world = World::new();
    world.register_sm(SmDef::new(SM_A, [S0], S0, vec![], vec![], vec![]));
    world.register_sm(SmDef::new(sm_b, [S0], S0, vec![], vec![], vec![]));

    if let Some(i) = world.instances.get_mut(&SM_A) { i.context.set("x", 10.0); }
    if let Some(i) = world.instances.get_mut(&sm_b) { i.context.set("y", 20.0); }

    world.declare_continuous_output(SM_A, vec!["x".to_string()]);
    world.declare_continuous_output(sm_b, vec!["y".to_string()]);

    world.activate(SM_A);
    world.activate(sm_b);
    let out = tick(&mut world);

    assert_eq!(out.continuous_outputs[&SM_A]["x"], 10.0);
    assert_eq!(out.continuous_outputs[&sm_b]["y"], 20.0);
}

/// Continuous output reflects the state AFTER Phase 3 (post-transition).
/// If a transition mutates context, the output shows the new value.
#[test]
fn test_continuous_output_reflects_post_transition_context() {
    let mut world = World::new();
    world.register_sm(SmDef::new(
        SM_A,
        [S0, S1],
        S0,
        vec![Transition {
            id: TransitionId(10), source: S0, target: S1, priority: 10,
            guard: Some(Box::new(|ctx, _| ctx.get("trigger") > 0.0)),
            effects: vec![Box::new(|ctx| {
                ctx.set("score", 100.0);
                vec![]
            })],
        }],
        vec![], vec![],
    ));

    world.declare_continuous_output(SM_A, vec!["score".to_string()]);
    if let Some(i) = world.instances.get_mut(&SM_A) {
        i.context.set("trigger", 1.0);
        i.context.set("score", 0.0);
    }
    world.activate(SM_A);
    let out = tick(&mut world);

    // Transition fired, SetContext effect set score=100
    assert_eq!(out.continuous_outputs[&SM_A]["score"], 100.0,
        "output shows post-transition context value");
}

// ── Bevy Adapter integration ───────────────────────────────────────────────

/// push_continuous_input (Bevy Adapter) equivalent: direct context write before tick.
/// Verifies the Adapter pattern works without bind_continuous_input.
#[test]
fn test_adapter_push_pattern() {
    let mut world = World::new();
    world.register_sm(make_speed_sm(SM_A));

    // Adapter pushes external values directly (alternative to bind_continuous_input)
    if let Some(i) = world.instances.get_mut(&SM_A) {
        i.context.set("speed", 7.0);
    }
    world.activate(SM_A);
    tick(&mut world);
    assert_eq!(world.instances[&SM_A].active_state, S1, "speed=7>5 → S1");
}
