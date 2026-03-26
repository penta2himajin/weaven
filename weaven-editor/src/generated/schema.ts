/**
 * Weaven Schema types (§12.3) — TypeScript mirror of weaven-core/src/schema.rs.
 *
 * These types define the JSON format that game designers author and version-control.
 * The editor reads/writes this format directly.
 */

// ---------------------------------------------------------------------------
// Top-level schema
// ---------------------------------------------------------------------------

export interface WeavenSchema {
  state_machines: SmSchema[];
  connections: ConnectionSchema[];
  named_tables: NamedTableSchema[];
}

// ---------------------------------------------------------------------------
// State Machine
// ---------------------------------------------------------------------------

export interface SmSchema {
  id: number;
  states: number[];
  initial_state: number;
  elapse_capability?: ElapseCapability;
  transitions: TransitionSchema[];
  input_ports: PortSchema[];
  output_ports: PortSchema[];
}

export type ElapseCapability = "Deterministic" | "Approximate" | "NonElapsable";

// ---------------------------------------------------------------------------
// Transition
// ---------------------------------------------------------------------------

export interface TransitionSchema {
  id: number;
  source: number;
  target: number;
  priority: number;
  guard?: ExprSchema | null;
  effects: EffectSchema[];
}

// ---------------------------------------------------------------------------
// Effect
// ---------------------------------------------------------------------------

export type EffectSchema =
  | { Signal: { port: number; payload: Record<string, ExprSchema> } }
  | { HitStop: { frames: number } }
  | { SlowMotion: { factor: number; duration_ticks: number } }
  | { TimeScale: number }
  | { SetContext: { field: string; expr: ExprSchema } };

// ---------------------------------------------------------------------------
// Port
// ---------------------------------------------------------------------------

export interface PortSchema {
  id: number;
  kind: PortKind;
  signal_type: number;
}

export type PortKind = "Input" | "Output" | "ContinuousInput" | "ContinuousOutput";

// ---------------------------------------------------------------------------
// Connection
// ---------------------------------------------------------------------------

export interface ConnectionSchema {
  id: number;
  source_sm: number;
  source_port: number;
  target_sm: number;
  target_port: number;
  delay_ticks: number;
  pipeline: PipelineStepSchema[];
}

export type PipelineStepSchema =
  | { Transform: Record<string, ExprSchema> }
  | { Filter: ExprSchema }
  | { Redirect: number };

// ---------------------------------------------------------------------------
// Named Table
// ---------------------------------------------------------------------------

export interface NamedTableSchema {
  name: string;
  entries: unknown;
}

// ---------------------------------------------------------------------------
// Expression Language (§5)
// ---------------------------------------------------------------------------

export type ExprSchema =
  | { Num: number }
  | { Bool: boolean }
  | { Str: string }
  | { CtxField: string }
  | { SigField: string }
  | { TableLookup: { table: string; keys: ExprSchema[] } }
  | { BinOp: { op: BinOp; left: ExprSchema; right: ExprSchema } }
  | { Not: ExprSchema }
  | { If: { cond: ExprSchema; then_: ExprSchema; else_: ExprSchema } }
  | { CollectionAny: { array_field: string; predicate: ExprSchema } }
  | { CollectionCount: { array_field: string; predicate: ExprSchema } }
  | { CollectionSum: { array_field: string; sum_field: string } };

export type BinOp =
  | "Add" | "Sub" | "Mul" | "Div" | "Mod"
  | "Eq" | "Neq" | "Lt" | "Gt" | "Lte" | "Gte"
  | "And" | "Or";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export function emptySchema(): WeavenSchema {
  return { state_machines: [], connections: [], named_tables: [] };
}

export function newSmSchema(id: number): SmSchema {
  return {
    id,
    states: [0],
    initial_state: 0,
    elapse_capability: "NonElapsable",
    transitions: [],
    input_ports: [],
    output_ports: [],
  };
}

export function newConnectionSchema(
  id: number,
  sourceSm: number,
  sourcePort: number,
  targetSm: number,
  targetPort: number,
): ConnectionSchema {
  return {
    id,
    source_sm: sourceSm,
    source_port: sourcePort,
    target_sm: targetSm,
    target_port: targetPort,
    delay_ticks: 0,
    pipeline: [],
  };
}
