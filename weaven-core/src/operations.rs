use crate::models::*;

/// @pre: t in sm.transitions
/// @pre: t.source = sm.activeState
pub fn fire_transition(sm: &StateMachine, t: &Transition) {
    todo!("oxidtr: implement fireTransition");
}

/// @pre: s.signalType = c.connSource.signalType
/// @post: c.connSource.portKind = PortKindOutput
/// @post: c.connTarget.portKind = PortKindInput
pub fn deliver_signal(c: &Connection, s: &Signal) {
    todo!("oxidtr: implement deliverSignal");
}

/// @pre: some e.machines
pub fn spawn_entity(e: &Entity) {
    todo!("oxidtr: implement spawnEntity");
}

/// @pre: sm in aset.activeMachines
pub fn activate_state_machine(aset: &ActiveSet, sm: &StateMachine) {
    todo!("oxidtr: implement activateStateMachine");
}

/// @post: no x: aset.activeMachines | x = sm
pub fn deactivate_state_machine(aset: &ActiveSet, sm: &StateMachine) {
    todo!("oxidtr: implement deactivateStateMachine");
}

