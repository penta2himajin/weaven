import { describe, it, expect } from "vitest";
import { parseSchema, serializeSchema, validateSchema } from "../schemaIo";
import type { WeavenSchema } from "../generated/schema";

const validJson = JSON.stringify({
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
});

describe("schemaIo", () => {
  describe("parseSchema", () => {
    it("parses valid JSON into WeavenSchema", () => {
      const result = parseSchema(validJson);
      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value.state_machines).toHaveLength(1);
        expect(result.value.state_machines[0].id).toBe(1);
      }
    });

    it("returns error for invalid JSON", () => {
      const result = parseSchema("{bad json}");
      expect(result.ok).toBe(false);
    });

    it("fills defaults for missing optional fields", () => {
      const minimal = JSON.stringify({
        state_machines: [{ id: 1, states: [0], initial_state: 0 }],
      });
      const result = parseSchema(minimal);
      expect(result.ok).toBe(true);
      if (result.ok) {
        expect(result.value.connections).toEqual([]);
        expect(result.value.named_tables).toEqual([]);
        expect(result.value.state_machines[0].transitions).toEqual([]);
      }
    });
  });

  describe("serializeSchema", () => {
    it("round-trips through parse → serialize → parse", () => {
      const first = parseSchema(validJson);
      expect(first.ok).toBe(true);
      if (!first.ok) return;

      const json = serializeSchema(first.value);
      const second = parseSchema(json);
      expect(second.ok).toBe(true);
      if (second.ok) {
        expect(second.value).toEqual(first.value);
      }
    });
  });

  describe("validateSchema", () => {
    it("returns no errors for valid schema", () => {
      const schema: WeavenSchema = JSON.parse(validJson);
      const errors = validateSchema(schema);
      expect(errors).toHaveLength(0);
    });

    it("detects duplicate SM IDs", () => {
      const schema: WeavenSchema = {
        state_machines: [
          { id: 1, states: [0], initial_state: 0, transitions: [], input_ports: [], output_ports: [] },
          { id: 1, states: [0], initial_state: 0, transitions: [], input_ports: [], output_ports: [] },
        ],
        connections: [],
        named_tables: [],
      };
      const errors = validateSchema(schema);
      expect(errors.some((e) => e.includes("Duplicate SM id"))).toBe(true);
    });

    it("detects initial_state not in states array", () => {
      const schema: WeavenSchema = {
        state_machines: [
          { id: 1, states: [0], initial_state: 5, transitions: [], input_ports: [], output_ports: [] },
        ],
        connections: [],
        named_tables: [],
      };
      const errors = validateSchema(schema);
      expect(errors.some((e) => e.includes("initial_state"))).toBe(true);
    });

    it("detects connection referencing non-existent SM", () => {
      const schema: WeavenSchema = {
        state_machines: [
          { id: 1, states: [0], initial_state: 0, transitions: [], input_ports: [], output_ports: [{ id: 1, kind: "Output", signal_type: 0 }] },
        ],
        connections: [
          { id: 1, source_sm: 1, source_port: 1, target_sm: 99, target_port: 0, delay_ticks: 0, pipeline: [] },
        ],
        named_tables: [],
      };
      const errors = validateSchema(schema);
      expect(errors.some((e) => e.includes("target_sm"))).toBe(true);
    });

    it("detects transition referencing state not in SM", () => {
      const schema: WeavenSchema = {
        state_machines: [
          {
            id: 1, states: [0, 1], initial_state: 0,
            transitions: [{ id: 10, source: 0, target: 99, priority: 10, effects: [] }],
            input_ports: [], output_ports: [],
          },
        ],
        connections: [],
        named_tables: [],
      };
      const errors = validateSchema(schema);
      expect(errors.some((e) => e.includes("target 99"))).toBe(true);
    });
  });
});
