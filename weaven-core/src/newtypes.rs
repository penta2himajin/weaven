#[allow(unused_imports)]
use crate::models::*;
#[allow(unused_imports)]
use crate::fixtures::*;

/// Newtype wrapper: ActiveSet validated by ActiveSetSubsetOfEntities.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedActiveSet(pub ActiveSet);

impl TryFrom<ActiveSet> for ValidatedActiveSet {
    type Error = &'static str;

    fn try_from(value: ActiveSet) -> Result<Self, Self::Error> {
        let active_sets: Vec<ActiveSet> = vec![value.clone()];
        let entitys: Vec<Entity> = Vec::new();
        if active_sets.iter().all(|aset| { let aset = aset.clone(); aset.activeMachines.iter().all(|sm| { let sm = sm.clone(); entitys.iter().any(|e| { let e = e.clone(); e.machines.contains(&sm) }) }) }) {
            Ok(ValidatedActiveSet(value))
        } else {
            Err("ActiveSetSubsetOfEntities invariant violated")
        }
    }
}

/// Newtype wrapper: Entity validated by ActiveSetSubsetOfEntities.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedEntity(pub Entity);

impl TryFrom<Entity> for ValidatedEntity {
    type Error = &'static str;

    fn try_from(value: Entity) -> Result<Self, Self::Error> {
        let active_sets: Vec<ActiveSet> = Vec::new();
        let entitys: Vec<Entity> = vec![value.clone()];
        if active_sets.iter().all(|aset| { let aset = aset.clone(); aset.activeMachines.iter().all(|sm| { let sm = sm.clone(); entitys.iter().any(|e| { let e = e.clone(); e.machines.contains(&sm) }) }) }) {
            Ok(ValidatedEntity(value))
        } else {
            Err("ActiveSetSubsetOfEntities invariant violated")
        }
    }
}

/// Newtype wrapper: StateMachine validated by ActiveStateOwned.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedStateMachine(pub StateMachine);

impl TryFrom<StateMachine> for ValidatedStateMachine {
    type Error = &'static str;

    fn try_from(value: StateMachine) -> Result<Self, Self::Error> {
        let state_machines: Vec<StateMachine> = vec![value.clone()];
        if state_machines.iter().all(|sm| { let sm = sm.clone(); sm.activeState.as_ref().map_or(true, |s| sm.states.contains(s)) }) {
            Ok(ValidatedStateMachine(value))
        } else {
            Err("ActiveStateOwned invariant violated")
        }
    }
}

/// Newtype wrapper: Connection validated by ConnectionDirectionality.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedConnection(pub Connection);

impl TryFrom<Connection> for ValidatedConnection {
    type Error = &'static str;

    fn try_from(value: Connection) -> Result<Self, Self::Error> {
        let connections: Vec<Connection> = vec![value.clone()];
        if connections.iter().all(|c| { let c = c.clone(); c.connSource.portKind == PortKind::PortKindOutput || c.connSource.portKind == PortKind::PortKindContinuousOutput && c.connTarget.portKind == PortKind::PortKindInput || c.connTarget.portKind == PortKind::PortKindContinuousInput }) {
            Ok(ValidatedConnection(value))
        } else {
            Err("ConnectionDirectionality invariant violated")
        }
    }
}

/// Newtype wrapper: IRResult validated by IRResultSignalTypeConsistent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedIRResult(pub IRResult);

impl TryFrom<IRResult> for ValidatedIRResult {
    type Error = &'static str;

    fn try_from(value: IRResult) -> Result<Self, Self::Error> {
        let i_r_results: Vec<IRResult> = vec![value.clone()];
        if i_r_results.iter().all(|r| { let r = r.clone(); r.resultSignalType == r.resultPort.signalType }) {
            Ok(ValidatedIRResult(value))
        } else {
            Err("IRResultSignalTypeConsistent invariant violated")
        }
    }
}

/// Newtype wrapper: InteractionRule validated by IRResultTargetsOwnParticipant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedInteractionRule(pub InteractionRule);

impl TryFrom<InteractionRule> for ValidatedInteractionRule {
    type Error = &'static str;

    fn try_from(value: InteractionRule) -> Result<Self, Self::Error> {
        if value.participants.len() < 2 {
            return Err("IRResultTargetsOwnParticipant: participants has fewer than 2 elements");
        }
        let interaction_rules: Vec<InteractionRule> = vec![value.clone()];
        if interaction_rules.iter().all(|ir| { let ir = ir.clone(); ir.results.iter().all(|r| { let r = r.clone(); ir.participants.contains(&r.resultParticipant) }) }) {
            Ok(ValidatedInteractionRule(value))
        } else {
            Err("IRResultTargetsOwnParticipant invariant violated")
        }
    }
}

/// Newtype wrapper: NamedTable validated by NamedTableNamesUnique.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedNamedTable(pub NamedTable);

impl TryFrom<NamedTable> for ValidatedNamedTable {
    type Error = &'static str;

    fn try_from(value: NamedTable) -> Result<Self, Self::Error> {
        let named_tables: Vec<NamedTable> = vec![value.clone()];
        if named_tables.iter().all(|t1| { let t1 = t1.clone(); named_tables.iter().all(|t2| { let t2 = t2.clone(); !(t1 != t2) || t1.tableName != t2.tableName }) }) {
            Ok(ValidatedNamedTable(value))
        } else {
            Err("NamedTableNamesUnique invariant violated")
        }
    }
}

/// Newtype wrapper: PortPromotion validated by PortPromotionOutputOnly.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedPortPromotion(pub PortPromotion);

impl TryFrom<PortPromotion> for ValidatedPortPromotion {
    type Error = &'static str;

    fn try_from(value: PortPromotion) -> Result<Self, Self::Error> {
        let port_promotions: Vec<PortPromotion> = vec![value.clone()];
        if port_promotions.iter().all(|pp| { let pp = pp.clone(); (*pp.promotedPort).portKind == PortKind::PortKindOutput || (*pp.promotedPort).portKind == PortKind::PortKindContinuousOutput }) {
            Ok(ValidatedPortPromotion(value))
        } else {
            Err("PortPromotionOutputOnly invariant violated")
        }
    }
}

/// Newtype wrapper: CompoundState validated by SubMachineDoesNotOwnParentState.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatedCompoundState(pub CompoundState);

impl TryFrom<CompoundState> for ValidatedCompoundState {
    type Error = &'static str;

    fn try_from(value: CompoundState) -> Result<Self, Self::Error> {
        let compound_states: Vec<CompoundState> = vec![value.clone()];
        if compound_states.iter().all(|cs| { let cs = cs.clone(); cs.subMachines.iter().all(|sm| { let sm = sm.clone(); !sm.states.iter().any(|s| { let s = s.clone(); s == cs.compoundParent }) }) }) {
            Ok(ValidatedCompoundState(value))
        } else {
            Err("SubMachineDoesNotOwnParentState invariant violated")
        }
    }
}

