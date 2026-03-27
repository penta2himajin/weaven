// Weaven Core — engine-independent pure game logic runtime.
//
// Generated structural types (from weaven.als via oxidtr):
#[allow(dead_code, unused_imports, non_snake_case)]
pub mod models;
#[allow(dead_code, unused_imports, non_snake_case)]
pub mod operations;
#[allow(dead_code, unused_imports, non_snake_case)]
pub mod fixtures;
#[allow(dead_code, unused_imports, non_snake_case)]
pub mod newtypes;

// Runtime implementation:
pub mod types;
pub mod tick;
pub mod expr;
pub mod schema;

pub use types::*;
pub use tick::{tick, TickOutput};
pub use expr::{
    Expr, BinOpKind, EvalCtx, TableRegistry, NamedTableData, TableValue,
    eval, eval_bool, eval_guard,
    eval_traced, eval_guard_traced, EvalTreeNode,
    parse, ParseError,
};
pub mod network;
pub mod spatial;
pub use network::{
    snapshot, restore, diff_snapshots, rewind_and_resimulate,
    policy_filtered_diff, scoped_snapshot, interest_region_sms,
    WorldSnapshot, SmInstanceSnapshot, SmStateDiff,
    TaggedInput, InputBuffer,
    Authority, SyncPolicy, ReconciliationPolicy, SmNetworkPolicy,
};
pub use spatial::{SpatialIndex, proximity, any_within_radius, SpatialConditionFn};
pub mod error;
pub use error::{
    WeavenDiagnostic, CascadeOverflowAction, CascadeOverflowPolicy, TickDiagnostics,
};
pub mod trace;
pub use trace::{TraceEvent, TraceCollector, Phase as TracePhase};
