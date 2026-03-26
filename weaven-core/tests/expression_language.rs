/// Expression Language (§5) and Named Table (§2.8) tests.

use weaven_core::{
    Expr, BinOpKind, EvalCtx, TableRegistry, NamedTableData, TableValue,
    eval, eval_bool, eval_guard, Context,
};
use std::collections::BTreeMap;

fn empty_tables() -> TableRegistry { TableRegistry::new() }
fn empty_ctx() -> Context { Context::default() }

fn ctx_with(pairs: &[(&str, f64)]) -> Context {
    let mut c = Context::default();
    for (k, v) in pairs { c.set(*k, *v); }
    c
}

fn sig_with(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

fn ec<'a>(ctx: &'a Context, sig: Option<&'a BTreeMap<String, f64>>, tables: &'a TableRegistry) -> EvalCtx<'a> {
    EvalCtx { context: ctx, signal: sig, received_ports: &[], tables }
}

// ── Literals ────────────────────────────────────────────────────────────────

#[test]
fn test_num_literal() {
    let c = empty_ctx(); let t = empty_tables();
    assert_eq!(eval(&Expr::Num(42.0), &ec(&c, None, &t)), 42.0);
    assert_eq!(eval(&Expr::Num(-1.5), &ec(&c, None, &t)), -1.5);
}

#[test]
fn test_bool_literal() {
    let c = empty_ctx(); let t = empty_tables();
    assert_eq!(eval(&Expr::Bool(true),  &ec(&c, None, &t)), 1.0);
    assert_eq!(eval(&Expr::Bool(false), &ec(&c, None, &t)), 0.0);
}

// ── Field access ─────────────────────────────────────────────────────────────

#[test]
fn test_ctx_field() {
    let c = ctx_with(&[("hp", 80.0)]);
    let t = empty_tables();
    assert_eq!(eval(&Expr::CtxField("hp".into()),      &ec(&c, None, &t)), 80.0);
    assert_eq!(eval(&Expr::CtxField("missing".into()), &ec(&c, None, &t)), 0.0);
}

#[test]
fn test_sig_field() {
    let c = empty_ctx(); let t = empty_tables();
    let s = sig_with(&[("intensity", 5.0)]);
    assert_eq!(eval(&Expr::SigField("intensity".into()), &ec(&c, Some(&s), &t)), 5.0);
    assert_eq!(eval(&Expr::SigField("missing".into()),   &ec(&c, Some(&s), &t)), 0.0);
}

// ── Arithmetic ───────────────────────────────────────────────────────────────

#[test]
fn test_arithmetic() {
    let c = empty_ctx(); let t = empty_tables();
    let e = &ec(&c, None, &t);
    let num = |n: f64| Expr::Num(n);

    let add = Expr::BinOp { op: BinOpKind::Add, left: Box::new(num(3.0)), right: Box::new(num(4.0)) };
    assert_eq!(eval(&add, e), 7.0);

    let sub = Expr::BinOp { op: BinOpKind::Sub, left: Box::new(num(10.0)), right: Box::new(num(3.0)) };
    assert_eq!(eval(&sub, e), 7.0);

    let mul = Expr::BinOp { op: BinOpKind::Mul, left: Box::new(num(3.0)), right: Box::new(num(4.0)) };
    assert_eq!(eval(&mul, e), 12.0);

    let div_zero = Expr::BinOp { op: BinOpKind::Div, left: Box::new(num(5.0)), right: Box::new(num(0.0)) };
    assert_eq!(eval(&div_zero, e), 0.0); // div-by-zero → 0

    let modulo = Expr::BinOp { op: BinOpKind::Mod, left: Box::new(num(7.0)), right: Box::new(num(3.0)) };
    assert_eq!(eval(&modulo, e), 1.0);
}

// ── Comparison ───────────────────────────────────────────────────────────────

#[test]
fn test_comparison() {
    let c = ctx_with(&[("hp", 50.0)]); let t = empty_tables();
    let e = &ec(&c, None, &t);
    let hp = || Expr::CtxField("hp".into());

    let lt = Expr::BinOp { op: BinOpKind::Lt, left: Box::new(hp()), right: Box::new(Expr::Num(100.0)) };
    assert!(eval_bool(&lt, e));

    let eq = Expr::BinOp { op: BinOpKind::Eq, left: Box::new(hp()), right: Box::new(Expr::Num(50.0)) };
    assert!(eval_bool(&eq, e));

    let gte = Expr::BinOp { op: BinOpKind::Gte, left: Box::new(hp()), right: Box::new(Expr::Num(50.0)) };
    assert!(eval_bool(&gte, e));

    let neq = Expr::BinOp { op: BinOpKind::Neq, left: Box::new(hp()), right: Box::new(Expr::Num(50.0)) };
    assert!(!eval_bool(&neq, e));
}

// ── Logical ──────────────────────────────────────────────────────────────────

#[test]
fn test_logical() {
    let c = ctx_with(&[("a", 1.0), ("b", 0.0)]); let t = empty_tables();
    let e = &ec(&c, None, &t);
    let a = || Expr::CtxField("a".into());
    let b = || Expr::CtxField("b".into());

    let and = Expr::BinOp { op: BinOpKind::And, left: Box::new(a()), right: Box::new(b()) };
    assert!(!eval_bool(&and, e));

    let or = Expr::BinOp { op: BinOpKind::Or, left: Box::new(a()), right: Box::new(b()) };
    assert!(eval_bool(&or, e));

    let not_b = Expr::Not(Box::new(b()));
    assert!(eval_bool(&not_b, e));
}

// ── If / then / else ─────────────────────────────────────────────────────────

#[test]
fn test_if_then_else() {
    let c = ctx_with(&[("hp", 30.0)]); let t = empty_tables();
    let e = &ec(&c, None, &t);

    // if hp < 50 then 2.0 else 1.0
    let expr = Expr::If {
        cond:  Box::new(Expr::BinOp { op: BinOpKind::Lt,
            left: Box::new(Expr::CtxField("hp".into())), right: Box::new(Expr::Num(50.0)) }),
        then_: Box::new(Expr::Num(2.0)),
        else_: Box::new(Expr::Num(1.0)),
    };
    assert_eq!(eval(&expr, e), 2.0); // hp=30 < 50 → then

    let c2 = ctx_with(&[("hp", 80.0)]);
    let e2 = &ec(&c2, None, &t);
    assert_eq!(eval(&expr, e2), 1.0); // hp=80 not < 50 → else
}

// ── Named Table: single-key ───────────────────────────────────────────────────

#[test]
fn test_table_single_key() {
    let mut tables = TableRegistry::new();
    let mut t = NamedTableData::new();
    t.insert("Fire",   TableValue::Num(1.5));
    t.insert("Water",  TableValue::Num(0.5));
    tables.register("elementDamage", t);

    let c = empty_ctx();
    let e = &ec(&c, None, &tables);

    let lookup = |key: &str| Expr::TableLookup {
        table: "elementDamage".into(),
        keys: vec![Box::new(Expr::Str(key.into()))],
    };
    assert_eq!(eval(&lookup("Fire"),  e), 1.5);
    assert_eq!(eval(&lookup("Water"), e), 0.5);
    assert_eq!(eval(&lookup("Ghost"), e), 0.0); // missing → 0
}

// ── Named Table: nested two-key (elemental reaction chart) ────────────────────

#[test]
fn test_table_nested_lookup() {
    let mut tables = TableRegistry::new();
    let mut reaction = NamedTableData::new();

    let mut fire = NamedTableData::new();
    fire.insert("Grass",  TableValue::Num(2.0));
    fire.insert("Water",  TableValue::Num(0.5));
    fire.insert("Fire",   TableValue::Num(1.0));
    reaction.insert("Fire", TableValue::Table(fire));

    let mut water = NamedTableData::new();
    water.insert("Fire",  TableValue::Num(2.0));
    water.insert("Grass", TableValue::Num(0.5));
    water.insert("Water", TableValue::Num(1.0));
    reaction.insert("Water", TableValue::Table(water));

    tables.register("reactionTable", reaction);
    let c = empty_ctx();
    let e = &ec(&c, None, &tables);

    let lookup = |a: &str, b: &str| Expr::TableLookup {
        table: "reactionTable".into(),
        keys: vec![Box::new(Expr::Str(a.into())), Box::new(Expr::Str(b.into()))],
    };
    assert_eq!(eval(&lookup("Fire",  "Grass"), e), 2.0);
    assert_eq!(eval(&lookup("Fire",  "Water"), e), 0.5);
    assert_eq!(eval(&lookup("Water", "Fire"),  e), 2.0);
}

// ── Left-to-right evaluation order (§5.3) ────────────────────────────────────

#[test]
fn test_left_to_right_order() {
    let c = ctx_with(&[("a", 10.0), ("b", 3.0)]);
    let t = empty_tables();
    let e = &ec(&c, None, &t);
    let a = || Expr::CtxField("a".into());
    let b = || Expr::CtxField("b".into());

    let a_minus_b = Expr::BinOp { op: BinOpKind::Sub, left: Box::new(a()), right: Box::new(b()) };
    assert_eq!(eval(&a_minus_b, e),  7.0);  // a(10) - b(3)

    let b_minus_a = Expr::BinOp { op: BinOpKind::Sub, left: Box::new(b()), right: Box::new(a()) };
    assert_eq!(eval(&b_minus_a, e), -7.0); // b(3) - a(10)
}

// ── eval_guard: table-driven damage threshold ─────────────────────────────────

#[test]
fn test_eval_guard_table_driven() {
    // Guard: signal.baseDamage * table.elementDamage["Fire"] > context.threshold
    let mut tables = TableRegistry::new();
    let mut t = NamedTableData::new();
    t.insert("Fire",  TableValue::Num(2.0));
    t.insert("Water", TableValue::Num(0.5));
    tables.register("elementDamage", t);

    let ctx = ctx_with(&[("threshold", 5.0)]);
    let sig = sig_with(&[("baseDamage", 3.0)]);

    let guard = |element: &str| Expr::BinOp {
        op: BinOpKind::Gt,
        left: Box::new(Expr::BinOp {
            op: BinOpKind::Mul,
            left:  Box::new(Expr::SigField("baseDamage".into())),
            right: Box::new(Expr::TableLookup {
                table: "elementDamage".into(),
                keys: vec![Box::new(Expr::Str(element.into()))],
            }),
        }),
        right: Box::new(Expr::CtxField("threshold".into())),
    };

    // Fire: 3 * 2.0 = 6 > 5 → true
    assert!(eval_guard(&guard("Fire"), &ctx, Some(&sig), &tables));
    // Water: 3 * 0.5 = 1.5 < 5 → false
    assert!(!eval_guard(&guard("Water"), &ctx, Some(&sig), &tables));
}

// ── World::register_table ────────────────────────────────────────────────────

#[test]
fn test_world_register_table() {
    use weaven_core::World;
    let mut world = World::new();
    let mut t = NamedTableData::new();
    t.insert("sword", TableValue::Num(50.0));
    world.register_table("weaponDamage", t);

    assert_eq!(
        world.tables.lookup("weaponDamage", &["sword"]),
        Some(TableValue::Num(50.0))
    );
}
