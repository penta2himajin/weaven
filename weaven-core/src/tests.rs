#[cfg(test)]
mod property_tests {
    #[allow(unused_imports)]
    use crate::models::*;
    #[allow(unused_imports)]
    use crate::fixtures::*;

    #[test]
    fn initial_state_always_valid() {
        let state_machines: Vec<StateMachine> = vec![default_state_machine()];
        assert!(state_machines.iter().all(|sm| { let sm = sm.clone(); sm.states.contains(&sm.initialState) }));
    }

    #[test]
    fn active_state_always_valid() {
        let state_machines: Vec<StateMachine> = vec![default_state_machine()];
        assert!(state_machines.iter().all(|sm| { let sm = sm.clone(); sm.activeState.as_ref().map_or(true, |s| sm.states.contains(s)) }));
    }

    #[test]
    fn transition_endpoints_valid() {
        let state_machines: Vec<StateMachine> = vec![default_state_machine()];
        assert!(state_machines.iter().all(|sm| { let sm = sm.clone(); sm.transitions.iter().all(|t| { let t = t.clone(); sm.states.contains(&t.source) && sm.states.contains(&t.target) }) }));
    }

    #[test]
    fn connections_are_directional() {
        let connections: Vec<Connection> = vec![default_connection()];
        assert!(connections.iter().all(|c| { let c = c.clone(); c.connSource.portKind == PortKind::PortKindOutput || c.connSource.portKind == PortKind::PortKindContinuousOutput }));
    }

    #[test]
    fn active_set_bounded() {
        let active_sets: Vec<ActiveSet> = vec![default_active_set()];
        let entitys: Vec<Entity> = vec![default_entity()];
        assert!(active_sets.iter().all(|aset| { let aset = aset.clone(); aset.activeMachines.iter().all(|sm| { let sm = sm.clone(); entitys.iter().any(|e| { let e = e.clone(); e.machines.contains(&sm) }) }) }));
    }

    /// @regression Partially type-guaranteed — regression test only.
    #[test]
    fn invariant_interaction_rule_requires_two_participants() {
        let interaction_rules: Vec<InteractionRule> = vec![default_interaction_rule()];
        assert!(interaction_rules.iter().all(|ir| { let ir = ir.clone(); ir.participants.len() >= 2 }));
    }

    /// @regression Partially type-guaranteed — regression test only.
    #[test]
    fn invariant_i_r_result_targets_own_participant() {
        let interaction_rules: Vec<InteractionRule> = vec![default_interaction_rule()];
        assert!(interaction_rules.iter().all(|ir| { let ir = ir.clone(); ir.results.iter().all(|r| { let r = r.clone(); ir.participants.contains(&r.resultParticipant) }) }));
    }

    #[test]
    fn invariant_i_r_result_signal_type_consistent() {
        let i_r_results: Vec<IRResult> = vec![default_i_r_result()];
        assert!(i_r_results.iter().all(|r| { let r = r.clone(); r.resultSignalType == r.resultPort.signalType }));
    }

    #[test]
    fn invariant_named_table_names_unique() {
        let named_tables: Vec<NamedTable> = vec![default_named_table()];
        assert!(named_tables.iter().all(|t1| { let t1 = t1.clone(); named_tables.iter().all(|t2| { let t2 = t2.clone(); !(t1 != t2) || t1.tableName != t2.tableName }) }));
    }

    #[test]
    fn invariant_port_promotion_ownership() {
        let port_promotions: Vec<PortPromotion> = vec![default_port_promotion()];
        assert!(port_promotions.iter().all(|pp| { let pp = pp.clone(); (*pp.promotionOwner).subMachines.ports.contains(&(*pp.promotedPort)) }));
    }

    #[test]
    fn invariant_port_promotion_output_only() {
        let port_promotions: Vec<PortPromotion> = vec![default_port_promotion()];
        assert!(port_promotions.iter().all(|pp| { let pp = pp.clone(); (*pp.promotedPort).portKind == PortKind::PortKindOutput || (*pp.promotedPort).portKind == PortKind::PortKindContinuousOutput }));
    }

    #[test]
    fn invariant_initial_state_owned() {
        let state_machines: Vec<StateMachine> = vec![default_state_machine()];
        assert!(state_machines.iter().all(|sm| { let sm = sm.clone(); sm.states.contains(&sm.initialState) }));
    }

    #[test]
    fn invariant_active_state_owned() {
        let state_machines: Vec<StateMachine> = vec![default_state_machine()];
        assert!(state_machines.iter().all(|sm| { let sm = sm.clone(); sm.activeState.as_ref().map_or(true, |s| sm.states.contains(s)) }));
    }

    #[test]
    fn invariant_transition_states_owned() {
        let state_machines: Vec<StateMachine> = vec![default_state_machine()];
        assert!(state_machines.iter().all(|sm| { let sm = sm.clone(); sm.transitions.iter().all(|t| { let t = t.clone(); sm.states.contains(&t.source) && sm.states.contains(&t.target) }) }));
    }

    #[test]
    fn invariant_unique_transition_priority() {
        let state_machines: Vec<StateMachine> = vec![default_state_machine()];
        assert!(state_machines.iter().all(|sm| { let sm = sm.clone(); sm.transitions.iter().all(|t1| { let t1 = t1.clone(); sm.transitions.iter().all(|t2| { let t2 = t2.clone(); t1 != t2 && !(t1.source == t2.source) || t1.priority != t2.priority }) }) }));
    }

    #[test]
    fn invariant_connection_directionality() {
        let connections: Vec<Connection> = vec![default_connection()];
        assert!(connections.iter().all(|c| { let c = c.clone(); c.connSource.portKind == PortKind::PortKindOutput || c.connSource.portKind == PortKind::PortKindContinuousOutput && c.connTarget.portKind == PortKind::PortKindInput || c.connTarget.portKind == PortKind::PortKindContinuousInput }));
    }

    #[test]
    fn invariant_connection_signal_type_compat() {
        let connections: Vec<Connection> = vec![default_connection()];
        assert!(connections.iter().all(|c| { let c = c.clone(); c.connSource.signalType == c.connTarget.signalType }));
    }

    #[test]
    fn invariant_entity_has_machines() {
        let entitys: Vec<Entity> = vec![default_entity()];
        assert!(entitys.iter().all(|e| { let e = e.clone(); e.machines.is_some() }));
    }

    #[test]
    fn invariant_active_set_subset_of_entities() {
        let active_sets: Vec<ActiveSet> = vec![default_active_set()];
        let entitys: Vec<Entity> = vec![default_entity()];
        assert!(active_sets.iter().all(|aset| { let aset = aset.clone(); aset.activeMachines.iter().all(|sm| { let sm = sm.clone(); entitys.iter().any(|e| { let e = e.clone(); e.machines.contains(&sm) }) }) }));
    }

    #[test]
    fn invariant_sub_machine_does_not_own_parent_state() {
        let compound_states: Vec<CompoundState> = vec![default_compound_state()];
        assert!(compound_states.iter().all(|cs| { let cs = cs.clone(); cs.subMachines.iter().all(|sm| { let sm = sm.clone(); !sm.states.iter().any(|s| { let s = s.clone(); s == cs.compoundParent }) }) }));
    }

    #[test]
    fn boundary_interaction_rule_requires_two_participants() {
        let interaction_rules: Vec<InteractionRule> = vec![boundary_interaction_rule()];
        assert!(interaction_rules.iter().all(|ir| { let ir = ir.clone(); ir.participants.len() >= 2 }), "boundary values should satisfy invariant");
    }

    #[test]
    fn invalid_interaction_rule_requires_two_participants() {
        let interaction_rules: Vec<InteractionRule> = vec![invalid_interaction_rule()];
        assert!(!(interaction_rules.iter().all(|ir| { let ir = ir.clone(); ir.participants.len() >= 2 })), "invalid values should violate invariant");
    }

    #[test]
    fn boundary_i_r_result_targets_own_participant() {
        let interaction_rules: Vec<InteractionRule> = vec![boundary_interaction_rule()];
        assert!(interaction_rules.iter().all(|ir| { let ir = ir.clone(); ir.results.iter().all(|r| { let r = r.clone(); ir.participants.contains(&r.resultParticipant) }) }), "boundary values should satisfy invariant");
    }

    #[test]
    fn invalid_i_r_result_targets_own_participant() {
        let interaction_rules: Vec<InteractionRule> = vec![invalid_interaction_rule()];
        assert!(!(interaction_rules.iter().all(|ir| { let ir = ir.clone(); ir.results.iter().all(|r| { let r = r.clone(); ir.participants.contains(&r.resultParticipant) }) })), "invalid values should violate invariant");
    }

    // --- Cross-tests: fact × operation ---

    #[test]
    #[ignore]
    fn interaction_rule_requires_two_participants_preserved_after_fire_transition() {
        // Verify that InteractionRuleRequiresTwoParticipants holds after fireTransition
        // pre: assert!(/* InteractionRuleRequiresTwoParticipants constraint */);
        // fire_transition(...);
        // post: assert!(/* InteractionRuleRequiresTwoParticipants constraint */);
        todo!("oxidtr: implement cross-test interaction_rule_requires_two_participants_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn interaction_rule_requires_two_participants_preserved_after_deliver_signal() {
        // Verify that InteractionRuleRequiresTwoParticipants holds after deliverSignal
        // pre: assert!(/* InteractionRuleRequiresTwoParticipants constraint */);
        // deliver_signal(...);
        // post: assert!(/* InteractionRuleRequiresTwoParticipants constraint */);
        todo!("oxidtr: implement cross-test interaction_rule_requires_two_participants_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn interaction_rule_requires_two_participants_preserved_after_spawn_entity() {
        // Verify that InteractionRuleRequiresTwoParticipants holds after spawnEntity
        // pre: assert!(/* InteractionRuleRequiresTwoParticipants constraint */);
        // spawn_entity(...);
        // post: assert!(/* InteractionRuleRequiresTwoParticipants constraint */);
        todo!("oxidtr: implement cross-test interaction_rule_requires_two_participants_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn interaction_rule_requires_two_participants_preserved_after_activate_state_machine() {
        // Verify that InteractionRuleRequiresTwoParticipants holds after activateStateMachine
        // pre: assert!(/* InteractionRuleRequiresTwoParticipants constraint */);
        // activate_state_machine(...);
        // post: assert!(/* InteractionRuleRequiresTwoParticipants constraint */);
        todo!("oxidtr: implement cross-test interaction_rule_requires_two_participants_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn interaction_rule_requires_two_participants_preserved_after_deactivate_state_machine() {
        // Verify that InteractionRuleRequiresTwoParticipants holds after deactivateStateMachine
        // pre: assert!(/* InteractionRuleRequiresTwoParticipants constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* InteractionRuleRequiresTwoParticipants constraint */);
        todo!("oxidtr: implement cross-test interaction_rule_requires_two_participants_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn i_r_result_targets_own_participant_preserved_after_fire_transition() {
        // Verify that IRResultTargetsOwnParticipant holds after fireTransition
        // pre: assert!(/* IRResultTargetsOwnParticipant constraint */);
        // fire_transition(...);
        // post: assert!(/* IRResultTargetsOwnParticipant constraint */);
        todo!("oxidtr: implement cross-test i_r_result_targets_own_participant_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn i_r_result_targets_own_participant_preserved_after_deliver_signal() {
        // Verify that IRResultTargetsOwnParticipant holds after deliverSignal
        // pre: assert!(/* IRResultTargetsOwnParticipant constraint */);
        // deliver_signal(...);
        // post: assert!(/* IRResultTargetsOwnParticipant constraint */);
        todo!("oxidtr: implement cross-test i_r_result_targets_own_participant_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn i_r_result_targets_own_participant_preserved_after_spawn_entity() {
        // Verify that IRResultTargetsOwnParticipant holds after spawnEntity
        // pre: assert!(/* IRResultTargetsOwnParticipant constraint */);
        // spawn_entity(...);
        // post: assert!(/* IRResultTargetsOwnParticipant constraint */);
        todo!("oxidtr: implement cross-test i_r_result_targets_own_participant_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn i_r_result_targets_own_participant_preserved_after_activate_state_machine() {
        // Verify that IRResultTargetsOwnParticipant holds after activateStateMachine
        // pre: assert!(/* IRResultTargetsOwnParticipant constraint */);
        // activate_state_machine(...);
        // post: assert!(/* IRResultTargetsOwnParticipant constraint */);
        todo!("oxidtr: implement cross-test i_r_result_targets_own_participant_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn i_r_result_targets_own_participant_preserved_after_deactivate_state_machine() {
        // Verify that IRResultTargetsOwnParticipant holds after deactivateStateMachine
        // pre: assert!(/* IRResultTargetsOwnParticipant constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* IRResultTargetsOwnParticipant constraint */);
        todo!("oxidtr: implement cross-test i_r_result_targets_own_participant_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn i_r_result_signal_type_consistent_preserved_after_fire_transition() {
        // Verify that IRResultSignalTypeConsistent holds after fireTransition
        // pre: assert!(/* IRResultSignalTypeConsistent constraint */);
        // fire_transition(...);
        // post: assert!(/* IRResultSignalTypeConsistent constraint */);
        todo!("oxidtr: implement cross-test i_r_result_signal_type_consistent_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn i_r_result_signal_type_consistent_preserved_after_deliver_signal() {
        // Verify that IRResultSignalTypeConsistent holds after deliverSignal
        // pre: assert!(/* IRResultSignalTypeConsistent constraint */);
        // deliver_signal(...);
        // post: assert!(/* IRResultSignalTypeConsistent constraint */);
        todo!("oxidtr: implement cross-test i_r_result_signal_type_consistent_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn i_r_result_signal_type_consistent_preserved_after_spawn_entity() {
        // Verify that IRResultSignalTypeConsistent holds after spawnEntity
        // pre: assert!(/* IRResultSignalTypeConsistent constraint */);
        // spawn_entity(...);
        // post: assert!(/* IRResultSignalTypeConsistent constraint */);
        todo!("oxidtr: implement cross-test i_r_result_signal_type_consistent_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn i_r_result_signal_type_consistent_preserved_after_activate_state_machine() {
        // Verify that IRResultSignalTypeConsistent holds after activateStateMachine
        // pre: assert!(/* IRResultSignalTypeConsistent constraint */);
        // activate_state_machine(...);
        // post: assert!(/* IRResultSignalTypeConsistent constraint */);
        todo!("oxidtr: implement cross-test i_r_result_signal_type_consistent_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn i_r_result_signal_type_consistent_preserved_after_deactivate_state_machine() {
        // Verify that IRResultSignalTypeConsistent holds after deactivateStateMachine
        // pre: assert!(/* IRResultSignalTypeConsistent constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* IRResultSignalTypeConsistent constraint */);
        todo!("oxidtr: implement cross-test i_r_result_signal_type_consistent_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn named_table_names_unique_preserved_after_fire_transition() {
        // Verify that NamedTableNamesUnique holds after fireTransition
        // pre: assert!(/* NamedTableNamesUnique constraint */);
        // fire_transition(...);
        // post: assert!(/* NamedTableNamesUnique constraint */);
        todo!("oxidtr: implement cross-test named_table_names_unique_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn named_table_names_unique_preserved_after_deliver_signal() {
        // Verify that NamedTableNamesUnique holds after deliverSignal
        // pre: assert!(/* NamedTableNamesUnique constraint */);
        // deliver_signal(...);
        // post: assert!(/* NamedTableNamesUnique constraint */);
        todo!("oxidtr: implement cross-test named_table_names_unique_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn named_table_names_unique_preserved_after_spawn_entity() {
        // Verify that NamedTableNamesUnique holds after spawnEntity
        // pre: assert!(/* NamedTableNamesUnique constraint */);
        // spawn_entity(...);
        // post: assert!(/* NamedTableNamesUnique constraint */);
        todo!("oxidtr: implement cross-test named_table_names_unique_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn named_table_names_unique_preserved_after_activate_state_machine() {
        // Verify that NamedTableNamesUnique holds after activateStateMachine
        // pre: assert!(/* NamedTableNamesUnique constraint */);
        // activate_state_machine(...);
        // post: assert!(/* NamedTableNamesUnique constraint */);
        todo!("oxidtr: implement cross-test named_table_names_unique_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn named_table_names_unique_preserved_after_deactivate_state_machine() {
        // Verify that NamedTableNamesUnique holds after deactivateStateMachine
        // pre: assert!(/* NamedTableNamesUnique constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* NamedTableNamesUnique constraint */);
        todo!("oxidtr: implement cross-test named_table_names_unique_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn no_cyclic_bin_op_preserved_after_fire_transition() {
        // Verify that NoCyclicBinOp holds after fireTransition
        // pre: assert!(/* NoCyclicBinOp constraint */);
        // fire_transition(...);
        // post: assert!(/* NoCyclicBinOp constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_bin_op_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn no_cyclic_bin_op_preserved_after_deliver_signal() {
        // Verify that NoCyclicBinOp holds after deliverSignal
        // pre: assert!(/* NoCyclicBinOp constraint */);
        // deliver_signal(...);
        // post: assert!(/* NoCyclicBinOp constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_bin_op_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn no_cyclic_bin_op_preserved_after_spawn_entity() {
        // Verify that NoCyclicBinOp holds after spawnEntity
        // pre: assert!(/* NoCyclicBinOp constraint */);
        // spawn_entity(...);
        // post: assert!(/* NoCyclicBinOp constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_bin_op_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn no_cyclic_bin_op_preserved_after_activate_state_machine() {
        // Verify that NoCyclicBinOp holds after activateStateMachine
        // pre: assert!(/* NoCyclicBinOp constraint */);
        // activate_state_machine(...);
        // post: assert!(/* NoCyclicBinOp constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_bin_op_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn no_cyclic_bin_op_preserved_after_deactivate_state_machine() {
        // Verify that NoCyclicBinOp holds after deactivateStateMachine
        // pre: assert!(/* NoCyclicBinOp constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* NoCyclicBinOp constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_bin_op_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn no_cyclic_expr_if_preserved_after_fire_transition() {
        // Verify that NoCyclicExprIf holds after fireTransition
        // pre: assert!(/* NoCyclicExprIf constraint */);
        // fire_transition(...);
        // post: assert!(/* NoCyclicExprIf constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_expr_if_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn no_cyclic_expr_if_preserved_after_deliver_signal() {
        // Verify that NoCyclicExprIf holds after deliverSignal
        // pre: assert!(/* NoCyclicExprIf constraint */);
        // deliver_signal(...);
        // post: assert!(/* NoCyclicExprIf constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_expr_if_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn no_cyclic_expr_if_preserved_after_spawn_entity() {
        // Verify that NoCyclicExprIf holds after spawnEntity
        // pre: assert!(/* NoCyclicExprIf constraint */);
        // spawn_entity(...);
        // post: assert!(/* NoCyclicExprIf constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_expr_if_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn no_cyclic_expr_if_preserved_after_activate_state_machine() {
        // Verify that NoCyclicExprIf holds after activateStateMachine
        // pre: assert!(/* NoCyclicExprIf constraint */);
        // activate_state_machine(...);
        // post: assert!(/* NoCyclicExprIf constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_expr_if_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn no_cyclic_expr_if_preserved_after_deactivate_state_machine() {
        // Verify that NoCyclicExprIf holds after deactivateStateMachine
        // pre: assert!(/* NoCyclicExprIf constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* NoCyclicExprIf constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_expr_if_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn no_cyclic_expr_not_preserved_after_fire_transition() {
        // Verify that NoCyclicExprNot holds after fireTransition
        // pre: assert!(/* NoCyclicExprNot constraint */);
        // fire_transition(...);
        // post: assert!(/* NoCyclicExprNot constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_expr_not_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn no_cyclic_expr_not_preserved_after_deliver_signal() {
        // Verify that NoCyclicExprNot holds after deliverSignal
        // pre: assert!(/* NoCyclicExprNot constraint */);
        // deliver_signal(...);
        // post: assert!(/* NoCyclicExprNot constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_expr_not_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn no_cyclic_expr_not_preserved_after_spawn_entity() {
        // Verify that NoCyclicExprNot holds after spawnEntity
        // pre: assert!(/* NoCyclicExprNot constraint */);
        // spawn_entity(...);
        // post: assert!(/* NoCyclicExprNot constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_expr_not_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn no_cyclic_expr_not_preserved_after_activate_state_machine() {
        // Verify that NoCyclicExprNot holds after activateStateMachine
        // pre: assert!(/* NoCyclicExprNot constraint */);
        // activate_state_machine(...);
        // post: assert!(/* NoCyclicExprNot constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_expr_not_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn no_cyclic_expr_not_preserved_after_deactivate_state_machine() {
        // Verify that NoCyclicExprNot holds after deactivateStateMachine
        // pre: assert!(/* NoCyclicExprNot constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* NoCyclicExprNot constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_expr_not_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn no_cyclic_table_ref_preserved_after_fire_transition() {
        // Verify that NoCyclicTableRef holds after fireTransition
        // pre: assert!(/* NoCyclicTableRef constraint */);
        // fire_transition(...);
        // post: assert!(/* NoCyclicTableRef constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_table_ref_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn no_cyclic_table_ref_preserved_after_deliver_signal() {
        // Verify that NoCyclicTableRef holds after deliverSignal
        // pre: assert!(/* NoCyclicTableRef constraint */);
        // deliver_signal(...);
        // post: assert!(/* NoCyclicTableRef constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_table_ref_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn no_cyclic_table_ref_preserved_after_spawn_entity() {
        // Verify that NoCyclicTableRef holds after spawnEntity
        // pre: assert!(/* NoCyclicTableRef constraint */);
        // spawn_entity(...);
        // post: assert!(/* NoCyclicTableRef constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_table_ref_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn no_cyclic_table_ref_preserved_after_activate_state_machine() {
        // Verify that NoCyclicTableRef holds after activateStateMachine
        // pre: assert!(/* NoCyclicTableRef constraint */);
        // activate_state_machine(...);
        // post: assert!(/* NoCyclicTableRef constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_table_ref_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn no_cyclic_table_ref_preserved_after_deactivate_state_machine() {
        // Verify that NoCyclicTableRef holds after deactivateStateMachine
        // pre: assert!(/* NoCyclicTableRef constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* NoCyclicTableRef constraint */);
        todo!("oxidtr: implement cross-test no_cyclic_table_ref_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn port_promotion_ownership_preserved_after_fire_transition() {
        // Verify that PortPromotionOwnership holds after fireTransition
        // pre: assert!(/* PortPromotionOwnership constraint */);
        // fire_transition(...);
        // post: assert!(/* PortPromotionOwnership constraint */);
        todo!("oxidtr: implement cross-test port_promotion_ownership_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn port_promotion_ownership_preserved_after_deliver_signal() {
        // Verify that PortPromotionOwnership holds after deliverSignal
        // pre: assert!(/* PortPromotionOwnership constraint */);
        // deliver_signal(...);
        // post: assert!(/* PortPromotionOwnership constraint */);
        todo!("oxidtr: implement cross-test port_promotion_ownership_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn port_promotion_ownership_preserved_after_spawn_entity() {
        // Verify that PortPromotionOwnership holds after spawnEntity
        // pre: assert!(/* PortPromotionOwnership constraint */);
        // spawn_entity(...);
        // post: assert!(/* PortPromotionOwnership constraint */);
        todo!("oxidtr: implement cross-test port_promotion_ownership_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn port_promotion_ownership_preserved_after_activate_state_machine() {
        // Verify that PortPromotionOwnership holds after activateStateMachine
        // pre: assert!(/* PortPromotionOwnership constraint */);
        // activate_state_machine(...);
        // post: assert!(/* PortPromotionOwnership constraint */);
        todo!("oxidtr: implement cross-test port_promotion_ownership_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn port_promotion_ownership_preserved_after_deactivate_state_machine() {
        // Verify that PortPromotionOwnership holds after deactivateStateMachine
        // pre: assert!(/* PortPromotionOwnership constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* PortPromotionOwnership constraint */);
        todo!("oxidtr: implement cross-test port_promotion_ownership_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn port_promotion_output_only_preserved_after_fire_transition() {
        // Verify that PortPromotionOutputOnly holds after fireTransition
        // pre: assert!(/* PortPromotionOutputOnly constraint */);
        // fire_transition(...);
        // post: assert!(/* PortPromotionOutputOnly constraint */);
        todo!("oxidtr: implement cross-test port_promotion_output_only_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn port_promotion_output_only_preserved_after_deliver_signal() {
        // Verify that PortPromotionOutputOnly holds after deliverSignal
        // pre: assert!(/* PortPromotionOutputOnly constraint */);
        // deliver_signal(...);
        // post: assert!(/* PortPromotionOutputOnly constraint */);
        todo!("oxidtr: implement cross-test port_promotion_output_only_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn port_promotion_output_only_preserved_after_spawn_entity() {
        // Verify that PortPromotionOutputOnly holds after spawnEntity
        // pre: assert!(/* PortPromotionOutputOnly constraint */);
        // spawn_entity(...);
        // post: assert!(/* PortPromotionOutputOnly constraint */);
        todo!("oxidtr: implement cross-test port_promotion_output_only_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn port_promotion_output_only_preserved_after_activate_state_machine() {
        // Verify that PortPromotionOutputOnly holds after activateStateMachine
        // pre: assert!(/* PortPromotionOutputOnly constraint */);
        // activate_state_machine(...);
        // post: assert!(/* PortPromotionOutputOnly constraint */);
        todo!("oxidtr: implement cross-test port_promotion_output_only_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn port_promotion_output_only_preserved_after_deactivate_state_machine() {
        // Verify that PortPromotionOutputOnly holds after deactivateStateMachine
        // pre: assert!(/* PortPromotionOutputOnly constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* PortPromotionOutputOnly constraint */);
        todo!("oxidtr: implement cross-test port_promotion_output_only_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn initial_state_owned_preserved_after_fire_transition() {
        // Verify that InitialStateOwned holds after fireTransition
        // pre: assert!(/* InitialStateOwned constraint */);
        // fire_transition(...);
        // post: assert!(/* InitialStateOwned constraint */);
        todo!("oxidtr: implement cross-test initial_state_owned_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn initial_state_owned_preserved_after_deliver_signal() {
        // Verify that InitialStateOwned holds after deliverSignal
        // pre: assert!(/* InitialStateOwned constraint */);
        // deliver_signal(...);
        // post: assert!(/* InitialStateOwned constraint */);
        todo!("oxidtr: implement cross-test initial_state_owned_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn initial_state_owned_preserved_after_spawn_entity() {
        // Verify that InitialStateOwned holds after spawnEntity
        // pre: assert!(/* InitialStateOwned constraint */);
        // spawn_entity(...);
        // post: assert!(/* InitialStateOwned constraint */);
        todo!("oxidtr: implement cross-test initial_state_owned_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn initial_state_owned_preserved_after_activate_state_machine() {
        // Verify that InitialStateOwned holds after activateStateMachine
        // pre: assert!(/* InitialStateOwned constraint */);
        // activate_state_machine(...);
        // post: assert!(/* InitialStateOwned constraint */);
        todo!("oxidtr: implement cross-test initial_state_owned_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn initial_state_owned_preserved_after_deactivate_state_machine() {
        // Verify that InitialStateOwned holds after deactivateStateMachine
        // pre: assert!(/* InitialStateOwned constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* InitialStateOwned constraint */);
        todo!("oxidtr: implement cross-test initial_state_owned_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn active_state_owned_preserved_after_fire_transition() {
        // Verify that ActiveStateOwned holds after fireTransition
        // pre: assert!(/* ActiveStateOwned constraint */);
        // fire_transition(...);
        // post: assert!(/* ActiveStateOwned constraint */);
        todo!("oxidtr: implement cross-test active_state_owned_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn active_state_owned_preserved_after_deliver_signal() {
        // Verify that ActiveStateOwned holds after deliverSignal
        // pre: assert!(/* ActiveStateOwned constraint */);
        // deliver_signal(...);
        // post: assert!(/* ActiveStateOwned constraint */);
        todo!("oxidtr: implement cross-test active_state_owned_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn active_state_owned_preserved_after_spawn_entity() {
        // Verify that ActiveStateOwned holds after spawnEntity
        // pre: assert!(/* ActiveStateOwned constraint */);
        // spawn_entity(...);
        // post: assert!(/* ActiveStateOwned constraint */);
        todo!("oxidtr: implement cross-test active_state_owned_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn active_state_owned_preserved_after_activate_state_machine() {
        // Verify that ActiveStateOwned holds after activateStateMachine
        // pre: assert!(/* ActiveStateOwned constraint */);
        // activate_state_machine(...);
        // post: assert!(/* ActiveStateOwned constraint */);
        todo!("oxidtr: implement cross-test active_state_owned_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn active_state_owned_preserved_after_deactivate_state_machine() {
        // Verify that ActiveStateOwned holds after deactivateStateMachine
        // pre: assert!(/* ActiveStateOwned constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* ActiveStateOwned constraint */);
        todo!("oxidtr: implement cross-test active_state_owned_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn transition_states_owned_preserved_after_fire_transition() {
        // Verify that TransitionStatesOwned holds after fireTransition
        // pre: assert!(/* TransitionStatesOwned constraint */);
        // fire_transition(...);
        // post: assert!(/* TransitionStatesOwned constraint */);
        todo!("oxidtr: implement cross-test transition_states_owned_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn transition_states_owned_preserved_after_deliver_signal() {
        // Verify that TransitionStatesOwned holds after deliverSignal
        // pre: assert!(/* TransitionStatesOwned constraint */);
        // deliver_signal(...);
        // post: assert!(/* TransitionStatesOwned constraint */);
        todo!("oxidtr: implement cross-test transition_states_owned_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn transition_states_owned_preserved_after_spawn_entity() {
        // Verify that TransitionStatesOwned holds after spawnEntity
        // pre: assert!(/* TransitionStatesOwned constraint */);
        // spawn_entity(...);
        // post: assert!(/* TransitionStatesOwned constraint */);
        todo!("oxidtr: implement cross-test transition_states_owned_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn transition_states_owned_preserved_after_activate_state_machine() {
        // Verify that TransitionStatesOwned holds after activateStateMachine
        // pre: assert!(/* TransitionStatesOwned constraint */);
        // activate_state_machine(...);
        // post: assert!(/* TransitionStatesOwned constraint */);
        todo!("oxidtr: implement cross-test transition_states_owned_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn transition_states_owned_preserved_after_deactivate_state_machine() {
        // Verify that TransitionStatesOwned holds after deactivateStateMachine
        // pre: assert!(/* TransitionStatesOwned constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* TransitionStatesOwned constraint */);
        todo!("oxidtr: implement cross-test transition_states_owned_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn unique_transition_priority_preserved_after_fire_transition() {
        // Verify that UniqueTransitionPriority holds after fireTransition
        // pre: assert!(/* UniqueTransitionPriority constraint */);
        // fire_transition(...);
        // post: assert!(/* UniqueTransitionPriority constraint */);
        todo!("oxidtr: implement cross-test unique_transition_priority_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn unique_transition_priority_preserved_after_deliver_signal() {
        // Verify that UniqueTransitionPriority holds after deliverSignal
        // pre: assert!(/* UniqueTransitionPriority constraint */);
        // deliver_signal(...);
        // post: assert!(/* UniqueTransitionPriority constraint */);
        todo!("oxidtr: implement cross-test unique_transition_priority_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn unique_transition_priority_preserved_after_spawn_entity() {
        // Verify that UniqueTransitionPriority holds after spawnEntity
        // pre: assert!(/* UniqueTransitionPriority constraint */);
        // spawn_entity(...);
        // post: assert!(/* UniqueTransitionPriority constraint */);
        todo!("oxidtr: implement cross-test unique_transition_priority_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn unique_transition_priority_preserved_after_activate_state_machine() {
        // Verify that UniqueTransitionPriority holds after activateStateMachine
        // pre: assert!(/* UniqueTransitionPriority constraint */);
        // activate_state_machine(...);
        // post: assert!(/* UniqueTransitionPriority constraint */);
        todo!("oxidtr: implement cross-test unique_transition_priority_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn unique_transition_priority_preserved_after_deactivate_state_machine() {
        // Verify that UniqueTransitionPriority holds after deactivateStateMachine
        // pre: assert!(/* UniqueTransitionPriority constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* UniqueTransitionPriority constraint */);
        todo!("oxidtr: implement cross-test unique_transition_priority_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn connection_directionality_preserved_after_fire_transition() {
        // Verify that ConnectionDirectionality holds after fireTransition
        // pre: assert!(/* ConnectionDirectionality constraint */);
        // fire_transition(...);
        // post: assert!(/* ConnectionDirectionality constraint */);
        todo!("oxidtr: implement cross-test connection_directionality_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn connection_directionality_preserved_after_deliver_signal() {
        // Verify that ConnectionDirectionality holds after deliverSignal
        // pre: assert!(/* ConnectionDirectionality constraint */);
        // deliver_signal(...);
        // post: assert!(/* ConnectionDirectionality constraint */);
        todo!("oxidtr: implement cross-test connection_directionality_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn connection_directionality_preserved_after_spawn_entity() {
        // Verify that ConnectionDirectionality holds after spawnEntity
        // pre: assert!(/* ConnectionDirectionality constraint */);
        // spawn_entity(...);
        // post: assert!(/* ConnectionDirectionality constraint */);
        todo!("oxidtr: implement cross-test connection_directionality_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn connection_directionality_preserved_after_activate_state_machine() {
        // Verify that ConnectionDirectionality holds after activateStateMachine
        // pre: assert!(/* ConnectionDirectionality constraint */);
        // activate_state_machine(...);
        // post: assert!(/* ConnectionDirectionality constraint */);
        todo!("oxidtr: implement cross-test connection_directionality_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn connection_directionality_preserved_after_deactivate_state_machine() {
        // Verify that ConnectionDirectionality holds after deactivateStateMachine
        // pre: assert!(/* ConnectionDirectionality constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* ConnectionDirectionality constraint */);
        todo!("oxidtr: implement cross-test connection_directionality_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn connection_signal_type_compat_preserved_after_fire_transition() {
        // Verify that ConnectionSignalTypeCompat holds after fireTransition
        // pre: assert!(/* ConnectionSignalTypeCompat constraint */);
        // fire_transition(...);
        // post: assert!(/* ConnectionSignalTypeCompat constraint */);
        todo!("oxidtr: implement cross-test connection_signal_type_compat_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn connection_signal_type_compat_preserved_after_deliver_signal() {
        // Verify that ConnectionSignalTypeCompat holds after deliverSignal
        // pre: assert!(/* ConnectionSignalTypeCompat constraint */);
        // deliver_signal(...);
        // post: assert!(/* ConnectionSignalTypeCompat constraint */);
        todo!("oxidtr: implement cross-test connection_signal_type_compat_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn connection_signal_type_compat_preserved_after_spawn_entity() {
        // Verify that ConnectionSignalTypeCompat holds after spawnEntity
        // pre: assert!(/* ConnectionSignalTypeCompat constraint */);
        // spawn_entity(...);
        // post: assert!(/* ConnectionSignalTypeCompat constraint */);
        todo!("oxidtr: implement cross-test connection_signal_type_compat_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn connection_signal_type_compat_preserved_after_activate_state_machine() {
        // Verify that ConnectionSignalTypeCompat holds after activateStateMachine
        // pre: assert!(/* ConnectionSignalTypeCompat constraint */);
        // activate_state_machine(...);
        // post: assert!(/* ConnectionSignalTypeCompat constraint */);
        todo!("oxidtr: implement cross-test connection_signal_type_compat_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn connection_signal_type_compat_preserved_after_deactivate_state_machine() {
        // Verify that ConnectionSignalTypeCompat holds after deactivateStateMachine
        // pre: assert!(/* ConnectionSignalTypeCompat constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* ConnectionSignalTypeCompat constraint */);
        todo!("oxidtr: implement cross-test connection_signal_type_compat_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn entity_has_machines_preserved_after_fire_transition() {
        // Verify that EntityHasMachines holds after fireTransition
        // pre: assert!(/* EntityHasMachines constraint */);
        // fire_transition(...);
        // post: assert!(/* EntityHasMachines constraint */);
        todo!("oxidtr: implement cross-test entity_has_machines_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn entity_has_machines_preserved_after_deliver_signal() {
        // Verify that EntityHasMachines holds after deliverSignal
        // pre: assert!(/* EntityHasMachines constraint */);
        // deliver_signal(...);
        // post: assert!(/* EntityHasMachines constraint */);
        todo!("oxidtr: implement cross-test entity_has_machines_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn entity_has_machines_preserved_after_spawn_entity() {
        // Verify that EntityHasMachines holds after spawnEntity
        // pre: assert!(/* EntityHasMachines constraint */);
        // spawn_entity(...);
        // post: assert!(/* EntityHasMachines constraint */);
        todo!("oxidtr: implement cross-test entity_has_machines_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn entity_has_machines_preserved_after_activate_state_machine() {
        // Verify that EntityHasMachines holds after activateStateMachine
        // pre: assert!(/* EntityHasMachines constraint */);
        // activate_state_machine(...);
        // post: assert!(/* EntityHasMachines constraint */);
        todo!("oxidtr: implement cross-test entity_has_machines_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn entity_has_machines_preserved_after_deactivate_state_machine() {
        // Verify that EntityHasMachines holds after deactivateStateMachine
        // pre: assert!(/* EntityHasMachines constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* EntityHasMachines constraint */);
        todo!("oxidtr: implement cross-test entity_has_machines_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn active_set_subset_of_entities_preserved_after_fire_transition() {
        // Verify that ActiveSetSubsetOfEntities holds after fireTransition
        // pre: assert!(/* ActiveSetSubsetOfEntities constraint */);
        // fire_transition(...);
        // post: assert!(/* ActiveSetSubsetOfEntities constraint */);
        todo!("oxidtr: implement cross-test active_set_subset_of_entities_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn active_set_subset_of_entities_preserved_after_deliver_signal() {
        // Verify that ActiveSetSubsetOfEntities holds after deliverSignal
        // pre: assert!(/* ActiveSetSubsetOfEntities constraint */);
        // deliver_signal(...);
        // post: assert!(/* ActiveSetSubsetOfEntities constraint */);
        todo!("oxidtr: implement cross-test active_set_subset_of_entities_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn active_set_subset_of_entities_preserved_after_spawn_entity() {
        // Verify that ActiveSetSubsetOfEntities holds after spawnEntity
        // pre: assert!(/* ActiveSetSubsetOfEntities constraint */);
        // spawn_entity(...);
        // post: assert!(/* ActiveSetSubsetOfEntities constraint */);
        todo!("oxidtr: implement cross-test active_set_subset_of_entities_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn active_set_subset_of_entities_preserved_after_activate_state_machine() {
        // Verify that ActiveSetSubsetOfEntities holds after activateStateMachine
        // pre: assert!(/* ActiveSetSubsetOfEntities constraint */);
        // activate_state_machine(...);
        // post: assert!(/* ActiveSetSubsetOfEntities constraint */);
        todo!("oxidtr: implement cross-test active_set_subset_of_entities_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn active_set_subset_of_entities_preserved_after_deactivate_state_machine() {
        // Verify that ActiveSetSubsetOfEntities holds after deactivateStateMachine
        // pre: assert!(/* ActiveSetSubsetOfEntities constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* ActiveSetSubsetOfEntities constraint */);
        todo!("oxidtr: implement cross-test active_set_subset_of_entities_preserved_after_deactivate_state_machine");
    }

    #[test]
    #[ignore]
    fn sub_machine_does_not_own_parent_state_preserved_after_fire_transition() {
        // Verify that SubMachineDoesNotOwnParentState holds after fireTransition
        // pre: assert!(/* SubMachineDoesNotOwnParentState constraint */);
        // fire_transition(...);
        // post: assert!(/* SubMachineDoesNotOwnParentState constraint */);
        todo!("oxidtr: implement cross-test sub_machine_does_not_own_parent_state_preserved_after_fire_transition");
    }

    #[test]
    #[ignore]
    fn sub_machine_does_not_own_parent_state_preserved_after_deliver_signal() {
        // Verify that SubMachineDoesNotOwnParentState holds after deliverSignal
        // pre: assert!(/* SubMachineDoesNotOwnParentState constraint */);
        // deliver_signal(...);
        // post: assert!(/* SubMachineDoesNotOwnParentState constraint */);
        todo!("oxidtr: implement cross-test sub_machine_does_not_own_parent_state_preserved_after_deliver_signal");
    }

    #[test]
    #[ignore]
    fn sub_machine_does_not_own_parent_state_preserved_after_spawn_entity() {
        // Verify that SubMachineDoesNotOwnParentState holds after spawnEntity
        // pre: assert!(/* SubMachineDoesNotOwnParentState constraint */);
        // spawn_entity(...);
        // post: assert!(/* SubMachineDoesNotOwnParentState constraint */);
        todo!("oxidtr: implement cross-test sub_machine_does_not_own_parent_state_preserved_after_spawn_entity");
    }

    #[test]
    #[ignore]
    fn sub_machine_does_not_own_parent_state_preserved_after_activate_state_machine() {
        // Verify that SubMachineDoesNotOwnParentState holds after activateStateMachine
        // pre: assert!(/* SubMachineDoesNotOwnParentState constraint */);
        // activate_state_machine(...);
        // post: assert!(/* SubMachineDoesNotOwnParentState constraint */);
        todo!("oxidtr: implement cross-test sub_machine_does_not_own_parent_state_preserved_after_activate_state_machine");
    }

    #[test]
    #[ignore]
    fn sub_machine_does_not_own_parent_state_preserved_after_deactivate_state_machine() {
        // Verify that SubMachineDoesNotOwnParentState holds after deactivateStateMachine
        // pre: assert!(/* SubMachineDoesNotOwnParentState constraint */);
        // deactivate_state_machine(...);
        // post: assert!(/* SubMachineDoesNotOwnParentState constraint */);
        todo!("oxidtr: implement cross-test sub_machine_does_not_own_parent_state_preserved_after_deactivate_state_machine");
    }

}
