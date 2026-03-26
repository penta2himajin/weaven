import { create } from "zustand";
import type {
  WeavenSchema,
  SmSchema,
  TransitionSchema,
  ConnectionSchema,
  PortSchema,
} from "../generated/schema";
import { emptySchema, newSmSchema } from "../generated/schema";

export interface EditorStore {
  schema: WeavenSchema;
  selectedSmId: number | null;
  selectedConnectionId: number | null;
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

  // Connection CRUD
  addConnection(connection: ConnectionSchema): void;
  removeConnection(id: number): void;

  // Port CRUD
  addPort(smId: number, direction: "input" | "output", port: PortSchema): void;

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

export const useEditorStore = create<EditorStore>((set, get) => ({
  schema: emptySchema(),
  selectedSmId: null,
  selectedConnectionId: null,
  dirty: false,

  loadSchema(schema) {
    set({ schema, selectedSmId: null, selectedConnectionId: null, dirty: false });
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

  selectSm(id) {
    set({ selectedSmId: id });
  },

  selectConnection(id) {
    set({ selectedConnectionId: id });
  },

  clearSelection() {
    set({ selectedSmId: null, selectedConnectionId: null });
  },
}));
