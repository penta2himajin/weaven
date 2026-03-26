import { describe, it, expect, beforeEach } from "vitest";
import { useEditorStore } from "./editorStore";
import type { WeavenSchema } from "../generated/schema";

function resetStore() {
  useEditorStore.setState({
    schema: { state_machines: [], connections: [], named_tables: [] },
    selectedSmId: null,
    selectedConnectionId: null,
    dirty: false,
  });
}

const fireSchema: WeavenSchema = {
  state_machines: [
    {
      id: 1,
      states: [0, 1],
      initial_state: 0,
      transitions: [
        { id: 10, source: 0, target: 1, priority: 10, effects: [] },
      ],
      input_ports: [{ id: 0, kind: "Input", signal_type: 0 }],
      output_ports: [{ id: 1, kind: "Output", signal_type: 0 }],
    },
  ],
  connections: [],
  named_tables: [],
};

describe("editorStore", () => {
  beforeEach(resetStore);

  // --- Load schema ---
  describe("loadSchema", () => {
    it("replaces current schema and clears selection", () => {
      useEditorStore.getState().selectSm(99);
      useEditorStore.getState().loadSchema(fireSchema);

      const s = useEditorStore.getState();
      expect(s.schema.state_machines).toHaveLength(1);
      expect(s.schema.state_machines[0].id).toBe(1);
      expect(s.selectedSmId).toBeNull();
      expect(s.dirty).toBe(false);
    });
  });

  // --- SM CRUD ---
  describe("addSm", () => {
    it("adds a new SM with default state [0]", () => {
      useEditorStore.getState().addSm();
      const sms = useEditorStore.getState().schema.state_machines;
      expect(sms).toHaveLength(1);
      expect(sms[0].states).toEqual([0]);
      expect(sms[0].initial_state).toBe(0);
      expect(useEditorStore.getState().dirty).toBe(true);
    });

    it("assigns incrementing IDs", () => {
      const { addSm } = useEditorStore.getState();
      addSm();
      addSm();
      const ids = useEditorStore.getState().schema.state_machines.map((sm) => sm.id);
      expect(ids[0]).not.toBe(ids[1]);
    });
  });

  describe("removeSm", () => {
    it("removes SM by id and clears selection if selected", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().selectSm(1);
      useEditorStore.getState().removeSm(1);

      const s = useEditorStore.getState();
      expect(s.schema.state_machines).toHaveLength(0);
      expect(s.selectedSmId).toBeNull();
      expect(s.dirty).toBe(true);
    });

    it("also removes connections referencing the deleted SM", () => {
      useEditorStore.getState().loadSchema({
        ...fireSchema,
        state_machines: [
          ...fireSchema.state_machines,
          { id: 2, states: [0], initial_state: 0, transitions: [], input_ports: [{ id: 0, kind: "Input", signal_type: 0 }], output_ports: [] },
        ],
        connections: [
          { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0, pipeline: [] },
        ],
      });

      useEditorStore.getState().removeSm(1);
      expect(useEditorStore.getState().schema.connections).toHaveLength(0);
    });
  });

  // --- State CRUD within SM ---
  describe("addState", () => {
    it("adds a state to the given SM", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().addState(1, 2);
      const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
      expect(sm.states).toContain(2);
      expect(useEditorStore.getState().dirty).toBe(true);
    });
  });

  describe("removeState", () => {
    it("removes a state and its transitions", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().removeState(1, 1);
      const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
      expect(sm.states).not.toContain(1);
      // Transition 10 goes source:0→target:1, should be removed
      expect(sm.transitions).toHaveLength(0);
    });

    it("prevents removing the initial state", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().removeState(1, 0);
      const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
      expect(sm.states).toContain(0);
    });
  });

  // --- Transition CRUD ---
  describe("addTransition", () => {
    it("adds a transition to the given SM", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().addTransition(1, { id: 11, source: 1, target: 0, priority: 5, effects: [] });
      const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
      expect(sm.transitions).toHaveLength(2);
      expect(sm.transitions[1].id).toBe(11);
    });
  });

  describe("removeTransition", () => {
    it("removes a transition by id", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().removeTransition(1, 10);
      const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
      expect(sm.transitions).toHaveLength(0);
    });
  });

  // --- Connection CRUD ---
  describe("addConnection", () => {
    it("adds a connection", () => {
      useEditorStore.getState().addConnection({
        id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0, pipeline: [],
      });
      expect(useEditorStore.getState().schema.connections).toHaveLength(1);
      expect(useEditorStore.getState().dirty).toBe(true);
    });
  });

  describe("removeConnection", () => {
    it("removes a connection by id", () => {
      useEditorStore.getState().loadSchema({
        ...fireSchema,
        connections: [
          { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0, pipeline: [] },
        ],
      });
      useEditorStore.getState().removeConnection(1);
      expect(useEditorStore.getState().schema.connections).toHaveLength(0);
    });
  });

  // --- Port CRUD ---
  describe("addPort", () => {
    it("adds an input port to an SM", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().addPort(1, "input", { id: 2, kind: "Input", signal_type: 0 });
      const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
      expect(sm.input_ports).toHaveLength(2);
    });

    it("adds an output port to an SM", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().addPort(1, "output", { id: 2, kind: "Output", signal_type: 0 });
      const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
      expect(sm.output_ports).toHaveLength(2);
    });
  });

  // --- Selection ---
  describe("selection", () => {
    it("selectSm sets selectedSmId", () => {
      useEditorStore.getState().selectSm(5);
      expect(useEditorStore.getState().selectedSmId).toBe(5);
    });

    it("selectConnection sets selectedConnectionId", () => {
      useEditorStore.getState().selectConnection(3);
      expect(useEditorStore.getState().selectedConnectionId).toBe(3);
    });

    it("clearSelection clears both", () => {
      useEditorStore.getState().selectSm(5);
      useEditorStore.getState().selectConnection(3);
      useEditorStore.getState().clearSelection();
      const s = useEditorStore.getState();
      expect(s.selectedSmId).toBeNull();
      expect(s.selectedConnectionId).toBeNull();
    });
  });

  // --- Export ---
  describe("exportJson", () => {
    it("returns schema as JSON string", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      const json = useEditorStore.getState().exportJson();
      const parsed = JSON.parse(json) as WeavenSchema;
      expect(parsed.state_machines[0].id).toBe(1);
    });
  });
});
