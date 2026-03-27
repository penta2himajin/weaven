/// Weaven tick lifecycle implementation.
///
/// Six strictly ordered phases per tick (§3):
///   1. Input   — update continuous ports, apply player input
///   2. Evaluate — pure read: determine which transitions fire, which IR signals to enqueue
///   3. Execute  — simultaneously fire all determined transitions, emit signals
///   4. Propagate — cascade signal delivery until queue empty or max depth
///   5. Lifecycle — spawn/despawn processing
///   6. Output   — push to continuous output ports, emit network diffs

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::types::*;
use crate::trace::{TraceCollector, TraceEvent, Phase as TracePhase};

// ---------------------------------------------------------------------------
// Phase 2 result: which transition fires in each SM
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct EvaluationResult {
    /// For each SM in the Active Set, the highest-priority Transition to fire.
    decisions: BTreeMap<SmId, Option<TransitionId>>,
    /// Signals to deliver from Interaction Rule matches (Phase 2 only — §2.7).
    ir_signals: Vec<IrSignal>,
}

// ---------------------------------------------------------------------------
// Public tick entry point
// ---------------------------------------------------------------------------

/// Advance the world by one tick.
///
/// All six phases execute to completion before returning.
/// Returns a summary of what changed (for the output/network layer).
pub fn tick(world: &mut World) -> TickOutput {
    world.tick += 1;
    let mut tc = TraceCollector::new();

    // Snapshot pre-tick states for diff computation in Phase 6.
    let pre_tick_states: BTreeMap<SmId, StateId> = world
        .instances
        .iter()
        .map(|(&id, inst)| (id, inst.active_state))
        .collect();

    // Phase 1: Input — stub (Adapter pushes values before calling tick)
    phase1_input(world);

    // Phase 2: Evaluate — pure read, determine firing decisions
    let mut eval = phase2_evaluate(world, &mut tc);

    // Phase 3: Execute — fire transitions simultaneously, enqueue emitted signals
    // Returns the set of SMs that actually fired a transition.
    let mut fired_this_tick = phase3_execute(world, &mut eval, &mut tc);

    // Phase 4: Propagate — cascade signal delivery
    let mut diag = crate::error::TickDiagnostics::default();
    phase4_propagate(world, &mut fired_this_tick, &mut diag, &mut tc);

    // Phase 5: Lifecycle — spawn/despawn
    let mut pre_lifecycle_queue: std::collections::VecDeque<QueuedSignal> =
        std::mem::take(&mut world.signal_queue);

    phase5_lifecycle(world, &mut diag);

    // Phase 5d: Cascade from despawn batch signals (§3 Phase 5d).
    if !world.signal_queue.is_empty() {
        phase4_propagate(world, &mut fired_this_tick, &mut diag, &mut tc);
    }

    // Restore deferred signals, purging any targeting SM instances destroyed in Phase 5.
    // These are stale in-flight signals (§11.5).
    pre_lifecycle_queue.retain(|qs| {
        let alive = world.instances.contains_key(&qs.target_sm);
        if !alive {
            diag.push(crate::error::WeavenDiagnostic::StaleSignal {
                target_sm:   qs.target_sm,
                source_conn: qs.source_conn,
            });
        }
        alive
    });
    world.signal_queue.extend(pre_lifecycle_queue);

    // Phase 6: Output
    phase6_output(world, &pre_tick_states, &fired_this_tick, diag, tc)
}

// ---------------------------------------------------------------------------
// Phase 1: Input
// ---------------------------------------------------------------------------

fn phase1_input(world: &mut World) {
    // Rotate dirty_sms into prev_dirty_sms (§11.2 dirty-flag optimization).
    // Phase 2 reads prev_dirty_sms to decide whether to evaluate IrWatch::AnySm rules.
    // dirty_sms accumulates transitions fired in Phase 3 of THIS tick.
    world.prev_dirty_sms = std::mem::take(&mut world.dirty_sms);

    // Pull external continuous values into SM context fields (§2.4.3).
    // Bindings are evaluated in registration order for determinism.
    // Collect (sm_id, field, value) first to avoid borrow conflicts.
    let updates: Vec<(SmId, String, f64)> = world.continuous_inputs
        .iter()
        .map(|b| (b.sm_id, b.target_field.clone(), (b.source)()))
        .collect();

    for (sm_id, field, value) in updates {
        if let Some(inst) = world.instances.get_mut(&sm_id) {
            let prev = inst.context.get(&field);
            inst.context.set(field, value);
            // Wake SM if value changed (avoids evaluating idle SMs needlessly).
            if (value - prev).abs() > f64::EPSILON {
                world.active_set.insert(sm_id);
            }
        }
    }
    // Discrete player inputs and external events are injected by the Adapter
    // via World::inject_signal() before tick() is called.
}

// ---------------------------------------------------------------------------
// Phase 2: Evaluate (pure read — no mutations)
// ---------------------------------------------------------------------------

fn phase2_evaluate(world: &World, tc: &mut TraceCollector) -> EvaluationResult {
    // ── Phase 2a: SM Guard evaluation ──────────────────────────────────────
    // Serial path (default): evaluate each SM in active_set sequentially.
    // Parallel path (feature = "parallel"): rayon par_iter over active_set.
    // Both paths produce identical BTreeMap<SmId, Option<TransitionId>>.
    #[cfg(not(feature = "parallel"))]
    let decisions: BTreeMap<SmId, Option<TransitionId>> = {
        let mut map = BTreeMap::new();
        for &sm_id in &world.active_set {
            let instance = match world.instances.get(&sm_id) {
                Some(i) => i,
                None => continue,
            };
            let def = match world.defs.get(&sm_id) {
                Some(d) => d,
                None => continue,
            };
            let decision = evaluate_sm_traced(instance, def, world.tick, sm_id, TracePhase::Evaluate, tc, &world.tables);
            map.insert(sm_id, decision);
        }
        map
    };

    #[cfg(feature = "parallel")]
    let decisions: BTreeMap<SmId, Option<TransitionId>> = {
        use rayon::prelude::*;
        // Collect (sm_id, decision, per-thread trace events) in parallel.
        // Each rayon thread has its own Vec<TraceEvent> — no locking needed.
        let results: Vec<(SmId, Option<TransitionId>, Vec<TraceEvent>)> = world
            .active_set
            .par_iter()
            .filter_map(|&sm_id| {
                let instance = world.instances.get(&sm_id)?;
                let def = world.defs.get(&sm_id)?;
                let (decision, events) =
                    evaluate_sm_pure(instance, def, world.tick, sm_id, TracePhase::Evaluate, &world.tables);
                Some((sm_id, decision, events))
            })
            .collect();
        // Merge results back into BTreeMap (insertion order = sm_id order for determinism).
        let mut map = BTreeMap::new();
        for (sm_id, decision, events) in results {
            map.insert(sm_id, decision);
            for e in events { tc.push(e); }
        }
        map
    };

    // ── Phase 2b: Interaction Rule evaluation ─────────────────────────────
    // Serial path (default): evaluate each rule sequentially.
    // Parallel path (feature = "parallel"): rayon par_iter with enumerate.
    // Both paths preserve rule_idx ordering in ir_signals for determinism.

    /// Evaluate a single IR and return (rule_idx, signals, trace_events).
    /// All inputs are read-only — safe to call from multiple rayon threads.
    #[inline]
    fn eval_one_rule(
        rule_idx: usize,
        rule: &InteractionRuleDef,
        instances: &std::collections::BTreeMap<SmId, SmInstance>,
        prev_dirty: &std::collections::BTreeSet<SmId>,
        spatial: &Option<crate::spatial::SpatialIndex>,
        tick: u64,
    ) -> (usize, Vec<IrSignal>, Vec<TraceEvent>) {
        let passes_dirty_check = match &rule.watch {
            IrWatch::All => true,
            IrWatch::AnySm(watched) => watched.iter().any(|id| prev_dirty.contains(id)),
        };
        let should_run = passes_dirty_check && match (&rule.spatial_condition, spatial) {
            (Some(_cond), None) => true,
            (None, _) => true,
            (Some(_), Some(_)) => true,
        };
        if !should_run {
            return (rule_idx, vec![], vec![]);
        }
        let mut signals = (rule.match_fn)(instances);
        if let (Some(cond), Some(sp)) = (&rule.spatial_condition, spatial) {
            // Apply spatial_condition as a per-signal post-filter (§7.1).
            // Signals with source_sm=None bypass spatial filtering (backward-compatible).
            signals.retain(|sig| {
                match sig.source_sm {
                    Some(src) => cond(sp, src, sig.target_sm),
                    None      => true,
                }
            });
        }
        let mut events: Vec<TraceEvent> = Vec::new();
        if !signals.is_empty() {
            let participants: Vec<SmId> = signals.iter().map(|s| s.target_sm).collect();
            events.push(TraceEvent::IrMatched {
                tick,
                phase: TracePhase::Evaluate,
                rule_index: rule_idx,
                participants,
            });
        }
        (rule_idx, signals, events)
    }

    #[cfg(not(feature = "parallel"))]
    let ir_signals: Vec<IrSignal> = {
        let mut acc: Vec<IrSignal> = Vec::new();
        for (rule_idx, rule) in world.interaction_rules.iter().enumerate() {
            let (_, mut signals, events) = eval_one_rule(
                rule_idx, rule, &world.instances,
                &world.prev_dirty_sms, &world.spatial_index, world.tick,
            );
            for e in events { tc.push(e); }
            acc.append(&mut signals);
        }
        acc
    };

    #[cfg(feature = "parallel")]
    let ir_signals: Vec<IrSignal> = {
        use rayon::prelude::*;
        // Collect per-rule results in parallel, then merge in rule_idx order.
        let mut results: Vec<(usize, Vec<IrSignal>, Vec<TraceEvent>)> = world
            .interaction_rules
            .par_iter()
            .enumerate()
            .map(|(rule_idx, rule)| {
                eval_one_rule(
                    rule_idx, rule, &world.instances,
                    &world.prev_dirty_sms, &world.spatial_index, world.tick,
                )
            })
            .collect();
        // Sort by rule_idx to maintain deterministic signal delivery order.
        results.sort_unstable_by_key(|(idx, _, _)| *idx);
        let mut acc: Vec<IrSignal> = Vec::new();
        for (_, mut signals, events) in results {
            for e in events { tc.push(e); }
            acc.append(&mut signals);
        }
        acc
    };

    EvaluationResult { decisions, ir_signals }
}

/// Evaluate which transition (if any) should fire for a single SM.
/// Guards are evaluated against the SM's current context and pending signals.
/// The highest-priority passing guard wins. Ties are a design error (warned at definition time).
/// Emits `GuardEvaluated` trace events for every candidate transition.
fn evaluate_sm_traced(
    instance: &SmInstance,
    def: &SmDef,
    tick: u64,
    sm_id: SmId,
    phase: TracePhase,
    tc: &mut TraceCollector,
    tables: &crate::expr::TableRegistry,
) -> Option<TransitionId> {
    let mut candidates: Vec<&Transition> = def
        .transitions
        .iter()
        .filter(|t| t.source == instance.active_state)
        .collect();
    candidates.sort_by(|a, b| b.priority.cmp(&a.priority));

    let signal = instance.pending_signals.first().map(|(_, s)| s);
    let mut winner: Option<TransitionId> = None;

    for t in candidates {
        // When guard_expr is available, produce an eval tree for AST visualization.
        let (passes, eval_tree) = match (&t.guard, &t.guard_expr) {
            (Some(_), Some(expr)) => {
                let sig_payload = signal.map(|s| &s.payload);
                let (result, tree) = crate::expr::eval_guard_traced(
                    expr, &instance.context, sig_payload, tables,
                );
                (result, Some(tree))
            }
            (Some(guard_fn), None) => (guard_fn(&instance.context, signal), None),
            (None, _) => (true, None),
        };
        // Capture context snapshot for debugging (only scalar fields).
        let ctx_snap: Option<Vec<(String, f64)>> = if t.guard.is_some() {
            Some(instance.context.scalars.iter()
                .map(|(k, &v)| (k.clone(), v))
                .collect())
        } else {
            None // Unconditional guard — context irrelevant.
        };
        tc.push(TraceEvent::GuardEvaluated {
            tick,
            phase,
            transition: t.id,
            sm_id,
            result: passes,
            context_snapshot: ctx_snap,
            eval_tree,
        });
        if passes && winner.is_none() {
            winner = Some(t.id);
            // Continue iterating to trace all candidates, but don't overwrite winner.
        }
    }

    winner
}

// ---------------------------------------------------------------------------
// Phase 2 parallel helper (§11.6)
// ---------------------------------------------------------------------------

/// Pure version of evaluate_sm_traced: returns (decision, trace_events) without
/// requiring &mut TraceCollector. Used by the parallel Phase 2 path so each
/// rayon thread can produce its own event list, which is merged after collection.
#[cfg(feature = "parallel")]
fn evaluate_sm_pure(
    instance: &SmInstance,
    def: &SmDef,
    tick: u64,
    sm_id: SmId,
    phase: TracePhase,
    tables: &crate::expr::TableRegistry,
) -> (Option<TransitionId>, Vec<TraceEvent>) {
    let mut candidates: Vec<&Transition> = def
        .transitions
        .iter()
        .filter(|t| t.source == instance.active_state)
        .collect();
    candidates.sort_by(|a, b| b.priority.cmp(&a.priority));

    let signal = instance.pending_signals.first().map(|(_, s)| s);
    let mut winner: Option<TransitionId> = None;
    let mut events: Vec<TraceEvent> = Vec::new();

    for t in candidates {
        let (passes, eval_tree) = match (&t.guard, &t.guard_expr) {
            (Some(_), Some(expr)) => {
                let sig_payload = signal.map(|s| &s.payload);
                let (result, tree) = crate::expr::eval_guard_traced(
                    expr, &instance.context, sig_payload, tables,
                );
                (result, Some(tree))
            }
            (Some(guard_fn), None) => (guard_fn(&instance.context, signal), None),
            (None, _) => (true, None),
        };
        #[cfg(feature = "trace")]
        {
            let ctx_snap: Option<Vec<(String, f64)>> = if t.guard.is_some() {
                Some(instance.context.scalars.iter()
                    .map(|(k, &v)| (k.clone(), v))
                    .collect())
            } else {
                None
            };
            events.push(TraceEvent::GuardEvaluated {
                tick,
                phase,
                transition: t.id,
                sm_id,
                result: passes,
                context_snapshot: ctx_snap,
                eval_tree,
            });
        }
        #[cfg(not(feature = "trace"))]
        let _ = (tick, phase, sm_id, events.len()); // suppress unused warnings
        if passes && winner.is_none() {
            winner = Some(t.id);
        }
    }

    (winner, events)
}

// ---------------------------------------------------------------------------
// Phase 3: Execute
// ---------------------------------------------------------------------------

fn phase3_execute(world: &mut World, eval: &mut EvaluationResult, tc: &mut TraceCollector) -> BTreeSet<SmId> {
    // Collect emissions and all fired transitions separately.
    // Compound lifecycle must run for every fired transition, even when effects are empty.
    let mut emissions: Vec<(SmId, StateId, Vec<(PortId, Signal)>)> = Vec::new();
    let mut all_fired_transitions: Vec<(SmId, StateId, StateId)> = Vec::new(); // (sm_id, from_state, new_state)
    let mut fired: BTreeSet<SmId> = BTreeSet::new();

    for (&sm_id, &maybe_transition) in &eval.decisions {
        let transition_id = match maybe_transition {
            Some(t) => t,
            None => continue,
        };

        let instance = match world.instances.get_mut(&sm_id) {
            Some(i) => i,
            None => continue,
        };
        let def = match world.defs.get(&sm_id) {
            Some(d) => d,
            None => continue,
        };

        let t = match def.transitions.iter().find(|t| t.id == transition_id) {
            Some(t) => t,
            None => continue,
        };

        // Fire: advance active state.
        let from_state = instance.active_state;
        let new_state = t.target;
        instance.active_state = new_state;
        fired.insert(sm_id);
        // Mark as dirty for IrWatch::AnySm optimization (§11.2).
        world.dirty_sms.insert(sm_id);
        all_fired_transitions.push((sm_id, from_state, new_state));

        // Trace: TransitionFired
        tc.push(TraceEvent::TransitionFired {
            tick: world.tick,
            phase: TracePhase::Execute,
            transition: transition_id,
            sm_id,
            from_state,
            to_state: new_state,
        });

        // Run effects: collect signals and system commands.
        let mut emitted_signals: Vec<(PortId, Signal)> = Vec::new();
        for effect in &t.effects {
            for output in effect(&mut instance.context) {
                match output {
                    EffectOutput::Signal(port, sig) => emitted_signals.push((port, sig)),
                    EffectOutput::Cmd(cmd) => world.pending_system_commands.push(cmd),
                }
            }
        }

        // Clear consumed pending signals.
        instance.pending_signals.clear();

        if !emitted_signals.is_empty() {
            emissions.push((sm_id, new_state, emitted_signals));
        }
    }

    // Handle Compound State lifecycle for every fired transition (§4).
    // Must run before signal routing so sub-SM Active Set state is correct.
    for (sm_id, _from_state, new_state) in all_fired_transitions {
        handle_compound_exit_enter(world, sm_id, new_state, &mut fired, &mut crate::error::TickDiagnostics::default());
    }

    // Enqueue emitted signals through connections into the signal queue.
    // Static Connection signals are enqueued before dynamic ones (§3 Determinism Guarantee).
    for (source_sm, _new_state, emitted) in emissions {
        for (source_port, signal) in emitted {
            route_signal(world, source_sm, source_port, signal, TracePhase::Execute, tc);
        }
    }

    // Enqueue Interaction Rule signals determined in Phase 2.
    // IR signals are delivered after static Connection signals (§3 Determinism Guarantee).
    // Sorted by target SmId ascending for determinism.
    let mut ir_signals = std::mem::take(&mut eval.ir_signals);
    ir_signals.sort_by_key(|s| s.target_sm);
    for ir_sig in ir_signals {
        // Write payload into context so guards can read it.
        if let Some(instance) = world.instances.get_mut(&ir_sig.target_sm) {
            for (k, v) in &ir_sig.signal.payload {
                instance.context.set(k.clone(), *v);
            }
        }
        world.signal_queue.push_back(QueuedSignal {
            target_sm:   ir_sig.target_sm,
            target_port: ir_sig.target_port,
            signal:      ir_sig.signal,
            delay: 0,
            source_conn: None,
            source_sm:   None, // IR-generated signal
        });
        world.active_set.insert(ir_sig.target_sm);
        fired.insert(ir_sig.target_sm);
    }

    fired
}

/// Handle Compound State entry/exit when a parent SM fires a transition.
///
/// Called after the parent SM's active_state has already been updated to `new_state`.
/// The previous state (exit side) is determined by looking up what the SM was before
/// the transition — which we reconstruct by checking compound_defs.
fn handle_compound_exit_enter(
    world: &mut World,
    parent_sm_id: SmId,
    new_state: StateId,
    fired: &mut BTreeSet<SmId>,
    diag: &mut crate::error::TickDiagnostics,
) {
    // --- EXIT: find compound def that was active before this transition ---
    // We need the previous state; since the SM is already updated we detect exit
    // by checking if any compound def's parent_sm matches and its state ≠ new_state.
    // Collect sub-SMs to potentially suspend.
    let exited_sub_machines: Vec<(StateId, Vec<SmId>, SuspendPolicyRt)> = world
        .compound_defs
        .values()
        .filter(|cd| cd.parent_sm == parent_sm_id && cd.parent_state != new_state)
        .map(|cd| (cd.parent_state, cd.sub_machines.clone(), cd.suspend_policy.clone()))
        .collect();

    for (_exited_state, sub_ids, policy) in exited_sub_machines {
        for sub_id in sub_ids {
            match policy {
                SuspendPolicyRt::Freeze => {
                    if let Some(inst) = world.instances.get(&sub_id) {
                        world.frozen_snapshots.insert(sub_id, FrozenSmSnapshot {
                            sm_id:        sub_id,
                            active_state: inst.active_state,
                            context:      inst.context.clone(),
                            frozen_at_tick: world.tick,
                        });
                    }
                    world.active_set.remove(&sub_id);
                }
                SuspendPolicyRt::Elapse => {
                    // Record snapshot with timestamp; elapse_fn applied on re-entry.
                    if let Some(inst) = world.instances.get(&sub_id) {
                        world.frozen_snapshots.insert(sub_id, FrozenSmSnapshot {
                            sm_id:        sub_id,
                            active_state: inst.active_state,
                            context:      inst.context.clone(),
                            frozen_at_tick: world.tick,
                        });
                    }
                    world.active_set.remove(&sub_id);
                }
                SuspendPolicyRt::Discard => {
                    world.frozen_snapshots.remove(&sub_id);
                    if let (Some(inst), Some(def)) = (
                        world.instances.get_mut(&sub_id),
                        world.defs.get(&sub_id),
                    ) {
                        inst.active_state = def.initial_state;
                        inst.context = Context::default();
                        inst.pending_signals.clear();
                    }
                    world.active_set.remove(&sub_id);
                }
            }
        }
    }

    // --- ENTER: activate sub-SMs for the new compound state (if any) ---
    let entering: Option<(Vec<SmId>, SuspendPolicyRt, Vec<(SmId, PortId)>)> = world
        .compound_defs
        .get(&new_state)
        .filter(|cd| cd.parent_sm == parent_sm_id)
        .map(|cd| (
            cd.sub_machines.clone(),
            cd.suspend_policy.clone(),
            cd.promoted_ports.clone(),
        ));

    if let Some((sub_ids, policy, _promoted)) = entering {
        for sub_id in sub_ids {
            match policy {
                SuspendPolicyRt::Freeze => {
                    if let Some(snap) = world.frozen_snapshots.remove(&sub_id) {
                        if let Some(inst) = world.instances.get_mut(&sub_id) {
                            inst.active_state = snap.active_state;
                            inst.context = snap.context;
                            inst.pending_signals.clear();
                        }
                    }
                }
                SuspendPolicyRt::Elapse => {
                    if let Some(snap) = world.frozen_snapshots.remove(&sub_id) {
                        let elapsed = world.tick.saturating_sub(snap.frozen_at_tick);
                        let (new_state_id, new_ctx) =
                            if let Some(def) = world.defs.get(&sub_id) {
                                match (&def.elapse_capability, &def.elapse_fn) {
                                    (ElapseCapabilityRt::NonElapsable, _) |
                                    (_, None) => {
                                        (snap.active_state, snap.context.clone())
                                    }
                                    (_, Some(elapse_fn)) => {
                                        let (returned, ctx) =
                                            elapse_fn(snap.active_state, &snap.context, elapsed);
                                        // Validate: returned state must be in def.states (§11.5).
                                        if def.states.contains(&returned) {
                                            (returned, ctx)
                                        } else {
                                            // Invalid state — fall back to frozen state and log.
                                            diag.push(crate::error::WeavenDiagnostic::ElapseInvalidState {
                                                sm_id:          sub_id,
                                                returned_state: returned,
                                                fallback_state: snap.active_state,
                                            });
                                            (snap.active_state, snap.context.clone())
                                        }
                                    }
                                }
                            } else {
                                (snap.active_state, snap.context.clone())
                            };

                        if let Some(inst) = world.instances.get_mut(&sub_id) {
                            inst.active_state = new_state_id;
                            inst.context = new_ctx;
                            inst.pending_signals.clear();
                        }
                    }
                }
                SuspendPolicyRt::Discard => {
                    if let (Some(inst), Some(def)) = (
                        world.instances.get_mut(&sub_id),
                        world.defs.get(&sub_id),
                    ) {
                        inst.active_state = def.initial_state;
                        inst.context = Context::default();
                        inst.pending_signals.clear();
                    }
                }
            }
            world.active_set.insert(sub_id);
            fired.insert(sub_id);
        }
    }
}


/// Each Connection's pipeline (Transform/Filter/Redirect) is applied before enqueuing.
fn route_signal(world: &mut World, source_sm: SmId, source_port: PortId, signal: Signal, phase: TracePhase, tc: &mut TraceCollector) {
    // Collect target info without borrowing `world.connections` mutably.
    let targets: Vec<(ConnectionId, usize, SmId, PortId, u32)> = world
        .connections
        .iter()
        .enumerate()
        .filter(|(_, c)| c.source_sm == source_sm && c.source_port == source_port)
        .map(|(i, c)| (c.id, i, c.target_sm, c.target_port, c.delay_ticks))
        .collect();

    // Sort by target SmId ascending for deterministic delivery order (§3 Determinism Guarantee).
    let mut targets = targets;
    targets.sort_by_key(|&(_, _, sm_id, _, _)| sm_id);

    for (conn_id, conn_idx, target_sm, target_port, delay) in targets {
        // Apply Connection-side pipeline.
        let routed = apply_pipeline(
            &world.connections[conn_idx].pipeline,
            signal.clone(),
            target_port,
        );

        let (final_port, final_signal) = match routed {
            Some(r) => r,
            None => {
                // Trace: Connection-side PipelineFiltered (Gap 2 fix).
                tc.push(TraceEvent::PipelineFiltered {
                    tick: world.tick,
                    phase,
                    connection: Some(conn_id),
                    sm_id: target_sm,
                    port: target_port,
                });
                continue; // filtered out — drop signal
            }
        };

        // Trace: SignalEmitted with resolved target (Gap 1 fix).
        tc.push(TraceEvent::SignalEmitted {
            tick: world.tick,
            phase,
            sm_id: source_sm,
            port: source_port,
            target: Some(target_sm),
        });

        // Write payload fields into target context for guard/effect access.
        if let Some(instance) = world.instances.get_mut(&target_sm) {
            for (k, v) in &final_signal.payload {
                instance.context.set(k.clone(), *v);
            }
        }

        world.signal_queue.push_back(QueuedSignal {
            target_sm,
            target_port: final_port,
            signal: final_signal,
            delay,
            source_conn: Some(conn_id),
            source_sm: Some(source_sm),
        });
        if delay == 0 {
            world.active_set.insert(target_sm);
        }
    }

    // ── Spatial routing via influence_radius (§7.1) ──────────────────────
    // If the source port has an influence_radius, deliver the signal to all
    // nearby SMs that have a compatible Input Port (matching signal_type).
    // This is the Port-based spatial routing layer (distinct from IR matching).
    //
    // Borrow strategy: collect (target_sm, target_port) pairs from read-only
    // refs first, then mutably update world.instances.
    let spatial_targets: Vec<(SmId, PortId)> = {
        match (&world.spatial_index, world.defs.get(&source_sm)) {
            (Some(spatial), Some(def)) => {
                match def.output_ports.iter().find(|p| p.id == source_port) {
                    Some(port) if port.influence_radius.is_some() => {
                        let radius   = port.influence_radius.unwrap();
                        let sig_type = port.signal_type;
                        let mut nearby: Vec<SmId> = spatial
                            .query_radius_of(source_sm, radius)
                            .into_iter()
                            .filter(|&id| id != source_sm)
                            .collect();
                        nearby.sort(); // deterministic delivery order
                        let mut pairs = Vec::new();
                        for nearby_sm in nearby {
                            if let Some(nearby_def) = world.defs.get(&nearby_sm) {
                                for ip in nearby_def.input_ports.iter()
                                    .filter(|p| p.signal_type == sig_type)
                                {
                                    pairs.push((nearby_sm, ip.id));
                                }
                            }
                        }
                        pairs
                    }
                    _ => vec![],
                }
            }
            _ => vec![],
        }
    };

    for (target_sm, target_port) in spatial_targets {
        // Write payload into context (same pattern as static connections).
        if let Some(instance) = world.instances.get_mut(&target_sm) {
            for (k, v) in &signal.payload {
                instance.context.set(k.clone(), *v);
            }
        }
        // Trace
        tc.push(TraceEvent::SignalEmitted {
            tick: world.tick,
            phase,
            sm_id: source_sm,
            port: source_port,
            target: Some(target_sm),
        });
        world.signal_queue.push_back(QueuedSignal {
            target_sm,
            target_port,
            signal: signal.clone(),
            delay: 0,
            source_conn: None,
            source_sm: Some(source_sm),
        });
        world.active_set.insert(target_sm);
    }
}

// ---------------------------------------------------------------------------
// Phase 4: Propagate
// ---------------------------------------------------------------------------

fn phase4_propagate(world: &mut World, fired_this_tick: &mut BTreeSet<SmId>, diag: &mut crate::error::TickDiagnostics, tc: &mut TraceCollector) {
    let max_depth = world.max_cascade_depth;
    let mut depth = 0u32;

    // Deliver zero-delay signals and cascade until queue is empty or max depth.
    loop {
        // Collect all zero-delay signals ready for delivery this pass.
        let ready: Vec<QueuedSignal> = {
            let mut r = Vec::new();
            let mut deferred = VecDeque::new();
            while let Some(qs) = world.signal_queue.pop_front() {
                if qs.delay == 0 {
                    r.push(qs);
                } else {
                    deferred.push_back(qs);
                }
            }
            world.signal_queue = deferred;
            r
        };

        if ready.is_empty() {
            break;
        }

        if depth >= max_depth {
            let pending_count = ready.len() + world.signal_queue.len();
            let policy = world.cascade_overflow_policy.clone();
            let action = match policy {
                crate::error::CascadeOverflowPolicy::DeferToNextTick => {
                    // Preserve signals for the next tick's Phase 4.
                    for qs in ready {
                        world.signal_queue.push_back(qs);
                    }
                    crate::error::CascadeOverflowAction::DeferToNextTick
                }
                crate::error::CascadeOverflowPolicy::DiscardAndContinue => {
                    // Drop ready signals; deferred signals already back in queue.
                    crate::error::CascadeOverflowAction::DiscardAndContinue
                }
            };
            diag.push(crate::error::WeavenDiagnostic::CascadeDepthExceeded {
                tick: world.tick,
                depth_reached: depth,
                pending_count,
                action,
            });
            break;
        }

        depth += 1;

        // Trace: CascadeStep
        tc.push(TraceEvent::CascadeStep {
            tick: world.tick,
            phase: TracePhase::Propagate,
            depth,
            queue_size: ready.len(),
        });

        // Deliver each ready signal to its target SM.
        // Collect cascade emissions to re-enqueue.
        let mut cascade_emissions: Vec<(SmId, PortId, Signal)> = Vec::new();

        for qs in ready {
            // Apply Input-Port-side pipeline (§6.2, §6.3 steps 4–6).
            // This runs after the Connection-side pipeline (already applied in route_signal).
            let (final_port, final_signal) = {
                let port_pipeline: Vec<&PipelineStep> = world
                    .defs
                    .get(&qs.target_sm)
                    .and_then(|def| {
                        def.input_ports
                            .iter()
                            .find(|p| p.id == qs.target_port)
                            .map(|p| p.input_pipeline.iter().collect())
                    })
                    .unwrap_or_default();

                if port_pipeline.is_empty() {
                    (qs.target_port, qs.signal)
                } else {
                    // Collect owned steps so we can pass a slice
                    let steps_owned: Vec<PipelineStep> = world
                        .defs
                        .get(&qs.target_sm)
                        .and_then(|def| {
                            def.input_ports
                                .iter()
                                .find(|p| p.id == qs.target_port)
                                .map(|p| {
                                    // We can't move out of a ref — rebuild via apply_pipeline
                                    // by passing the steps directly via a closure.
                                    let _ = p; // borrow released below
                                    vec![] // placeholder — handled in apply below
                                })
                        })
                        .unwrap_or_default();
                    let _ = steps_owned;

                    // Apply via the shared apply_pipeline function.
                    // We need to temporarily extract the pipeline steps reference.
                    let routed = {
                        let def = world.defs.get(&qs.target_sm).unwrap();
                        let port = def.input_ports.iter().find(|p| p.id == qs.target_port).unwrap();
                        apply_pipeline(&port.input_pipeline, qs.signal.clone(), qs.target_port)
                    };
                    match routed {
                        Some(r) => r,
                        None => {
                            // Trace: PipelineFiltered
                            tc.push(TraceEvent::PipelineFiltered {
                                tick: world.tick,
                                phase: TracePhase::Propagate,
                                connection: qs.source_conn,
                                sm_id: qs.target_sm,
                                port: qs.target_port,
                            });
                            // Filtered out by Input-Port-side pipeline — discard.
                            world.active_set.insert(qs.target_sm);
                            continue;
                        }
                    }
                }
            };

            let instance = match world.instances.get_mut(&qs.target_sm) {
                Some(i) => i,
                None => continue,
            };

            // Write payload into context (pipeline may have transformed it).
            for (k, v) in &final_signal.payload {
                instance.context.set(k.clone(), *v);
            }

            // Buffer the signal into the target SM's pending signals.
            instance.pending_signals.push((final_port, final_signal));

            let def = match world.defs.get(&qs.target_sm) {
                Some(d) => d,
                None => continue,
            };

            // Evaluate guards on transitions that reference this input port.
            // (In Phase 4, only signal-triggered transitions participate — §3 Phase 4.)
            if let Some(transition_id) = evaluate_sm_traced(instance, def, world.tick, qs.target_sm, TracePhase::Propagate, tc, &world.tables) {
                let t = match def.transitions.iter().find(|t| t.id == transition_id) {
                    Some(t) => t,
                    None => continue,
                };

                // Fire the cascade transition.
                let from_state = instance.active_state;
                instance.active_state = t.target;
                fired_this_tick.insert(qs.target_sm);

                // Trace: TransitionFired (cascade)
                tc.push(TraceEvent::TransitionFired {
                    tick: world.tick,
                    phase: TracePhase::Propagate,
                    transition: transition_id,
                    sm_id: qs.target_sm,
                    from_state,
                    to_state: t.target,
                });

                let mut emitted_signals: Vec<(PortId, Signal)> = Vec::new();
                for effect in &t.effects {
                    for output in effect(&mut instance.context) {
                        match output {
                            EffectOutput::Signal(port, sig) => emitted_signals.push((port, sig)),
                            EffectOutput::Cmd(cmd) => world.pending_system_commands.push(cmd),
                        }
                    }
                }
                instance.pending_signals.clear();

                for (port_id, signal) in emitted_signals {
                    cascade_emissions.push((qs.target_sm, port_id, signal));
                }
                // Trace: SignalDelivered (transition fired)
                tc.push(TraceEvent::SignalDelivered {
                    tick: world.tick,
                    phase: TracePhase::Propagate,
                    depth,
                    source_sm: qs.source_sm,
                    target_sm: qs.target_sm,
                    target_port: final_port,
                    triggered_transition: Some(transition_id),
                });
            } else {
                // No transition fired — SM consumed the signal without transitioning.
                instance.pending_signals.clear();

                // Trace: SignalDelivered (no transition)
                tc.push(TraceEvent::SignalDelivered {
                    tick: world.tick,
                    phase: TracePhase::Propagate,
                    depth,
                    source_sm: qs.source_sm,
                    target_sm: qs.target_sm,
                    target_port: final_port,
                    triggered_transition: None,
                });
            }

            // SM was active this pass; keep it in Active Set.
            world.active_set.insert(qs.target_sm);
        }

        // Route cascade emissions back into the queue.
        for (source_sm, source_port, signal) in cascade_emissions {
            route_signal(world, source_sm, source_port, signal, TracePhase::Propagate, tc);
        }
    }

    // Decrement delay counters on deferred signals (for next tick's Phase 4).
    for qs in &mut world.signal_queue {
        if qs.delay > 0 {
            qs.delay -= 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 5: Lifecycle
// ---------------------------------------------------------------------------

fn phase5_lifecycle(world: &mut World, diag: &mut crate::error::TickDiagnostics) {
    // --- Despawning (§3 Phase 5) ---
    //
    // 1. For each despawning entity, fire OnDespawn transitions and collect
    //    emitted signals into a batch queue.
    // 2. After all entities processed, deliver batch signals simultaneously.
    // 3. Cascade from despawn signals follows Phase 4 rules.
    // 4. Sever connections and remove from Active Set.

    let despawn_requests: Vec<DespawnRequest> = std::mem::take(&mut world.despawn_queue);

    let mut despawn_batch: Vec<QueuedSignal> = Vec::new();

    for req in despawn_requests {
        for sm_id in &req.sm_ids {
            let sm_id = *sm_id;

            // Fire OnDespawn transitions: run effects, collect emissions.
            let on_despawn_transitions: Vec<(Vec<EffectFn>, PortId)> = world
                .defs
                .get(&sm_id)
                .map(|def| {
                    def.on_despawn_transitions
                        .iter()
                        .flat_map(|t| {
                            t.effects.iter().map(|_| ()).collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>();
                    // Re-collect effects directly
                    vec![]
                })
                .unwrap_or_default();

            // Iterate on_despawn effects, split signals and system commands.
            let (emitted, despawn_cmds): (Vec<(PortId, Signal)>, Vec<SystemCommand>) =
                if let Some(def) = world.defs.get(&sm_id) {
                    let mut ctx = world
                        .instances
                        .get(&sm_id)
                        .map(|i| i.context.clone())
                        .unwrap_or_default();
                    let mut signals = Vec::new();
                    let mut cmds = Vec::new();
                    for t in &def.on_despawn_transitions {
                        for eff in &t.effects {
                            for output in eff(&mut ctx) {
                                match output {
                                    EffectOutput::Signal(port, sig) => signals.push((port, sig)),
                                    EffectOutput::Cmd(cmd) => cmds.push(cmd),
                                }
                            }
                        }
                    }
                    (signals, cmds)
                } else {
                    (vec![], vec![])
                };
            world.pending_system_commands.extend(despawn_cmds);

            // Route emitted signals through connections → batch queue.
            for (source_port, signal) in emitted {
                let targets: Vec<(SmId, PortId)> = world
                    .connections
                    .iter()
                    .filter(|c| c.source_sm == sm_id && c.source_port == source_port)
                    .map(|c| (c.target_sm, c.target_port))
                    .collect();
                let mut targets = targets;
                targets.sort_by_key(|&(id, _)| id);
                for (target_sm, target_port) in targets {
                    if let Some(inst) = world.instances.get_mut(&target_sm) {
                        for (k, v) in &signal.payload {
                            inst.context.set(k.clone(), *v);
                        }
                    }
                    despawn_batch.push(QueuedSignal {
                        target_sm,
                        target_port,
                        signal: signal.clone(),
                        delay: 0,
                        source_conn: None,
                        source_sm: Some(sm_id),
                    });
                }
            }
        }

        // Sever connections, remove from Active Set, and purge stale in-flight signals
        // targeting the despawned SM (§11.5 — in-flight signal handling).
        for sm_id in &req.sm_ids {
            let sm_id = *sm_id;
            world.active_set.remove(&sm_id);
            world.connections.retain(|c| {
                c.source_sm != sm_id && c.target_sm != sm_id
            });
            world.frozen_snapshots.remove(&sm_id);
            // Remove the instance itself — SM is Destroyed (§4.5).
            world.instances.remove(&sm_id);

            // Remove any queued signals targeting this SM.
            // These are stale — the SM no longer exists to receive them.
            let stale: Vec<_> = world.signal_queue
                .iter()
                .filter(|qs| qs.target_sm == sm_id)
                .map(|qs| qs.source_conn)
                .collect();
            world.signal_queue.retain(|qs| qs.target_sm != sm_id);
            for conn_id in stale {
                diag.push(crate::error::WeavenDiagnostic::StaleSignal {
                    target_sm:   sm_id,
                    source_conn: conn_id,
                });
            }
        }
    }

    // Deliver all despawn batch signals simultaneously (order-independent — §3 Phase 5).
    // These are added to the signal_queue with delay=0; they will be processed
    // in the Phase 4 cascade of the *current* tick (called from tick() after phase5).
    // Do NOT call phase4_propagate here — that would consume unrelated deferred signals.
    let mut activated_by_despawn: Vec<SmId> = Vec::new();
    for qs in despawn_batch {
        activated_by_despawn.push(qs.target_sm);
        world.signal_queue.push_back(qs);
    }
    for sm_id in activated_by_despawn {
        world.active_set.insert(sm_id);
    }

    // --- Spawning (§3 Phase 5, §4.5) ---
    //
    // New entities are registered and their SMs initialized.
    // They do NOT enter the Active Set until the next tick.

    let spawn_requests: Vec<SpawnRequest> = std::mem::take(&mut world.spawn_queue);

    for req in spawn_requests {
        // Establish Connection Templates from the spawn request.
        for conn in req.connections {
            world.connections.push(conn);
        }
        // Reset SM instances to initial state (they may have been pre-registered).
        for sm_id in req.sm_ids {
            if let (Some(inst), Some(def)) = (
                world.instances.get_mut(&sm_id),
                world.defs.get(&sm_id),
            ) {
                inst.active_state = def.initial_state;
                inst.context = Context::default();
                inst.pending_signals.clear();
            }
            // NOTE: do NOT add to active_set here — enters next tick (§4.5).
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 6: Output
// ---------------------------------------------------------------------------

/// Summary of changes produced by one tick, for the presentation/network layer.
#[derive(Debug, Default)]
pub struct TickOutput {
    /// SMs whose active state changed this tick (pre-tick state → new state).
    pub state_changes: BTreeMap<SmId, (StateId, StateId)>,
    /// System Commands to execute this frame (§7.3).
    pub system_commands: Vec<SystemCommand>,
    /// Continuous Output Port values — exposed context fields (§2.4.4).
    pub continuous_outputs: BTreeMap<SmId, BTreeMap<String, f64>>,
    /// Runtime diagnostics — errors and warnings for this tick (§11.5).
    pub diagnostics: crate::error::TickDiagnostics,
    /// Debug trace events collected during this tick (§11.3).
    /// Only populated when the `trace` feature is enabled.
    pub trace_events: Vec<crate::trace::TraceEvent>,
}

fn phase6_output(
    world: &mut World,
    pre_tick_states: &BTreeMap<SmId, StateId>,
    fired_this_tick: &BTreeSet<SmId>,
    diag: crate::error::TickDiagnostics,
    tc: TraceCollector,
) -> TickOutput {
    // --- State Diff ---
    let mut out = TickOutput::default();
    for (&sm_id, instance) in &world.instances {
        let prev = match pre_tick_states.get(&sm_id) {
            Some(&s) => s,
            None => continue,
        };
        if instance.active_state != prev {
            out.state_changes.insert(sm_id, (prev, instance.active_state));
        }
    }

    // --- Execute System Commands (§7.3, Phase 6) ---
    // Drain accumulated commands, apply to world state, expose to Adapter via TickOutput.
    let commands: Vec<SystemCommand> = std::mem::take(&mut world.pending_system_commands);
    for cmd in &commands {
        match cmd {
            SystemCommand::HitStop { frames } => {
                world.hit_stop_frames = world.hit_stop_frames.saturating_add(*frames);
            }
            SystemCommand::SlowMotion { factor, duration_ticks } => {
                world.slow_motion_remaining = *duration_ticks;
                world.slow_motion_factor = *factor;
            }
            SystemCommand::TimeScale(scale) => {
                world.time_scale = *scale;
            }
        }
    }
    out.system_commands = commands;

    // Tick down slow motion counter.
    if world.slow_motion_remaining > 0 {
        world.slow_motion_remaining -= 1;
        if world.slow_motion_remaining == 0 {
            world.slow_motion_factor = 1.0;
        }
    }

    // Tick down hit stop (each call to tick() consumes one frame).
    if world.hit_stop_frames > 0 {
        world.hit_stop_frames -= 1;
    }

    // --- Active Set Management (§7.2) ---
    let sms_with_pending: BTreeSet<SmId> = world
        .signal_queue.iter().map(|qs| qs.target_sm).collect();

    let to_deactivate: Vec<SmId> = world
        .active_set.iter().copied()
        .filter(|sm_id| {
            !fired_this_tick.contains(sm_id) && !sms_with_pending.contains(sm_id)
        })
        .collect();

    for sm_id in to_deactivate {
        world.active_set.remove(&sm_id);
    }

    out.diagnostics = diag;
    out.trace_events = tc.into_events();

    // --- Continuous Output Ports (§2.4.4) ---
    // Publish declared context fields for read-only consumption by rendering/audio/UI.
    // Only explicitly declared fields are exposed — internal state does not leak.
    for decl in &world.continuous_outputs {
        if let Some(inst) = world.instances.get(&decl.sm_id) {
            let values: BTreeMap<String, f64> = decl.exposed_fields.iter()
                .map(|f| (f.clone(), inst.context.get(f)))
                .collect();
            out.continuous_outputs.insert(decl.sm_id, values);
        }
    }

    out
}
