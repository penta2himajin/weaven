import { create } from "zustand";
import type {
  WeavenSchema,
  SmSchema,
  TransitionSchema,
  ConnectionSchema,
  PortSchema,
  PipelineStepSchema,
  InteractionRuleSchema,
  NamedTableSchema,
} from "../generated/schema";
import { emptySchema, newSmSchema } from "../generated/schema";

export interface EditorStore {
  schema: WeavenSchema;
  selectedSmId: number | null;
  selectedConnectionId: number | null;
  selectedInteractionRuleId: number | null;
  dirty: boolean;

  // Schema lifecycle
  loadSchema(schema: WeavenSchema): void;
  exportJson(): string;

  // SM CRUD
  addSm(): void;
  removeSm(id: number): void;

  // State CRUD within SM
  addState(smId: number, stateId: number): void;
  removeState(smId: number, stateId: number): void;

  // Transition CRUD
  addTransition(smId: number, transition: TransitionSchema): void;
  removeTransition(smId: number, transitionId: number): void;
  updateTransition(smId: number, transitionId: number, patch: Partial<Omit<TransitionSchema, "id">>): void;

  // Connection CRUD
  addConnection(connection: ConnectionSchema): void;
  removeConnection(id: number): void;
  addConnectionFromDrag(sourceSm: number, sourcePort: number, targetSm: number, targetPort: number): void;
  updateConnectionDelay(connectionId: number, delay: number): void;

  // Pipeline CRUD
  addPipelineStep(connectionId: number, step: PipelineStepSchema): void;
  removePipelineStep(connectionId: number, index: number): void;
  updatePipelineStep(connectionId: number, index: number, step: PipelineStepSchema): void;

  // Port CRUD
  addPort(smId: number, direction: "input" | "output", port: PortSchema): void;

  // Interaction Rule CRUD
  addInteractionRule(): void;
  removeInteractionRule(id: number): void;
  updateInteractionRule(id: number, patch: Partial<Omit<InteractionRuleSchema, "id">>): void;
  selectInteractionRule(id: number | null): void;

  // Named Table CRUD
  addNamedTable(name: string): void;
  removeNamedTable(name: string): void;
  updateNamedTable(name: string, entries: unknown): void;

  // Selection
  selectSm(id: number | null): void;
  selectConnection(id: number | null): void;
  clearSelection(): void;
}

function nextId(existing: { id: number }[]): number {
  if (existing.length === 0) return 1;
  return Math.max(...existing.map((e) => e.id)) + 1;
}

function updateSm(
  schema: WeavenSchema,
  smId: number,
  updater: (sm: SmSchema) => SmSchema,
): WeavenSchema {
  return {
    ...schema,
    state_machines: schema.state_machines.map((sm) =>
      sm.id === smId ? updater(sm) : sm,
    ),
  };
}

function updateConnection(
  schema: WeavenSchema,
  connId: number,
  updater: (c: ConnectionSchema) => ConnectionSchema,
): WeavenSchema {
  return {
    ...schema,
    connections: schema.connections.map((c) =>
      c.id === connId ? updater(c) : c,
    ),
  };
}

export const useEditorStore = create<EditorStore>((set, get) => ({
  schema: emptySchema(),
  selectedSmId: null,
  selectedConnectionId: null,
  selectedInteractionRuleId: null,
  dirty: false,

  loadSchema(schema) {
    const withIR = { ...schema, interaction_rules: schema.interaction_rules ?? [] };
    set({ schema: withIR, selectedSmId: null, selectedConnectionId: null, selectedInteractionRuleId: null, dirty: false });
  },

  exportJson() {
    return JSON.stringify(get().schema, null, 2);
  },

  addSm() {
    set((s) => {
      const id = nextId(s.schema.state_machines);
      return {
        schema: {
          ...s.schema,
          state_machines: [...s.schema.state_machines, newSmSchema(id)],
        },
        dirty: true,
      };
    });
  },

  removeSm(id) {
    set((s) => ({
      schema: {
        ...s.schema,
        state_machines: s.schema.state_machines.filter((sm) => sm.id !== id),
        connections: s.schema.connections.filter(
          (c) => c.source_sm !== id && c.target_sm !== id,
        ),
      },
      selectedSmId: s.selectedSmId === id ? null : s.selectedSmId,
      dirty: true,
    }));
  },

  addState(smId, stateId) {
    set((s) => ({
      schema: updateSm(s.schema, smId, (sm) => ({
        ...sm,
        states: [...sm.states, stateId],
      })),
      dirty: true,
    }));
  },

  removeState(smId, stateId) {
    set((s) => {
      const sm = s.schema.state_machines.find((m) => m.id === smId);
      if (!sm || sm.initial_state === stateId) return s;
      return {
        schema: updateSm(s.schema, smId, (sm) => ({
          ...sm,
          states: sm.states.filter((sid) => sid !== stateId),
          transitions: sm.transitions.filter(
            (t) => t.source !== stateId && t.target !== stateId,
          ),
        })),
        dirty: true,
      };
    });
  },

  addTransition(smId, transition) {
    set((s) => ({
      schema: updateSm(s.schema, smId, (sm) => ({
        ...sm,
        transitions: [...sm.transitions, transition],
      })),
      dirty: true,
    }));
  },

  removeTransition(smId, transitionId) {
    set((s) => ({
      schema: updateSm(s.schema, smId, (sm) => ({
        ...sm,
        transitions: sm.transitions.filter((t) => t.id !== transitionId),
      })),
      dirty: true,
    }));
  },

  updateTransition(smId, transitionId, patch) {
    set((s) => ({
      schema: updateSm(s.schema, smId, (sm) => ({
        ...sm,
        transitions: sm.transitions.map((t) =>
          t.id === transitionId ? { ...t, ...patch } : t,
        ),
      })),
      dirty: true,
    }));
  },

  addConnection(connection) {
    set((s) => ({
      schema: {
        ...s.schema,
        connections: [...s.schema.connections, connection],
      },
      dirty: true,
    }));
  },

  removeConnection(id) {
    set((s) => ({
      schema: {
        ...s.schema,
        connections: s.schema.connections.filter((c) => c.id !== id),
      },
      dirty: true,
    }));
  },

  addConnectionFromDrag(sourceSm, sourcePort, targetSm, targetPort) {
    set((s) => {
      if (sourceSm === targetSm) return s;
      const dup = s.schema.connections.some(
        (c) =>
          c.source_sm === sourceSm &&
          c.source_port === sourcePort &&
          c.target_sm === targetSm &&
          c.target_port === targetPort,
      );
      if (dup) return s;
      const id = nextId(s.schema.connections);
      return {
        schema: {
          ...s.schema,
          connections: [
            ...s.schema.connections,
            {
              id,
              source_sm: sourceSm,
              source_port: sourcePort,
              target_sm: targetSm,
              target_port: targetPort,
              delay_ticks: 0,
              pipeline: [],
            },
          ],
        },
        dirty: true,
      };
    });
  },

  updateConnectionDelay(connectionId, delay) {
    set((s) => ({
      schema: updateConnection(s.schema, connectionId, (c) => ({
        ...c,
        delay_ticks: delay,
      })),
      dirty: true,
    }));
  },

  addPipelineStep(connectionId, step) {
    set((s) => ({
      schema: updateConnection(s.schema, connectionId, (c) => ({
        ...c,
        pipeline: [...c.pipeline, step],
      })),
      dirty: true,
    }));
  },

  removePipelineStep(connectionId, index) {
    set((s) => ({
      schema: updateConnection(s.schema, connectionId, (c) => ({
        ...c,
        pipeline: c.pipeline.filter((_, i) => i !== index),
      })),
      dirty: true,
    }));
  },

  updatePipelineStep(connectionId, index, step) {
    set((s) => ({
      schema: updateConnection(s.schema, connectionId, (c) => ({
        ...c,
        pipeline: c.pipeline.map((s, i) => (i === index ? step : s)),
      })),
      dirty: true,
    }));
  },

  addPort(smId, direction, port) {
    set((s) => ({
      schema: updateSm(s.schema, smId, (sm) => ({
        ...sm,
        ...(direction === "input"
          ? { input_ports: [...sm.input_ports, port] }
          : { output_ports: [...sm.output_ports, port] }),
      })),
      dirty: true,
    }));
  },

  addInteractionRule() {
    set((s) => {
      const id = nextId(s.schema.interaction_rules);
      const rule: InteractionRuleSchema = {
        id,
        participants: [],
        conditions: [],
        effects: [],
      };
      return {
        schema: {
          ...s.schema,
          interaction_rules: [...s.schema.interaction_rules, rule],
        },
        dirty: true,
      };
    });
  },

  removeInteractionRule(id) {
    set((s) => ({
      schema: {
        ...s.schema,
        interaction_rules: s.schema.interaction_rules.filter((r) => r.id !== id),
      },
      selectedInteractionRuleId:
        s.selectedInteractionRuleId === id ? null : s.selectedInteractionRuleId,
      dirty: true,
    }));
  },

  updateInteractionRule(id, patch) {
    set((s) => ({
      schema: {
        ...s.schema,
        interaction_rules: s.schema.interaction_rules.map((r) =>
          r.id === id ? { ...r, ...patch } : r,
        ),
      },
      dirty: true,
    }));
  },

  addNamedTable(name) {
    set((s) => ({
      schema: {
        ...s.schema,
        named_tables: [...s.schema.named_tables, { name, entries: [] }],
      },
      dirty: true,
    }));
  },

  removeNamedTable(name) {
    set((s) => ({
      schema: {
        ...s.schema,
        named_tables: s.schema.named_tables.filter((t) => t.name !== name),
      },
      dirty: true,
    }));
  },

  updateNamedTable(name, entries) {
    set((s) => ({
      schema: {
        ...s.schema,
        named_tables: s.schema.named_tables.map((t) =>
          t.name === name ? { ...t, entries } : t,
        ),
      },
      dirty: true,
    }));
  },

  selectInteractionRule(id) {
    set({ selectedInteractionRuleId: id });
  },

  selectSm(id) {
    set({ selectedSmId: id });
  },

  selectConnection(id) {
    set({ selectedConnectionId: id });
  },

  clearSelection() {
    set({ selectedSmId: null, selectedConnectionId: null, selectedInteractionRuleId: null });
  },
}));
