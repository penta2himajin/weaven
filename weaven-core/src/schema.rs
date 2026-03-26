/// Weaven Schema (§12.3) — declarative JSON format for SM definitions.
///
/// Schema files are the primary artifact game designers author and version-control.
/// They are loaded at startup, validated by generated invariant validators, and
/// compiled into runtime SmDef instances.
///
/// JSON format example:
/// ```json
/// {
///   "state_machines": [{
///     "id": 1,
///     "states": [0, 1, 2],
///     "initial_state": 0,
///     "elapse_capability": "NonElapsable",
///     "transitions": [{
///       "id": 10, "source": 0, "target": 1, "priority": 10,
///       "guard": { "BinOp": { "op": "Gt",
///         "left": { "CtxField": "hp" },
///         "right": { "Num": 0.0 }
///       }},
///       "effects": [{ "Signal": [0, { "Num": 1.0 }] }]
///     }],
///     "input_ports": [{ "id": 0, "kind": "Input", "signal_type": 0 }],
///     "output_ports": [{ "id": 1, "kind": "Output", "signal_type": 0 }]
///   }],
///   "connections": [{
///     "id": 1, "source_sm": 1, "source_port": 1,
///     "target_sm": 2, "target_port": 0, "delay_ticks": 0
///   }],
///   "named_tables": [{
///     "name": "elementDamage",
///     "entries": { "Fire": 1.5, "Water": 0.5 }
///   }]
/// }
/// ```

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::types::*;
use crate::expr::{Expr as RtExpr, BinOpKind, TableRegistry, NamedTableData, TableValue,
                  eval_guard};

// ---------------------------------------------------------------------------
// Schema types (JSON-serializable)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeavenSchema {
    #[serde(default)]
    pub state_machines: Vec<SmSchema>,
    #[serde(default)]
    pub connections: Vec<ConnectionSchema>,
    #[serde(default)]
    pub named_tables: Vec<NamedTableSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmSchema {
    pub id: u32,
    pub states: Vec<u32>,
    pub initial_state: u32,
    #[serde(default)]
    pub elapse_capability: ElapseCapabilitySchema,
    #[serde(default)]
    pub transitions: Vec<TransitionSchema>,
    #[serde(default)]
    pub input_ports: Vec<PortSchema>,
    #[serde(default)]
    pub output_ports: Vec<PortSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum ElapseCapabilitySchema {
    Deterministic,
    Approximate,
    #[default]
    NonElapsable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSchema {
    pub id: u32,
    pub source: u32,
    pub target: u32,
    pub priority: u32,
    #[serde(default)]
    pub guard: Option<ExprSchema>,
    #[serde(default)]
    pub effects: Vec<EffectSchema>,
}

/// Schema representation of an Effect output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EffectSchema {
    /// Emit a signal: [port_id, payload_expr_map]
    Signal { port: u32, payload: BTreeMap<String, ExprSchema> },
    /// Emit a HitStop system command.
    HitStop { frames: u32 },
    /// Emit a SlowMotion system command.
    SlowMotion { factor: f64, duration_ticks: u32 },
    /// Adjust global time scale.
    TimeScale(f64),
    /// Mutate a context field: { field, expr }
    SetContext { field: String, expr: ExprSchema },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortSchema {
    pub id: u32,
    pub kind: PortKindSchema,
    pub signal_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PortKindSchema {
    Input,
    Output,
    ContinuousInput,
    ContinuousOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSchema {
    pub id: u32,
    pub source_sm: u32,
    pub source_port: u32,
    pub target_sm: u32,
    pub target_port: u32,
    #[serde(default)]
    pub delay_ticks: u32,
    #[serde(default)]
    pub pipeline: Vec<PipelineStepSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PipelineStepSchema {
    /// Transform: map of field assignments { field: expr }
    Transform(BTreeMap<String, ExprSchema>),
    /// Filter: boolean expression
    Filter(ExprSchema),
    /// Redirect: target port id
    Redirect(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedTableSchema {
    pub name: String,
    pub entries: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Expression schema (subset of §5 expression language)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ExprSchema {
    Num(f64),
    Bool(bool),
    Str(String),
    CtxField(String),
    SigField(String),
    TableLookup { table: String, keys: Vec<ExprSchema> },
    BinOp { op: BinOpSchema, left: Box<ExprSchema>, right: Box<ExprSchema> },
    Not(Box<ExprSchema>),
    If { cond: Box<ExprSchema>, then_: Box<ExprSchema>, else_: Box<ExprSchema> },
    CollectionAny   { array_field: String, predicate: Box<ExprSchema> },
    CollectionCount { array_field: String, predicate: Box<ExprSchema> },
    CollectionSum   { array_field: String, sum_field: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinOpSchema {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt, Lte, Gte,
    And, Or,
}

// ---------------------------------------------------------------------------
// Schema → runtime conversion
// ---------------------------------------------------------------------------

/// Load a WeavenSchema from JSON string.
pub fn load_schema(json: &str) -> Result<WeavenSchema, serde_json::Error> {
    serde_json::from_str(json)
}

/// Load a WeavenSchema from a JSON file path.
pub fn load_schema_file(path: &str) -> Result<WeavenSchema, Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

/// Compile a WeavenSchema into runtime SmDefs, Connections, and a TableRegistry.
pub fn compile_schema(schema: &WeavenSchema) -> SchemaCompileResult {
    let mut sm_defs = Vec::new();
    let mut connections = Vec::new();
    let mut table_registry = TableRegistry::new();

    // Compile Named Tables
    for t in &schema.named_tables {
        let data = compile_table_value(&t.entries);
        if let TableValue::Table(nested) = data {
            table_registry.register(t.name.clone(), nested);
        }
    }

    // Compile SM definitions
    for sm in &schema.state_machines {
        sm_defs.push(compile_sm(sm, &table_registry));
    }

    // Compile Connections
    for c in &schema.connections {
        connections.push(compile_connection(c));
    }

    SchemaCompileResult { sm_defs, connections, table_registry }
}

pub struct SchemaCompileResult {
    pub sm_defs: Vec<SmDef>,
    pub connections: Vec<Connection>,
    pub table_registry: TableRegistry,
}

fn compile_sm(sm: &SmSchema, tables: &TableRegistry) -> SmDef {
    let transitions = sm.transitions.iter()
        .map(|t| compile_transition(t, tables))
        .collect();

    let input_ports = sm.input_ports.iter().map(compile_port).collect();
    let output_ports = sm.output_ports.iter().map(compile_port).collect();

    SmDef {
        id: SmId(sm.id),
        states: sm.states.iter().map(|&s| StateId(s)).collect(),
        initial_state: StateId(sm.initial_state),
        transitions,
        input_ports,
        output_ports,
        on_despawn_transitions: vec![],
        elapse_capability: match sm.elapse_capability {
            ElapseCapabilitySchema::Deterministic => ElapseCapabilityRt::Deterministic,
            ElapseCapabilitySchema::Approximate   => ElapseCapabilityRt::Approximate,
            ElapseCapabilitySchema::NonElapsable  => ElapseCapabilityRt::NonElapsable,
        },
        elapse_fn: None,
    }
}

fn compile_transition(t: &TransitionSchema, tables: &TableRegistry) -> Transition {
    let guard: Option<GuardFn> = t.guard.as_ref().map(|g| {
        let rt_expr = compile_expr(g);
        // Clone the table registry for use inside the closure.
        let tables_clone = tables.clone();
        let guard_fn: GuardFn = Box::new(move |ctx, signal| {
            eval_guard(&rt_expr, ctx, signal.map(|s| &s.payload), &tables_clone)
        });
        guard_fn
    });

    let effects: Vec<EffectFn> = t.effects.iter()
        .map(|e| compile_effect(e, tables))
        .collect();

    Transition {
        id: TransitionId(t.id),
        source: StateId(t.source),
        target: StateId(t.target),
        priority: t.priority,
        guard,
        effects,
    }
}

fn compile_effect(effect: &EffectSchema, tables: &TableRegistry) -> EffectFn {
    match effect {
        EffectSchema::Signal { port, payload } => {
            let port_id = PortId(*port);
            let payload_exprs: BTreeMap<String, RtExpr> = payload.iter()
                .map(|(k, v)| (k.clone(), compile_expr(v)))
                .collect();
            let tables_clone = tables.clone();
            Box::new(move |ctx| {
                let mut p = BTreeMap::new();
                for (k, expr) in &payload_exprs {
                    let ectx = crate::expr::EvalCtx {
                        context: ctx,
                        signal: None,
                        received_ports: &[],
                        tables: &tables_clone,
                    };
                    p.insert(k.clone(), crate::expr::eval(expr, &ectx));
                }
                vec![EffectOutput::Signal(port_id, Signal {
                    signal_type: SignalTypeId(0), // resolved by port type in full impl
                    payload: p,
                })]
            })
        }
        EffectSchema::SetContext { field, expr } => {
            let field = field.clone();
            let rt_expr = compile_expr(expr);
            let tables_clone = tables.clone();
            Box::new(move |ctx| {
                let ectx = crate::expr::EvalCtx {
                    context: ctx,
                    signal: None,
                    received_ports: &[],
                    tables: &tables_clone,
                };
                let val = crate::expr::eval(&rt_expr, &ectx);
                ctx.set(field.clone(), val);
                vec![]
            })
        }
        EffectSchema::HitStop { frames } => {
            let frames = *frames;
            Box::new(move |_ctx| vec![EffectOutput::Cmd(SystemCommand::HitStop { frames })])
        }
        EffectSchema::SlowMotion { factor, duration_ticks } => {
            let factor = *factor;
            let duration_ticks = *duration_ticks;
            Box::new(move |_ctx| vec![EffectOutput::Cmd(
                SystemCommand::SlowMotion { factor, duration_ticks }
            )])
        }
        EffectSchema::TimeScale(s) => {
            let scale = *s;
            Box::new(move |_ctx| vec![EffectOutput::Cmd(SystemCommand::TimeScale(scale))])
        }
    }
}

fn compile_port(p: &PortSchema) -> Port {
    Port::new(
        PortId(p.id),
        match p.kind {
            PortKindSchema::Input           => PortKind::Input,
            PortKindSchema::Output          => PortKind::Output,
            PortKindSchema::ContinuousInput  => PortKind::ContinuousInput,
            PortKindSchema::ContinuousOutput => PortKind::ContinuousOutput,
        },
        SignalTypeId(p.signal_type),
    )
}

fn compile_connection(c: &ConnectionSchema) -> Connection {
    let pipeline = c.pipeline.iter().map(compile_pipeline_step).collect();
    Connection {
        id: ConnectionId(c.id),
        source_sm: SmId(c.source_sm),
        source_port: PortId(c.source_port),
        target_sm: SmId(c.target_sm),
        target_port: PortId(c.target_port),
        delay_ticks: c.delay_ticks,
        pipeline,
    }
}

fn compile_pipeline_step(step: &PipelineStepSchema) -> PipelineStep {
    match step {
        PipelineStepSchema::Transform(fields) => {
            let field_exprs: BTreeMap<String, RtExpr> = fields.iter()
                .map(|(k, v)| (k.clone(), compile_expr(v)))
                .collect();
            PipelineStep::Transform(Box::new(move |mut sig| {
                // Simple eval: use signal payload as both context and signal
                for (field, expr) in &field_exprs {
                    let dummy_ctx = Context::default();
                    let ectx = crate::expr::EvalCtx {
                        context: &dummy_ctx,
                        signal: Some(&sig.payload),
                        received_ports: &[],
                        tables: &TableRegistry::new(),
                    };
                    sig.payload.insert(field.clone(), crate::expr::eval(expr, &ectx));
                }
                sig
            }))
        }
        PipelineStepSchema::Filter(expr) => {
            let rt_expr = compile_expr(expr);
            PipelineStep::Filter(Box::new(move |sig| {
                let dummy_ctx = Context::default();
                let ectx = crate::expr::EvalCtx {
                    context: &dummy_ctx,
                    signal: Some(&sig.payload),
                    received_ports: &[],
                    tables: &TableRegistry::new(),
                };
                crate::expr::eval_bool(&rt_expr, &ectx)
            }))
        }
        PipelineStepSchema::Redirect(port) => {
            PipelineStep::Redirect(PortId(*port))
        }
    }
}

fn compile_expr(expr: &ExprSchema) -> RtExpr {
    match expr {
        ExprSchema::Num(n)       => RtExpr::Num(*n),
        ExprSchema::Bool(b)      => RtExpr::Bool(*b),
        ExprSchema::Str(s)       => RtExpr::Str(s.clone()),
        ExprSchema::CtxField(f)  => RtExpr::CtxField(f.clone()),
        ExprSchema::SigField(f)  => RtExpr::SigField(f.clone()),
        ExprSchema::TableLookup { table, keys } => RtExpr::TableLookup {
            table: table.clone(),
            keys: keys.iter().map(|k| Box::new(compile_expr(k))).collect(),
        },
        ExprSchema::BinOp { op, left, right } => RtExpr::BinOp {
            op: match op {
                BinOpSchema::Add => BinOpKind::Add,
                BinOpSchema::Sub => BinOpKind::Sub,
                BinOpSchema::Mul => BinOpKind::Mul,
                BinOpSchema::Div => BinOpKind::Div,
                BinOpSchema::Mod => BinOpKind::Mod,
                BinOpSchema::Eq  => BinOpKind::Eq,
                BinOpSchema::Neq => BinOpKind::Neq,
                BinOpSchema::Lt  => BinOpKind::Lt,
                BinOpSchema::Gt  => BinOpKind::Gt,
                BinOpSchema::Lte => BinOpKind::Lte,
                BinOpSchema::Gte => BinOpKind::Gte,
                BinOpSchema::And => BinOpKind::And,
                BinOpSchema::Or  => BinOpKind::Or,
            },
            left: Box::new(compile_expr(left)),
            right: Box::new(compile_expr(right)),
        },
        ExprSchema::Not(inner) => RtExpr::Not(Box::new(compile_expr(inner))),
        ExprSchema::If { cond, then_, else_ } => RtExpr::If {
            cond:  Box::new(compile_expr(cond)),
            then_: Box::new(compile_expr(then_)),
            else_: Box::new(compile_expr(else_)),
        },
        ExprSchema::CollectionAny { array_field, predicate } => RtExpr::CollectionAny {
            array_field: array_field.clone(),
            predicate: Box::new(compile_expr(predicate)),
        },
        ExprSchema::CollectionCount { array_field, predicate } => RtExpr::CollectionCount {
            array_field: array_field.clone(),
            predicate: Box::new(compile_expr(predicate)),
        },
        ExprSchema::CollectionSum { array_field, sum_field } => RtExpr::CollectionSum {
            array_field: array_field.clone(),
            sum_field: sum_field.clone(),
        },
    }
}

fn compile_table_value(value: &serde_json::Value) -> TableValue {
    match value {
        serde_json::Value::Number(n) => TableValue::Num(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::Bool(b)   => TableValue::Bool(*b),
        serde_json::Value::String(s) => TableValue::Str(s.clone()),
        serde_json::Value::Object(map) => {
            let mut data = NamedTableData::new();
            for (k, v) in map {
                data.insert(k.clone(), compile_table_value(v));
            }
            TableValue::Table(data)
        }
        _ => TableValue::Num(0.0),
    }
}
