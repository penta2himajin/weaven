/// Expression Language parser tests (§5, §11.1).
///
/// Validates `weaven_core::parse()` — the hand-written recursive descent
/// parser that converts source text into the `Expr` AST.

use weaven_core::{
    parse, ParseError,
    Expr, BinOpKind,
    EvalCtx, TableRegistry, Context,
    eval, eval_bool,
};
use std::collections::BTreeMap;

fn empty_ctx() -> Context { Context::default() }
fn empty_tables() -> TableRegistry { TableRegistry::new() }
fn ec<'a>(ctx: &'a Context, tables: &'a TableRegistry) -> EvalCtx<'a> {
    EvalCtx { context: ctx, signal: None, received_ports: &[], tables }
}

// ── Literals ────────────────────────────────────────────────────────────────

#[test]
fn test_parse_number_literal() {
    let e = parse("42.0").unwrap();
    assert_eq!(eval(&e, &ec(&empty_ctx(), &empty_tables())), 42.0);
}

#[test]
fn test_parse_negative_number() {
    let e = parse("-3.5").unwrap();
    assert_eq!(eval(&e, &ec(&empty_ctx(), &empty_tables())), -3.5);
}

#[test]
fn test_parse_integer_literal() {
    let e = parse("10").unwrap();
    assert_eq!(eval(&e, &ec(&empty_ctx(), &empty_tables())), 10.0);
}

#[test]
fn test_parse_bool_true() {
    let e = parse("true").unwrap();
    assert!(eval_bool(&e, &ec(&empty_ctx(), &empty_tables())));
}

#[test]
fn test_parse_bool_false() {
    let e = parse("false").unwrap();
    assert!(!eval_bool(&e, &ec(&empty_ctx(), &empty_tables())));
}

// ── Context / Signal field references ─────────────────────────────────────

#[test]
fn test_parse_context_field() {
    let mut ctx = empty_ctx();
    ctx.set("hp", 80.0);
    let e = parse("context.hp").unwrap();
    assert_eq!(eval(&e, &ec(&ctx, &empty_tables())), 80.0);
}

#[test]
fn test_parse_signal_field() {
    let mut ctx = empty_ctx();
    ctx.set("_", 0.0); // ensure ctx is non-empty
    let sig: BTreeMap<String, f64> = [("damage".to_string(), 25.0)].into_iter().collect();
    let t = empty_tables();
    let e = parse("signal.damage").unwrap();
    let ec = EvalCtx { context: &ctx, signal: Some(&sig), received_ports: &[], tables: &t };
    assert_eq!(eval(&e, &ec), 25.0);
}

#[test]
fn test_parse_port_received() {
    let ctx = empty_ctx();
    let t = empty_tables();
    let e = parse("port.hit_in.received").unwrap();
    let ec_with = EvalCtx {
        context: &ctx, signal: None,
        received_ports: &["hit_in".to_string()],
        tables: &t,
    };
    let ec_without = EvalCtx {
        context: &ctx, signal: None,
        received_ports: &[],
        tables: &t,
    };
    assert!(eval_bool(&e, &ec_with));
    assert!(!eval_bool(&e, &ec_without));
}

// ── Arithmetic ─────────────────────────────────────────────────────────────

#[test]
fn test_parse_addition() {
    let e = parse("2.0 + 3.0").unwrap();
    assert_eq!(eval(&e, &ec(&empty_ctx(), &empty_tables())), 5.0);
}

#[test]
fn test_parse_subtraction() {
    let e = parse("10.0 - 4.0").unwrap();
    assert_eq!(eval(&e, &ec(&empty_ctx(), &empty_tables())), 6.0);
}

#[test]
fn test_parse_multiplication() {
    let e = parse("3.0 * 4.0").unwrap();
    assert_eq!(eval(&e, &ec(&empty_ctx(), &empty_tables())), 12.0);
}

#[test]
fn test_parse_division() {
    let e = parse("10.0 / 4.0").unwrap();
    assert_eq!(eval(&e, &ec(&empty_ctx(), &empty_tables())), 2.5);
}

#[test]
fn test_parse_modulo() {
    let e = parse("7.0 % 3.0").unwrap();
    assert_eq!(eval(&e, &ec(&empty_ctx(), &empty_tables())), 1.0);
}

#[test]
fn test_parse_operator_precedence_mul_over_add() {
    // 2 + 3 * 4 = 2 + 12 = 14  (not 20)
    let e = parse("2.0 + 3.0 * 4.0").unwrap();
    assert_eq!(eval(&e, &ec(&empty_ctx(), &empty_tables())), 14.0);
}

#[test]
fn test_parse_parentheses_override_precedence() {
    let e = parse("(2.0 + 3.0) * 4.0").unwrap();
    assert_eq!(eval(&e, &ec(&empty_ctx(), &empty_tables())), 20.0);
}

// ── Comparisons ────────────────────────────────────────────────────────────

#[test]
fn test_parse_eq() {
    assert!(eval_bool(&parse("1.0 == 1.0").unwrap(), &ec(&empty_ctx(), &empty_tables())));
    assert!(!eval_bool(&parse("1.0 == 2.0").unwrap(), &ec(&empty_ctx(), &empty_tables())));
}

#[test]
fn test_parse_neq() {
    assert!(eval_bool(&parse("1.0 != 2.0").unwrap(), &ec(&empty_ctx(), &empty_tables())));
}

#[test]
fn test_parse_lt_lte_gt_gte() {
    let t = empty_tables(); let c = empty_ctx();
    assert!(eval_bool(&parse("1.0 < 2.0").unwrap(),  &ec(&c, &t)));
    assert!(eval_bool(&parse("2.0 <= 2.0").unwrap(), &ec(&c, &t)));
    assert!(eval_bool(&parse("3.0 > 2.0").unwrap(),  &ec(&c, &t)));
    assert!(eval_bool(&parse("2.0 >= 2.0").unwrap(), &ec(&c, &t)));
}

// ── Logical ────────────────────────────────────────────────────────────────

#[test]
fn test_parse_and() {
    let t = empty_tables(); let c = empty_ctx();
    assert!(eval_bool(&parse("true AND true").unwrap(),   &ec(&c, &t)));
    assert!(!eval_bool(&parse("true AND false").unwrap(), &ec(&c, &t)));
}

#[test]
fn test_parse_or() {
    let t = empty_tables(); let c = empty_ctx();
    assert!(eval_bool(&parse("false OR true").unwrap(),    &ec(&c, &t)));
    assert!(!eval_bool(&parse("false OR false").unwrap(),  &ec(&c, &t)));
}

#[test]
fn test_parse_not() {
    let t = empty_tables(); let c = empty_ctx();
    assert!(eval_bool(&parse("NOT false").unwrap(), &ec(&c, &t)));
    assert!(!eval_bool(&parse("NOT true").unwrap(), &ec(&c, &t)));
}

// ── If expression ──────────────────────────────────────────────────────────

#[test]
fn test_parse_if_then_else() {
    let t = empty_tables(); let c = empty_ctx();
    let e = parse("if true then 1.0 else 2.0").unwrap();
    assert_eq!(eval(&e, &ec(&c, &t)), 1.0);
    let e2 = parse("if false then 1.0 else 2.0").unwrap();
    assert_eq!(eval(&e2, &ec(&c, &t)), 2.0);
}

#[test]
fn test_parse_nested_if() {
    let t = empty_tables(); let c = empty_ctx();
    let e = parse("if true then if false then 99.0 else 2.0 else 3.0").unwrap();
    assert_eq!(eval(&e, &ec(&c, &t)), 2.0);
}

// ── Table lookup ──────────────────────────────────────────────────────────

#[test]
fn test_parse_table_lookup_single_key() {
    use weaven_core::{NamedTableData, TableValue};
    let mut data = BTreeMap::new();
    data.insert("Fire".to_string(), TableValue::Num(2.0));
    let mut reg = TableRegistry::new();
    reg.register("effectTable", NamedTableData(data));

    let e = parse(r#"table.effectTable["Fire"]"#).unwrap();
    let c = empty_ctx();
    let ec = EvalCtx { context: &c, signal: None, received_ports: &[], tables: &reg };
    assert_eq!(eval(&e, &ec), 2.0);
}

// ── Complex guard expression ───────────────────────────────────────────────

#[test]
fn test_parse_complex_guard() {
    // Mirrors a real guard: "context.hp <= 0.0 AND NOT port.shield_in.received"
    let mut ctx = empty_ctx();
    ctx.set("hp", 0.0);
    let t = empty_tables();
    let e = parse("context.hp <= 0.0 AND NOT port.shield_in.received").unwrap();
    let ec_no_port = EvalCtx { context: &ctx, signal: None, received_ports: &[], tables: &t };
    assert!(eval_bool(&e, &ec_no_port), "hp=0, no shield → guard passes");

    let ec_with_port = EvalCtx {
        context: &ctx, signal: None,
        received_ports: &["shield_in".to_string()],
        tables: &t,
    };
    assert!(!eval_bool(&e, &ec_with_port), "hp=0 but shield received → guard fails");
}

// ── Error cases ────────────────────────────────────────────────────────────

#[test]
fn test_parse_empty_string_errors() {
    assert!(matches!(parse(""), Err(ParseError { .. })));
}

#[test]
fn test_parse_unmatched_paren_errors() {
    assert!(parse("(1.0 + 2.0").is_err());
}

#[test]
fn test_parse_unknown_token_errors() {
    assert!(parse("@invalid").is_err());
}

#[test]
fn test_parse_incomplete_if_errors() {
    assert!(parse("if true then 1.0").is_err()); // missing else
}
