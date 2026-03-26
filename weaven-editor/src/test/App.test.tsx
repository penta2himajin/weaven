import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "../App";
import { useEditorStore } from "../stores/editorStore";

function resetStore() {
  useEditorStore.setState({
    schema: { state_machines: [], connections: [], named_tables: [] },
    selectedSmId: null,
    selectedConnectionId: null,
    dirty: false,
  });
}

describe("App", () => {
  beforeEach(resetStore);

  it("renders the app title", () => {
    render(<App />);
    expect(screen.getByText(/weaven editor/i)).toBeInTheDocument();
  });

  it("shows Add SM button in toolbar", () => {
    render(<App />);
    expect(screen.getByRole("button", { name: /add sm/i })).toBeInTheDocument();
  });

  it("Add SM button creates a state machine", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: /add sm/i }));
    expect(useEditorStore.getState().schema.state_machines).toHaveLength(1);
  });

  it("shows Export JSON button", () => {
    render(<App />);
    expect(screen.getByRole("button", { name: /export/i })).toBeInTheDocument();
  });

  it("shows Import JSON button", () => {
    render(<App />);
    expect(screen.getByRole("button", { name: /import/i })).toBeInTheDocument();
  });

  it("shows validation errors panel", () => {
    render(<App />);
    // Initially no errors
    expect(screen.getByText(/no validation errors/i)).toBeInTheDocument();
  });
});
