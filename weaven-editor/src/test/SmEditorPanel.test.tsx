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
  named_tables: [],
};

function resetStore() {
  useEditorStore.setState({
    schema: { state_machines: [], connections: [], named_tables: [] },
    selectedSmId: null,
    selectedConnectionId: null,
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

  it("lists ports", () => {
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
});
