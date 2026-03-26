import { describe, it, expect } from "vitest";
import { schemaToNodes, schemaToEdges } from "../components/topologyHelpers";
import type { WeavenSchema } from "../generated/schema";

const schema: WeavenSchema = {
  state_machines: [
    {
      id: 1, states: [0, 1], initial_state: 0,
      transitions: [{ id: 10, source: 0, target: 1, priority: 10, effects: [] }],
      input_ports: [{ id: 0, kind: "Input", signal_type: 0 }],
      output_ports: [{ id: 1, kind: "Output", signal_type: 0 }],
    },
    {
      id: 2, states: [0, 1], initial_state: 0,
      transitions: [],
      input_ports: [{ id: 0, kind: "Input", signal_type: 0 }],
      output_ports: [],
    },
  ],
  connections: [
    { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0, pipeline: [] },
  ],
  named_tables: [], interaction_rules: [],
};

describe("topologyHelpers", () => {
  describe("schemaToNodes", () => {
    it("creates one node per SM", () => {
      const nodes = schemaToNodes(schema);
      expect(nodes).toHaveLength(2);
    });

    it("uses sm-{id} as node id", () => {
      const nodes = schemaToNodes(schema);
      expect(nodes[0].id).toBe("sm-1");
      expect(nodes[1].id).toBe("sm-2");
    });

    it("includes SM metadata in data", () => {
      const nodes = schemaToNodes(schema);
      expect(nodes[0].data.smId).toBe(1);
      expect(nodes[0].data.label).toBe("SM(1)");
      expect(nodes[0].data.states).toEqual([0, 1]);
      expect(nodes[0].data.inputPorts).toHaveLength(1);
      expect(nodes[0].data.outputPorts).toHaveLength(1);
    });

    it("assigns positions using dagre layout", () => {
      const nodes = schemaToNodes(schema);
      for (const node of nodes) {
        expect(typeof node.position.x).toBe("number");
        expect(typeof node.position.y).toBe("number");
      }
    });
  });

  describe("schemaToEdges", () => {
    it("creates one edge per connection", () => {
      const edges = schemaToEdges(schema);
      expect(edges).toHaveLength(1);
    });

    it("uses conn-{id} as edge id", () => {
      const edges = schemaToEdges(schema);
      expect(edges[0].id).toBe("conn-1");
    });

    it("maps source/target to sm-{id} node ids", () => {
      const edges = schemaToEdges(schema);
      expect(edges[0].source).toBe("sm-1");
      expect(edges[0].target).toBe("sm-2");
    });

    it("includes connection metadata in data", () => {
      const edges = schemaToEdges(schema);
      expect(edges[0].data?.connectionId).toBe(1);
    });
  });
});
