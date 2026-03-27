/// Elapse Function tests (§4.3).
///
/// Three capability levels:
///   Deterministic  — exact fast-forward (e.g. crop growth, weather cycle)
///   Approximate    — heuristic fast-forward (e.g. NPC patrol ignoring collisions)
///   NonElapsable   — falls back to Freeze (e.g. player combat SM, interactive dialogue)
///
/// Setup pattern: parent SM cycles between StateA and StateB.
/// Sub-SM has SuspendPolicy::Elapse + an elapse_fn.
/// While suspended (parent in StateA), ticks accumulate.
/// On re-entry to StateB, elapse_fn advances the sub-SM state.

use weaven_core::*;

const PARENT_SM: SmId = SmId(1);
const SUB_SM:    SmId = SmId(2);

const PARENT_A: StateId = StateId(0);
const PARENT_B: StateId = StateId(1);

// Sub-SM states: crop growth stages
const CROP_SEED:    StateId = StateId(10);
const CROP_SPROUT:  StateId = StateId(11);
const CROP_MATURE:  StateId = StateId(12);
const CROP_HARVEST: StateId = StateId(13);

const SIGTYPE: SignalTypeId = SignalTypeId(0);

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
                guard_expr: None,
                effects: vec![],
            },
            Transition {
                id: TransitionId(11),
                source: PARENT_B, target: PARENT_A, priority: 10,
                guard: Some(Box::new(|ctx, _| ctx.get("leave_b") > 0.0)),
                guard_expr: None,
                effects: vec![],
            },
        ],
        input_ports: vec![],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: ElapseCapabilityRt::NonElapsable,
        elapse_fn: None,
    }
}

/// Crop growth elapse function.
/// Every 5 ticks of suspension advances one growth stage.
fn crop_elapse_fn() -> ElapseFn {
    Box::new(|state: StateId, ctx: &Context, elapsed: u64| {
        let stages_to_advance = elapsed / 5;
        let current_stage = match state {
            s if s == CROP_SEED    => 0,
            s if s == CROP_SPROUT  => 1,
            s if s == CROP_MATURE  => 2,
            s if s == CROP_HARVEST => 3,
            _ => 0,
        };
        let new_stage = (current_stage + stages_to_advance as usize).min(3);
        let new_state = match new_stage {
            0 => CROP_SEED,
            1 => CROP_SPROUT,
            2 => CROP_MATURE,
            _ => CROP_HARVEST,
        };
        // Accumulate growth_days in context
        let mut new_ctx = ctx.clone();
        new_ctx.set("growth_days", elapsed as f64);
        (new_state, new_ctx)
    })
}

fn make_crop_sm(capability: ElapseCapabilityRt, elapse_fn: Option<ElapseFn>) -> SmDef {
    SmDef {
        id: SUB_SM,
        states: [CROP_SEED, CROP_SPROUT, CROP_MATURE, CROP_HARVEST].into_iter().collect(),
        initial_state: CROP_SEED,
        transitions: vec![],
        input_ports: vec![],
        output_ports: vec![],
        on_despawn_transitions: vec![],
        elapse_capability: capability,
        elapse_fn,
    }
}

fn enter_b(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("enter_b", 1.0); }
    world.activate(PARENT_SM);
    tick(world);
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("enter_b", 0.0); }
}

fn leave_b(world: &mut World) {
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("leave_b", 1.0); }
    world.activate(PARENT_SM);
    tick(world);
    if let Some(i) = world.instances.get_mut(&PARENT_SM) { i.context.set("leave_b", 0.0); }
}

// ── Deterministic Elapse ───────────────────────────────────────────────────

/// Deterministic elapse: crop advances exactly 2 stages after 10 ticks suspended.
/// Seed → (10 ticks / 5 = 2 stages) → Mature.
#[test]
fn test_deterministic_elapse_advances_stages() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    world.register_sm(make_crop_sm(ElapseCapabilityRt::Deterministic, Some(crop_elapse_fn())));
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B,
        parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Elapse,
        promoted_ports: vec![],
    });

    // Enter B: crop activates at Seed
    enter_b(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, CROP_SEED);

    // Exit B: crop frozen at Seed, tick=1
    leave_b(&mut world);
    let freeze_tick = world.tick;

    // Advance 10 ticks while suspended
    for _ in 0..10 {
        world.activate(PARENT_SM);
        tick(&mut world);
    }

    let elapsed = world.tick - freeze_tick;
    // enter_b itself advances one tick, so elapsed = 10 loop ticks + 1 = 11
    assert!(elapsed >= 10, "should have elapsed at least 10 ticks, got {elapsed}");

    // Re-enter B: elapse_fn applied → Seed + (elapsed/5 stages) ≥ Mature (2 stages)
    enter_b(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, CROP_MATURE,
        "crop should advance 2 stages (10+ suspended ticks / 5 per stage)");
    let growth_days = world.instances[&SUB_SM].context.get("growth_days");
    assert!(growth_days >= 10.0, "growth_days should be >= 10, got {growth_days}");
}

/// Deterministic elapse: 5 ticks → 1 stage advance (Seed → Sprout).
#[test]
fn test_deterministic_elapse_one_stage() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    world.register_sm(make_crop_sm(ElapseCapabilityRt::Deterministic, Some(crop_elapse_fn())));
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Elapse,
        promoted_ports: vec![],
    });

    enter_b(&mut world);
    leave_b(&mut world);

    for _ in 0..5 { world.activate(PARENT_SM); tick(&mut world); }

    enter_b(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, CROP_SPROUT,
        "5 ticks → Seed+1 stage = Sprout");
}

/// Elapse caps at max stage — no overflow past Harvest.
#[test]
fn test_deterministic_elapse_caps_at_max_stage() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    world.register_sm(make_crop_sm(ElapseCapabilityRt::Deterministic, Some(crop_elapse_fn())));
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Elapse,
        promoted_ports: vec![],
    });

    enter_b(&mut world);
    leave_b(&mut world);

    // 100 ticks = 20 stages — capped at Harvest (stage 3)
    for _ in 0..100 { world.activate(PARENT_SM); tick(&mut world); }

    enter_b(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, CROP_HARVEST,
        "elapse capped at Harvest regardless of elapsed ticks");
}

// ── NonElapsable → Freeze fallback ────────────────────────────────────────

/// NonElapsable SM with Elapse policy falls back to Freeze.
/// State is preserved exactly as-is (no progression during suspension).
#[test]
fn test_nonelapsable_falls_back_to_freeze() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    // NonElapsable + no elapse_fn → Freeze fallback
    world.register_sm(make_crop_sm(ElapseCapabilityRt::NonElapsable, None));
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Elapse,
        promoted_ports: vec![],
    });

    // Manually advance sub-SM to Sprout before suspending
    enter_b(&mut world);
    if let Some(i) = world.instances.get_mut(&SUB_SM) {
        i.active_state = CROP_SPROUT;
    }

    leave_b(&mut world);

    // 50 ticks pass — but NonElapsable means no progression
    for _ in 0..50 { world.activate(PARENT_SM); tick(&mut world); }

    enter_b(&mut world);
    // Should resume at Sprout (frozen state), not Harvest
    assert_eq!(world.instances[&SUB_SM].active_state, CROP_SPROUT,
        "NonElapsable falls back to Freeze — state preserved without progression");
}

// ── Approximate Elapse ─────────────────────────────────────────────────────

/// Approximate elapse: same function as Deterministic but declared as heuristic.
/// Framework doesn't validate accuracy — it's a designer contract.
#[test]
fn test_approximate_elapse_uses_provided_function() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    world.register_sm(make_crop_sm(ElapseCapabilityRt::Approximate, Some(crop_elapse_fn())));
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Elapse,
        promoted_ports: vec![],
    });

    enter_b(&mut world);
    leave_b(&mut world);
    for _ in 0..15 { world.activate(PARENT_SM); tick(&mut world); }

    enter_b(&mut world);
    // 15 ticks / 5 = 3 stages: Seed → Harvest
    assert_eq!(world.instances[&SUB_SM].active_state, CROP_HARVEST,
        "Approximate elapse function executes same as Deterministic");
}

// ── Zero elapsed ticks ─────────────────────────────────────────────────────

/// If parent immediately re-enters (0 ticks elapsed), elapse_fn is called with 0.
/// Crop should stay at Seed (0/5 = 0 stages).
#[test]
fn test_elapse_zero_ticks_no_change() {
    let mut world = World::new();
    world.register_sm(make_parent_sm());
    world.register_sm(make_crop_sm(ElapseCapabilityRt::Deterministic, Some(crop_elapse_fn())));
    world.register_compound(CompoundStateDef {
        parent_state: PARENT_B, parent_sm: PARENT_SM,
        sub_machines: vec![SUB_SM],
        suspend_policy: SuspendPolicyRt::Elapse,
        promoted_ports: vec![],
    });

    enter_b(&mut world);
    leave_b(&mut world);
    // Immediately re-enter — 0 ticks elapsed (leave_b and enter_b happen same round)
    enter_b(&mut world);
    assert_eq!(world.instances[&SUB_SM].active_state, CROP_SEED,
        "0 elapsed ticks: crop stays at Seed");
}
