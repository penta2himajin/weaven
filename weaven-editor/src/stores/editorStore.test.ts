import { describe, it, expect, beforeEach } from "vitest";
import { useEditorStore } from "./editorStore";
import type { WeavenSchema } from "../generated/schema";

function resetStore() {
  useEditorStore.setState({
    schema: { state_machines: [], connections: [], named_tables: [], interaction_rules: [] },
    selectedSmId: null,
    selectedConnectionId: null,
    selectedInteractionRuleId: null,
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
  interaction_rules: [],
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

  // --- Drag & Drop Connection ---
  describe("addConnectionFromDrag", () => {
    it("creates a connection between two SM ports", () => {
      useEditorStore.getState().loadSchema({
        ...fireSchema,
        state_machines: [
          ...fireSchema.state_machines,
          { id: 2, states: [0], initial_state: 0, transitions: [], input_ports: [{ id: 0, kind: "Input", signal_type: 0 }], output_ports: [] },
        ],
      });
      useEditorStore.getState().addConnectionFromDrag(1, 1, 2, 0);
      const conns = useEditorStore.getState().schema.connections;
      expect(conns).toHaveLength(1);
      expect(conns[0].source_sm).toBe(1);
      expect(conns[0].source_port).toBe(1);
      expect(conns[0].target_sm).toBe(2);
      expect(conns[0].target_port).toBe(0);
      expect(conns[0].delay_ticks).toBe(0);
      expect(conns[0].pipeline).toEqual([]);
      expect(useEditorStore.getState().dirty).toBe(true);
    });

    it("assigns unique IDs to new connections", () => {
      useEditorStore.getState().loadSchema({
        ...fireSchema,
        state_machines: [
          ...fireSchema.state_machines,
          { id: 2, states: [0], initial_state: 0, transitions: [], input_ports: [{ id: 0, kind: "Input", signal_type: 0 }, { id: 2, kind: "Input", signal_type: 0 }], output_ports: [] },
        ],
      });
      useEditorStore.getState().addConnectionFromDrag(1, 1, 2, 0);
      useEditorStore.getState().addConnectionFromDrag(1, 1, 2, 2);
      const conns = useEditorStore.getState().schema.connections;
      expect(conns).toHaveLength(2);
      expect(conns[0].id).not.toBe(conns[1].id);
    });

    it("does not create duplicate connections", () => {
      useEditorStore.getState().loadSchema({
        ...fireSchema,
        state_machines: [
          ...fireSchema.state_machines,
          { id: 2, states: [0], initial_state: 0, transitions: [], input_ports: [{ id: 0, kind: "Input", signal_type: 0 }], output_ports: [] },
        ],
      });
      useEditorStore.getState().addConnectionFromDrag(1, 1, 2, 0);
      useEditorStore.getState().addConnectionFromDrag(1, 1, 2, 0);
      expect(useEditorStore.getState().schema.connections).toHaveLength(1);
    });

    it("does not allow self-connections (same SM)", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().addConnectionFromDrag(1, 1, 1, 0);
      expect(useEditorStore.getState().schema.connections).toHaveLength(0);
    });
  });

  // --- Pipeline Step CRUD ---
  describe("addPipelineStep", () => {
    it("adds a Transform step to a connection", () => {
      useEditorStore.getState().loadSchema({
        ...fireSchema,
        connections: [
          { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0, pipeline: [] },
        ],
      });
      useEditorStore.getState().addPipelineStep(1, { Transform: { value: { Num: 42 } } });
      const conn = useEditorStore.getState().schema.connections.find((c) => c.id === 1)!;
      expect(conn.pipeline).toHaveLength(1);
      expect(useEditorStore.getState().dirty).toBe(true);
    });

    it("adds a Filter step to a connection", () => {
      useEditorStore.getState().loadSchema({
        ...fireSchema,
        connections: [
          { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0, pipeline: [] },
        ],
      });
      useEditorStore.getState().addPipelineStep(1, { Filter: { Bool: true } });
      const conn = useEditorStore.getState().schema.connections.find((c) => c.id === 1)!;
      expect(conn.pipeline).toHaveLength(1);
      expect("Filter" in conn.pipeline[0]).toBe(true);
    });

    it("adds a Redirect step to a connection", () => {
      useEditorStore.getState().loadSchema({
        ...fireSchema,
        connections: [
          { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0, pipeline: [] },
        ],
      });
      useEditorStore.getState().addPipelineStep(1, { Redirect: 5 });
      const conn = useEditorStore.getState().schema.connections.find((c) => c.id === 1)!;
      expect(conn.pipeline).toHaveLength(1);
      expect("Redirect" in conn.pipeline[0]).toBe(true);
    });
  });

  describe("removePipelineStep", () => {
    it("removes a pipeline step by index", () => {
      useEditorStore.getState().loadSchema({
        ...fireSchema,
        connections: [
          { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0, pipeline: [{ Filter: { Bool: true } }, { Redirect: 3 }] },
        ],
      });
      useEditorStore.getState().removePipelineStep(1, 0);
      const conn = useEditorStore.getState().schema.connections.find((c) => c.id === 1)!;
      expect(conn.pipeline).toHaveLength(1);
      expect("Redirect" in conn.pipeline[0]).toBe(true);
    });
  });

  describe("updateConnectionDelay", () => {
    it("updates delay_ticks on a connection", () => {
      useEditorStore.getState().loadSchema({
        ...fireSchema,
        connections: [
          { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0, pipeline: [] },
        ],
      });
      useEditorStore.getState().updateConnectionDelay(1, 5);
      const conn = useEditorStore.getState().schema.connections.find((c) => c.id === 1)!;
      expect(conn.delay_ticks).toBe(5);
      expect(useEditorStore.getState().dirty).toBe(true);
    });
  });

  // --- Interaction Rule CRUD ---
  describe("addInteractionRule", () => {
    it("adds an interaction rule to the schema", () => {
      useEditorStore.getState().addInteractionRule();
      const rules = useEditorStore.getState().schema.interaction_rules;
      expect(rules).toHaveLength(1);
      expect(rules[0].id).toBe(1);
      expect(rules[0].participants).toEqual([]);
      expect(rules[0].conditions).toEqual([]);
      expect(rules[0].effects).toEqual([]);
      expect(useEditorStore.getState().dirty).toBe(true);
    });

    it("assigns incrementing IDs", () => {
      useEditorStore.getState().addInteractionRule();
      useEditorStore.getState().addInteractionRule();
      const rules = useEditorStore.getState().schema.interaction_rules;
      expect(rules[0].id).not.toBe(rules[1].id);
    });
  });

  describe("removeInteractionRule", () => {
    it("removes an interaction rule by id", () => {
      useEditorStore.getState().addInteractionRule();
      const id = useEditorStore.getState().schema.interaction_rules[0].id;
      useEditorStore.getState().removeInteractionRule(id);
      expect(useEditorStore.getState().schema.interaction_rules).toHaveLength(0);
    });
  });

  describe("updateInteractionRule", () => {
    it("updates participants", () => {
      useEditorStore.getState().addInteractionRule();
      const id = useEditorStore.getState().schema.interaction_rules[0].id;
      useEditorStore.getState().updateInteractionRule(id, {
        participants: [{ sm_id: 1, required_state: 0 }],
      });
      const rule = useEditorStore.getState().schema.interaction_rules.find((r) => r.id === id)!;
      expect(rule.participants).toHaveLength(1);
      expect(rule.participants[0].sm_id).toBe(1);
    });

    it("updates conditions", () => {
      useEditorStore.getState().addInteractionRule();
      const id = useEditorStore.getState().schema.interaction_rules[0].id;
      useEditorStore.getState().updateInteractionRule(id, {
        conditions: [{ kind: "Spatial", radius: 10 }],
      });
      const rule = useEditorStore.getState().schema.interaction_rules.find((r) => r.id === id)!;
      expect(rule.conditions).toHaveLength(1);
    });

    it("updates effects", () => {
      useEditorStore.getState().addInteractionRule();
      const id = useEditorStore.getState().schema.interaction_rules[0].id;
      useEditorStore.getState().updateInteractionRule(id, {
        effects: [{ Signal: { port: 0, payload: {} } }],
      });
      const rule = useEditorStore.getState().schema.interaction_rules.find((r) => r.id === id)!;
      expect(rule.effects).toHaveLength(1);
    });
  });

  describe("selectedInteractionRuleId", () => {
    it("selectInteractionRule sets the selected IR id", () => {
      useEditorStore.getState().addInteractionRule();
      const id = useEditorStore.getState().schema.interaction_rules[0].id;
      useEditorStore.getState().selectInteractionRule(id);
      expect(useEditorStore.getState().selectedInteractionRuleId).toBe(id);
    });

    it("clearSelection clears IR selection too", () => {
      useEditorStore.getState().addInteractionRule();
      const id = useEditorStore.getState().schema.interaction_rules[0].id;
      useEditorStore.getState().selectInteractionRule(id);
      useEditorStore.getState().clearSelection();
      expect(useEditorStore.getState().selectedInteractionRuleId).toBeNull();
    });
  });

  // --- updateTransition ---
  describe("updateTransition", () => {
    it("updates transition priority", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().updateTransition(1, 10, { priority: 99 });
      const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
      expect(sm.transitions[0].priority).toBe(99);
      expect(useEditorStore.getState().dirty).toBe(true);
    });

    it("updates transition guard", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().updateTransition(1, 10, { guard: { Bool: true } });
      const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
      expect(sm.transitions[0].guard).toEqual({ Bool: true });
    });

    it("updates transition effects", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().updateTransition(1, 10, {
        effects: [{ HitStop: { frames: 5 } }],
      });
      const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
      expect(sm.transitions[0].effects).toHaveLength(1);
    });

    it("does not affect other transitions", () => {
      useEditorStore.getState().loadSchema(fireSchema);
      useEditorStore.getState().addTransition(1, { id: 11, source: 1, target: 0, priority: 5, effects: [] });
      useEditorStore.getState().updateTransition(1, 10, { priority: 99 });
      const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
      expect(sm.transitions[1].priority).toBe(5);
    });
  });

  // --- updatePipelineStep ---
  describe("updatePipelineStep", () => {
    it("replaces a pipeline step at index", () => {
      useEditorStore.getState().loadSchema({
        ...fireSchema,
        connections: [
          { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0, pipeline: [{ Filter: { Bool: true } }] },
        ],
      });
      useEditorStore.getState().updatePipelineStep(1, 0, { Filter: { Bool: false } });
      const conn = useEditorStore.getState().schema.connections.find((c) => c.id === 1)!;
      expect(conn.pipeline[0]).toEqual({ Filter: { Bool: false } });
      expect(useEditorStore.getState().dirty).toBe(true);
    });
  });

  // --- Named Table CRUD ---
  describe("addNamedTable", () => {
    it("adds a named table with empty entries", () => {
      useEditorStore.getState().addNamedTable("damage_types");
      const tables = useEditorStore.getState().schema.named_tables;
      expect(tables).toHaveLength(1);
      expect(tables[0].name).toBe("damage_types");
      expect(tables[0].entries).toEqual([]);
      expect(useEditorStore.getState().dirty).toBe(true);
    });
  });

  describe("removeNamedTable", () => {
    it("removes a named table by name", () => {
      useEditorStore.getState().addNamedTable("test");
      useEditorStore.getState().removeNamedTable("test");
      expect(useEditorStore.getState().schema.named_tables).toHaveLength(0);
    });
  });

  describe("updateNamedTable", () => {
    it("updates entries for a named table", () => {
      useEditorStore.getState().addNamedTable("elements");
      useEditorStore.getState().updateNamedTable("elements", [
        { fire: 2, water: 0.5 },
      ]);
      const table = useEditorStore.getState().schema.named_tables.find((t) => t.name === "elements")!;
      expect(table.entries).toEqual([{ fire: 2, water: 0.5 }]);
      expect(useEditorStore.getState().dirty).toBe(true);
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
