import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import ConnectionEditorPanel from "../components/ConnectionEditorPanel";
import { useEditorStore } from "../stores/editorStore";
import type { WeavenSchema } from "../generated/schema";

const schema: WeavenSchema = {
  state_machines: [
    { id: 1, states: [0], initial_state: 0, transitions: [], input_ports: [], output_ports: [{ id: 1, kind: "Output", signal_type: 0 }] },
    { id: 2, states: [0], initial_state: 0, transitions: [], input_ports: [{ id: 0, kind: "Input", signal_type: 0 }], output_ports: [] },
  ],
  connections: [
    { id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 2, pipeline: [] },
  ],
  named_tables: [],
  interaction_rules: [],
};

const schemaWithPipeline: WeavenSchema = {
  ...schema,
  connections: [
    {
      id: 1, source_sm: 1, source_port: 1, target_sm: 2, target_port: 0, delay_ticks: 0,
      pipeline: [{ Transform: { value: { Num: 1 } } }, { Filter: { Bool: true } }],
    },
  ],
};

function resetStore() {
  useEditorStore.setState({
    schema: { state_machines: [], connections: [], named_tables: [], interaction_rules: [] },
    selectedSmId: null,
    selectedConnectionId: null,
    selectedInteractionRuleId: null,
    dirty: false,
  });
}

describe("ConnectionEditorPanel", () => {
  beforeEach(resetStore);

  it("shows prompt when no connection selected", () => {
    render(<ConnectionEditorPanel />);
    expect(screen.getByText(/select a connection/i)).toBeInTheDocument();
  });

  it("shows connection details when selected", () => {
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);
    expect(screen.getByText("Connection(1)")).toBeInTheDocument();
    expect(screen.getByText(/SM\(1\) → SM\(2\)/)).toBeInTheDocument();
  });

  it("shows delay_ticks input", () => {
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);
    const input = screen.getByLabelText("delay ticks") as HTMLInputElement;
    expect(input.value).toBe("2");
  });

  it("updating delay_ticks changes the store", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);

    const input = screen.getByLabelText("delay ticks");
    await user.clear(input);
    await user.type(input, "5");
    expect(useEditorStore.getState().schema.connections[0].delay_ticks).toBe(5);
  });

  it("delete button removes the connection", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);

    await user.click(screen.getByRole("button", { name: /delete connection/i }));
    expect(useEditorStore.getState().schema.connections).toHaveLength(0);
  });

  // Pipeline editing tests
  it("shows empty pipeline message", () => {
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);
    expect(screen.getByText(/no pipeline steps/i)).toBeInTheDocument();
  });

  it("shows step type selector and Add Step button", () => {
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);
    expect(screen.getByLabelText("step type")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /add step/i })).toBeInTheDocument();
  });

  it("Add Step adds a Transform step by default", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);

    await user.click(screen.getByRole("button", { name: /add step/i }));
    const conn = useEditorStore.getState().schema.connections[0];
    expect(conn.pipeline).toHaveLength(1);
    expect("Transform" in conn.pipeline[0]).toBe(true);
  });

  it("can add a Filter step", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);

    await user.selectOptions(screen.getByLabelText("step type"), "Filter");
    await user.click(screen.getByRole("button", { name: /add step/i }));
    const conn = useEditorStore.getState().schema.connections[0];
    expect("Filter" in conn.pipeline[0]).toBe(true);
  });

  it("can add a Redirect step", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schema);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);

    await user.selectOptions(screen.getByLabelText("step type"), "Redirect");
    await user.click(screen.getByRole("button", { name: /add step/i }));
    const conn = useEditorStore.getState().schema.connections[0];
    expect("Redirect" in conn.pipeline[0]).toBe(true);
  });

  it("displays existing pipeline steps with remove buttons", () => {
    useEditorStore.getState().loadSchema(schemaWithPipeline);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);
    const removeButtons = screen.getAllByText("Remove");
    expect(removeButtons).toHaveLength(2);
  });

  it("Remove button removes a pipeline step", async () => {
    const user = userEvent.setup();
    useEditorStore.getState().loadSchema(schemaWithPipeline);
    useEditorStore.getState().selectConnection(1);
    render(<ConnectionEditorPanel />);

    const removeButtons = screen.getAllByText("Remove");
    await user.click(removeButtons[0]);
    const conn = useEditorStore.getState().schema.connections[0];
    expect(conn.pipeline).toHaveLength(1);
    expect("Filter" in conn.pipeline[0]).toBe(true);
  });
});
