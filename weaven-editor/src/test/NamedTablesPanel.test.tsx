import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import NamedTablesPanel from "../components/NamedTablesPanel";
import { useEditorStore } from "../stores/editorStore";

function resetStore() {
  useEditorStore.setState({
    schema: { state_machines: [], connections: [], named_tables: [], interaction_rules: [] },
    selectedSmId: null,
    selectedConnectionId: null,
    selectedInteractionRuleId: null,
    dirty: false,
  });
}

describe("NamedTablesPanel", () => {
  beforeEach(resetStore);

  it("shows empty message when no tables exist", () => {
    render(<NamedTablesPanel />);
    expect(screen.getByText(/no named tables/i)).toBeInTheDocument();
  });

  it("shows Named Tables heading", () => {
    render(<NamedTablesPanel />);
    expect(screen.getByText("Named Tables")).toBeInTheDocument();
  });

  it("Add Table button creates a named table", async () => {
    const user = userEvent.setup();
    render(<NamedTablesPanel />);

    await user.type(screen.getByLabelText("new table name"), "damage_types");
    await user.click(screen.getByRole("button", { name: /add table/i }));

    expect(useEditorStore.getState().schema.named_tables).toHaveLength(1);
    expect(useEditorStore.getState().schema.named_tables[0].name).toBe("damage_types");
  });

  it("displays table names after adding", async () => {
    const user = userEvent.setup();
    render(<NamedTablesPanel />);

    await user.type(screen.getByLabelText("new table name"), "elements");
    await user.click(screen.getByRole("button", { name: /add table/i }));

    expect(screen.getByText("elements")).toBeInTheDocument();
  });

  it("prevents adding duplicate table names", async () => {
    useEditorStore.getState().addNamedTable("test");
    const user = userEvent.setup();
    render(<NamedTablesPanel />);

    await user.type(screen.getByLabelText("new table name"), "test");
    await user.click(screen.getByRole("button", { name: /add table/i }));

    expect(useEditorStore.getState().schema.named_tables).toHaveLength(1);
  });

  it("Remove button removes a named table", async () => {
    useEditorStore.getState().addNamedTable("test_table");
    const user = userEvent.setup();
    render(<NamedTablesPanel />);

    await user.click(screen.getByLabelText("remove table test_table"));
    expect(useEditorStore.getState().schema.named_tables).toHaveLength(0);
  });

  it("clicking a table shows the entry editor", async () => {
    useEditorStore.getState().addNamedTable("elements");
    const user = userEvent.setup();
    render(<NamedTablesPanel />);

    await user.click(screen.getByText("elements"));
    expect(screen.getByText("Table: elements")).toBeInTheDocument();
    expect(screen.getByLabelText("table entries json")).toBeInTheDocument();
  });

  it("Apply button saves JSON entries", async () => {
    useEditorStore.getState().addNamedTable("dmg");
    const user = userEvent.setup();
    render(<NamedTablesPanel />);

    await user.click(screen.getByText("dmg"));
    const textarea = screen.getByLabelText("table entries json") as HTMLTextAreaElement;
    // Use fireEvent.change to set value (brackets are special in userEvent)
    fireEvent.change(textarea, { target: { value: "[1,2,3]" } });
    await user.click(screen.getByRole("button", { name: /apply/i }));

    const table = useEditorStore.getState().schema.named_tables[0];
    expect(table.entries).toEqual([1, 2, 3]);
  });

  it("shows error for invalid JSON", async () => {
    useEditorStore.getState().addNamedTable("test");
    const user = userEvent.setup();
    render(<NamedTablesPanel />);

    await user.click(screen.getByText("test"));
    const textarea = screen.getByLabelText("table entries json") as HTMLTextAreaElement;
    await user.clear(textarea);
    await user.type(textarea, "not json");
    await user.click(screen.getByRole("button", { name: /apply/i }));

    expect(screen.getByText(/invalid json/i)).toBeInTheDocument();
  });
});
