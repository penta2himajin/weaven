import type { WeavenSchema } from "./generated/schema";

export type ParseResult =
  | { ok: true; value: WeavenSchema }
  | { ok: false; error: string };

export function parseSchema(json: string): ParseResult {
  let raw: unknown;
  try {
    raw = JSON.parse(json);
  } catch (e) {
    return { ok: false, error: `Invalid JSON: ${e}` };
  }

  if (typeof raw !== "object" || raw === null) {
    return { ok: false, error: "Schema must be a JSON object" };
  }

  const obj = raw as Record<string, unknown>;
  const schema: WeavenSchema = {
    state_machines: Array.isArray(obj.state_machines)
      ? (obj.state_machines.map(fillSmDefaults) as unknown as WeavenSchema["state_machines"])
      : [],
    connections: Array.isArray(obj.connections) ? obj.connections : [],
    named_tables: Array.isArray(obj.named_tables) ? obj.named_tables : [],
    interaction_rules: Array.isArray(obj.interaction_rules) ? obj.interaction_rules : [],
  };

  return { ok: true, value: schema };
}

function fillSmDefaults(raw: Record<string, unknown>): Record<string, unknown> {
  return {
    ...raw,
    transitions: Array.isArray(raw.transitions) ? raw.transitions : [],
    input_ports: Array.isArray(raw.input_ports) ? raw.input_ports : [],
    output_ports: Array.isArray(raw.output_ports) ? raw.output_ports : [],
    elapse_capability: raw.elapse_capability ?? "NonElapsable",
  };
}

export function serializeSchema(schema: WeavenSchema): string {
  return JSON.stringify(schema, null, 2);
}

export function validateSchema(schema: WeavenSchema): string[] {
  const errors: string[] = [];
  const smIds = new Set<number>();

  for (const sm of schema.state_machines) {
    if (smIds.has(sm.id)) {
      errors.push(`Duplicate SM id: ${sm.id}`);
    }
    smIds.add(sm.id);

    if (!sm.states.includes(sm.initial_state)) {
      errors.push(
        `SM(${sm.id}): initial_state ${sm.initial_state} not in states [${sm.states.join(",")}]`,
      );
    }

    for (const t of sm.transitions) {
      if (!sm.states.includes(t.source)) {
        errors.push(
          `SM(${sm.id}): transition ${t.id} source ${t.source} not in states`,
        );
      }
      if (!sm.states.includes(t.target)) {
        errors.push(
          `SM(${sm.id}): transition ${t.id} target ${t.target} not in states`,
        );
      }
    }
  }

  for (const c of schema.connections) {
    if (!smIds.has(c.source_sm)) {
      errors.push(`Connection(${c.id}): source_sm ${c.source_sm} not found`);
    }
    if (!smIds.has(c.target_sm)) {
      errors.push(`Connection(${c.id}): target_sm ${c.target_sm} not found`);
    }
  }

  const irIds = new Set<number>();
  for (const ir of schema.interaction_rules ?? []) {
    if (irIds.has(ir.id)) {
      errors.push(`Duplicate IR id: ${ir.id}`);
    }
    irIds.add(ir.id);
    for (const p of ir.participants) {
      if (!smIds.has(p.sm_id)) {
        errors.push(`IR(${ir.id}): participant sm_id ${p.sm_id} not found`);
      }
    }
  }

  return errors;
}
