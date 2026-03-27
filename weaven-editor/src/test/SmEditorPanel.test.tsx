import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import SmEditorPanel from "../components/SmEditorPanel";
import { useEditorStore } from "../stores/editorStore";
import type { WeavenSchema } from "../generated/schema";

const schema: WeavenSchema = {
  state_machines: [
    {
      id: 1, states: [0, 1], initial_state: 0,
      transitions: [{ id: 10, source: 0, target: 1, priority: 10, effects: [] }],
      input_ports: [{ id: 0, kind: "Input", signal_type: 0 }],
      output_ports: [{ id: 1, kind: "Output", signal_type: 0 }],
    },
  ],
  connections: [],
  named_tables: [], interaction_rules: [],
};

const schemaWithGuard: WeavenSchema = {
  state_machines: [
    {
      id: 1, states: [0, 1], initial_state: 0,
      transitions: [{
        id: 10, source: 0, target: 1, priority: 10,
        guard: { BinOp: { op: "Gt", left: { CtxField: "hp" }, right: { Num: 0 } } },
        effects: [{ HitStop: { frames: 5 } }],
      }],
      input_ports: [],
      output_ports: [],
    },
  ],
  connections: [],
  named_tables: [], interaction_rules: [],
};

function resetStore() {
  useEditorStore.setState({
    schema: { state_machines: [], connections: [], named_tables: [], interaction_rules: [] },
    selectedSmId: null,
    selectedConnectionId: null, selectedInteractionRuleId: null,
    dirty: false,
  });
}

describe("SmEditorPanel", () => {
  beforeEach(resetStore);

  it("shows prompt when no SM selected", () => {
    render(<SmEditorPanel />);
    expect(screen.getByText(/select a state machine/i)).toBeInTheDocument();
  });

  it("shows SM details when selected", () => {
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);
    expect(screen.getByText("SM(1)")).toBeInTheDocument();
    expect(screen.getByText(/State 0/)).toBeInTheDocument();
    expect(screen.getByText(/State 1/)).toBeInTheDocument();
  });

  it("lists transitions", () => {
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);
    expect(screen.getByText(/T\(10\)/)).toBeInTheDocument();
    expect(screen.getByText(/0 → 1/)).toBeInTheDocument();
  });

  it("lists ports with kind", () => {
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);
    expect(screen.getByText(/Input:0/)).toBeInTheDocument();
    expect(screen.getByText(/Output:1/)).toBeInTheDocument();
  });

  it("add state button creates a new state", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.click(screen.getByRole("button", { name: /add state/i }));
    const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
    expect(sm.states).toHaveLength(3);
  });

  it("delete SM button removes the SM", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.click(screen.getByRole("button", { name: /delete sm/i }));
    expect(useEditorStore.getState().schema.state_machines).toHaveLength(0);
    expect(useEditorStore.getState().selectedSmId).toBeNull();
  });

  // --- Transition CRUD ---
  it("Add Transition button creates a new transition", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.type(screen.getByLabelText("transition source"), "1");
    await user.type(screen.getByLabelText("transition target"), "0");
    await user.click(screen.getByRole("button", { name: /add transition/i }));

    const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
    expect(sm.transitions).toHaveLength(2);
    expect(sm.transitions[1].source).toBe(1);
    expect(sm.transitions[1].target).toBe(0);
  });

  it("Remove Transition button removes a transition", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.click(screen.getByLabelText("remove transition 10"));
    const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
    expect(sm.transitions).toHaveLength(0);
  });

  // --- Transition detail editing ---
  it("clicking a transition shows detail editor", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.click(screen.getByText(/T\(10\)/));
    expect(screen.getByText(/Transition T\(10\) Detail/)).toBeInTheDocument();
    expect(screen.getByLabelText("edit priority")).toHaveValue(10);
  });

  it("can edit transition priority", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.click(screen.getByText(/T\(10\)/));
    const input = screen.getByLabelText("edit priority");
    await user.clear(input);
    await user.type(input, "20");

    const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
    expect(sm.transitions[0].priority).toBe(20);
  });

  it("can add a guard to a transition", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.click(screen.getByText(/T\(10\)/));
    await user.click(screen.getByText("Add Guard"));

    const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
    expect(sm.transitions[0].guard).toEqual({ Bool: true });
  });

  it("can remove a guard from a transition", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schemaWithGuard);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.click(screen.getByText(/T\(10\)/));
    await user.click(screen.getByLabelText("remove guard"));

    const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
    expect(sm.transitions[0].guard).toBeNull();
  });

  it("shows guard expression builder when guard exists", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schemaWithGuard);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.click(screen.getByText(/T\(10\)/));
    // ExpressionBuilder renders with expression type selectors (guard + nested sub-expressions)
    const selectors = screen.getAllByLabelText("expression type");
    expect(selectors.length).toBeGreaterThanOrEqual(1);
  });

  it("shows [G] indicator for transitions with guards", () => {
    useEditorStore.getState().loadSchema(schemaWithGuard);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);
    expect(screen.getByText("[G]")).toBeInTheDocument();
  });

  it("shows effect count indicator for transitions with effects", () => {
    useEditorStore.getState().loadSchema(schemaWithGuard);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);
    expect(screen.getByText("[1E]")).toBeInTheDocument();
  });

  it("can add an effect to a transition", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.click(screen.getByText(/T\(10\)/));
    await user.click(screen.getByText("Add Effect"));

    const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
    expect(sm.transitions[0].effects).toHaveLength(1);
  });

  it("can remove an effect from a transition", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schemaWithGuard);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.click(screen.getByText(/T\(10\)/));
    await user.click(screen.getByLabelText("remove effect"));

    const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
    expect(sm.transitions[0].effects).toHaveLength(0);
  });

  // --- Port kind selector ---
  it("shows port kind selector", () => {
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);
    expect(screen.getByLabelText("port kind")).toBeInTheDocument();
  });

  it("can add a port with specific kind", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectSm(1);
    render(<SmEditorPanel />);

    await user.selectOptions(screen.getByLabelText("port kind"), "ContinuousInput");
    await user.click(screen.getByRole("button", { name: /add port/i }));

    const sm = useEditorStore.getState().schema.state_machines.find((s) => s.id === 1)!;
    expect(sm.input_ports).toHaveLength(2);
    expect(sm.input_ports[1].kind).toBe("ContinuousInput");
  });
});
