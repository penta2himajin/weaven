import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import ExpressionBuilder from "../components/ExpressionBuilder";
import type { ExprSchema } from "../generated/schema";

describe("ExpressionBuilder", () => {
  it("renders a Num expression with input", () => {
    const onChange = vi.fn();
    render(<ExpressionBuilder expr={{ Num: 42 }} onChange={onChange} />);
    const input = screen.getByLabelText("number value") as HTMLInputElement;
    expect(input.value).toBe("42");
  });

  it("calls onChange when Num value changes", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<ExpressionBuilder expr={{ Num: 0 }} onChange={onChange} />);

    const input = screen.getByLabelText("number value");
    await user.clear(input);
    await user.type(input, "5");
    expect(onChange).toHaveBeenCalledWith({ Num: 5 });
  });

  it("renders a Bool expression with select", () => {
    const onChange = vi.fn();
    render(<ExpressionBuilder expr={{ Bool: true }} onChange={onChange} />);
    const select = screen.getByLabelText("boolean value") as HTMLSelectElement;
    expect(select.value).toBe("true");
  });

  it("calls onChange when Bool value changes", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<ExpressionBuilder expr={{ Bool: true }} onChange={onChange} />);

    await user.selectOptions(screen.getByLabelText("boolean value"), "false");
    expect(onChange).toHaveBeenCalledWith({ Bool: false });
  });

  it("renders a Str expression with text input", () => {
    const onChange = vi.fn();
    render(<ExpressionBuilder expr={{ Str: "hello" }} onChange={onChange} />);
    const input = screen.getByLabelText("string value") as HTMLInputElement;
    expect(input.value).toBe("hello");
  });

  it("renders a CtxField expression", () => {
    const onChange = vi.fn();
    render(<ExpressionBuilder expr={{ CtxField: "hp" }} onChange={onChange} />);
    const input = screen.getByLabelText("context field") as HTMLInputElement;
    expect(input.value).toBe("hp");
  });

  it("renders a SigField expression", () => {
    const onChange = vi.fn();
    render(<ExpressionBuilder expr={{ SigField: "damage" }} onChange={onChange} />);
    const input = screen.getByLabelText("signal field") as HTMLInputElement;
    expect(input.value).toBe("damage");
  });

  it("renders a BinOp expression with operator and sub-expressions", () => {
    const onChange = vi.fn();
    const expr: ExprSchema = {
      BinOp: {
        op: "Add",
        left: { Num: 1 },
        right: { Num: 2 },
      },
    };
    render(<ExpressionBuilder expr={expr} onChange={onChange} />);

    expect(screen.getByLabelText("operator")).toBeInTheDocument();
    expect(screen.getByText("Left:")).toBeInTheDocument();
    expect(screen.getByText("Right:")).toBeInTheDocument();
  });

  it("changes operator in BinOp", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    const expr: ExprSchema = {
      BinOp: { op: "Add", left: { Num: 1 }, right: { Num: 2 } },
    };
    render(<ExpressionBuilder expr={expr} onChange={onChange} />);

    await user.selectOptions(screen.getByLabelText("operator"), "Mul");
    expect(onChange).toHaveBeenCalledWith({
      BinOp: { op: "Mul", left: { Num: 1 }, right: { Num: 2 } },
    });
  });

  it("renders a Not expression with sub-expression", () => {
    const onChange = vi.fn();
    render(<ExpressionBuilder expr={{ Not: { Bool: true } }} onChange={onChange} />);
    // Should render nested expression builder
    const nodes = screen.getAllByTestId(/expr-node/);
    expect(nodes.length).toBeGreaterThanOrEqual(2);
  });

  it("renders an If expression with three sub-expressions", () => {
    const onChange = vi.fn();
    const expr: ExprSchema = {
      If: {
        cond: { Bool: true },
        then_: { Num: 1 },
        else_: { Num: 0 },
      },
    };
    render(<ExpressionBuilder expr={expr} onChange={onChange} />);
    expect(screen.getByText("Condition:")).toBeInTheDocument();
    expect(screen.getByText("Then:")).toBeInTheDocument();
    expect(screen.getByText("Else:")).toBeInTheDocument();
  });

  it("switching expression type resets to default", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<ExpressionBuilder expr={{ Num: 42 }} onChange={onChange} />);

    await user.selectOptions(screen.getByLabelText("expression type"), "Bool");
    expect(onChange).toHaveBeenCalledWith({ Bool: true });
  });

  it("switching to CtxField creates CtxField expression", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<ExpressionBuilder expr={{ Num: 0 }} onChange={onChange} />);

    await user.selectOptions(screen.getByLabelText("expression type"), "CtxField");
    expect(onChange).toHaveBeenCalledWith({ CtxField: "" });
  });

  it("switching to BinOp creates BinOp expression with defaults", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<ExpressionBuilder expr={{ Num: 0 }} onChange={onChange} />);

    await user.selectOptions(screen.getByLabelText("expression type"), "BinOp");
    expect(onChange).toHaveBeenCalledWith({
      BinOp: { op: "Add", left: { Num: 0 }, right: { Num: 0 } },
    });
  });

  it("switching to If creates If expression with defaults", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<ExpressionBuilder expr={{ Num: 0 }} onChange={onChange} />);

    await user.selectOptions(screen.getByLabelText("expression type"), "If");
    expect(onChange).toHaveBeenCalledWith({
      If: { cond: { Bool: true }, then_: { Num: 0 }, else_: { Num: 0 } },
    });
  });

  it("renders CollectionAny with array_field input", () => {
    const onChange = vi.fn();
    render(
      <ExpressionBuilder
        expr={{ CollectionAny: { array_field: "items", predicate: { Bool: true } } }}
        onChange={onChange}
      />,
    );
    const input = screen.getByLabelText("array field") as HTMLInputElement;
    expect(input.value).toBe("items");
    expect(screen.getByText("Predicate:")).toBeInTheDocument();
  });

  it("renders CollectionSum with array_field and sum_field inputs", () => {
    const onChange = vi.fn();
    render(
      <ExpressionBuilder
        expr={{ CollectionSum: { array_field: "items", sum_field: "price" } }}
        onChange={onChange}
      />,
    );
    const arrayInput = screen.getByLabelText("array field") as HTMLInputElement;
    const sumInput = screen.getByLabelText("sum field") as HTMLInputElement;
    expect(arrayInput.value).toBe("items");
    expect(sumInput.value).toBe("price");
  });

  it("renders TableLookup with table input", () => {
    const onChange = vi.fn();
    render(
      <ExpressionBuilder
        expr={{ TableLookup: { table: "damage_table", keys: [] } }}
        onChange={onChange}
      />,
    );
    const input = screen.getByLabelText("table name") as HTMLInputElement;
    expect(input.value).toBe("damage_table");
  });

  it("respects depth limit and does not render infinitely", () => {
    const onChange = vi.fn();
    const expr: ExprSchema = {
      BinOp: {
        op: "Add",
        left: { BinOp: { op: "Sub", left: { Num: 1 }, right: { Num: 2 } } },
        right: { Num: 3 },
      },
    };
    render(<ExpressionBuilder expr={expr} onChange={onChange} />);
    const nodes = screen.getAllByTestId(/expr-node/);
    expect(nodes.length).toBeGreaterThanOrEqual(3);
  });
});
