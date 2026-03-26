/// Collection operations tests (§5.1): any, count, sum.
///
/// Collections are array fields in SM Context.
/// Predicates receive each element's fields as SigField.

use weaven_core::{Expr, BinOpKind, EvalCtx, TableRegistry, Context,
                  eval, eval_bool};
use std::collections::BTreeMap;

fn empty_tables() -> TableRegistry { TableRegistry::new() }

fn elem(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
    pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
}

fn ctx_with_array(field: &str, elements: Vec<BTreeMap<String, f64>>) -> Context {
    let mut c = Context::default();
    c.set_array(field, elements);
    c
}

fn ec<'a>(ctx: &'a Context, tables: &'a TableRegistry) -> EvalCtx<'a> {
    EvalCtx { context: ctx, signal: None, received_ports: &[], tables }
}

// ── CollectionAny ─────────────────────────────────────────────────────────

/// any: returns true if at least one element satisfies the predicate.
#[test]
fn test_collection_any_true() {
    // Context has "enemies" array with hp values
    // Predicate: elem.hp <= 0 (any dead enemy)
    let ctx = ctx_with_array("enemies", vec![
        elem(&[("hp", 50.0)]),
        elem(&[("hp", 0.0)]),   // dead
        elem(&[("hp", 30.0)]),
    ]);
    let t = empty_tables();

    let expr = Expr::CollectionAny {
        array_field: "enemies".into(),
        predicate: Box::new(Expr::BinOp {
            op: BinOpKind::Lte,
            left:  Box::new(Expr::SigField("hp".into())),
            right: Box::new(Expr::Num(0.0)),
        }),
    };
    assert!(eval_bool(&expr, &ec(&ctx, &t)), "at least one enemy has hp<=0");
}

#[test]
fn test_collection_any_false() {
    let ctx = ctx_with_array("enemies", vec![
        elem(&[("hp", 50.0)]),
        elem(&[("hp", 20.0)]),
    ]);
    let t = empty_tables();

    let expr = Expr::CollectionAny {
        array_field: "enemies".into(),
        predicate: Box::new(Expr::BinOp {
            op: BinOpKind::Lte,
            left:  Box::new(Expr::SigField("hp".into())),
            right: Box::new(Expr::Num(0.0)),
        }),
    };
    assert!(!eval_bool(&expr, &ec(&ctx, &t)), "no enemy dead");
}

#[test]
fn test_collection_any_empty_array() {
    let ctx = ctx_with_array("enemies", vec![]);
    let t = empty_tables();
    let expr = Expr::CollectionAny {
        array_field: "enemies".into(),
        predicate: Box::new(Expr::Bool(true)),
    };
    assert!(!eval_bool(&expr, &ec(&ctx, &t)), "empty collection → false");
}

// ── CollectionCount ───────────────────────────────────────────────────────

/// count: returns number of elements satisfying the predicate.
#[test]
fn test_collection_count() {
    let ctx = ctx_with_array("buffs", vec![
        elem(&[("active", 1.0), ("power", 10.0)]),
        elem(&[("active", 0.0), ("power", 5.0)]),
        elem(&[("active", 1.0), ("power", 8.0)]),
        elem(&[("active", 1.0), ("power", 3.0)]),
    ]);
    let t = empty_tables();

    let expr = Expr::CollectionCount {
        array_field: "buffs".into(),
        predicate: Box::new(Expr::BinOp {
            op: BinOpKind::Gt,
            left:  Box::new(Expr::SigField("active".into())),
            right: Box::new(Expr::Num(0.0)),
        }),
    };
    assert_eq!(eval(&expr, &ec(&ctx, &t)), 3.0, "3 active buffs");
}

#[test]
fn test_collection_count_zero() {
    let ctx = ctx_with_array("buffs", vec![
        elem(&[("active", 0.0)]),
        elem(&[("active", 0.0)]),
    ]);
    let t = empty_tables();
    let expr = Expr::CollectionCount {
        array_field: "buffs".into(),
        predicate: Box::new(Expr::SigField("active".into())),
    };
    assert_eq!(eval(&expr, &ec(&ctx, &t)), 0.0);
}

// ── CollectionSum ─────────────────────────────────────────────────────────

/// sum: returns sum of a named field across all elements.
#[test]
fn test_collection_sum() {
    let ctx = ctx_with_array("projectiles", vec![
        elem(&[("damage", 10.0)]),
        elem(&[("damage", 15.0)]),
        elem(&[("damage", 5.0)]),
    ]);
    let t = empty_tables();

    let expr = Expr::CollectionSum {
        array_field: "projectiles".into(),
        sum_field: "damage".into(),
    };
    assert_eq!(eval(&expr, &ec(&ctx, &t)), 30.0, "10+15+5=30");
}

#[test]
fn test_collection_sum_empty() {
    let ctx = ctx_with_array("projectiles", vec![]);
    let t = empty_tables();
    let expr = Expr::CollectionSum {
        array_field: "projectiles".into(),
        sum_field: "damage".into(),
    };
    assert_eq!(eval(&expr, &ec(&ctx, &t)), 0.0, "empty → 0");
}

#[test]
fn test_collection_sum_missing_field() {
    // Element doesn't have the sum_field → treated as 0.0
    let ctx = ctx_with_array("items", vec![
        elem(&[("value", 5.0)]),
        elem(&[("other", 3.0)]), // no "damage" field
    ]);
    let t = empty_tables();
    let expr = Expr::CollectionSum {
        array_field: "items".into(),
        sum_field: "damage".into(),
    };
    assert_eq!(eval(&expr, &ec(&ctx, &t)), 0.0, "missing fields default to 0");
}

// ── Combined expression ───────────────────────────────────────────────────

/// Count + threshold guard: "if active status effect count > 2, trigger overload"
#[test]
fn test_collection_count_in_guard() {
    let ctx = ctx_with_array("status_effects", vec![
        elem(&[("active", 1.0)]),
        elem(&[("active", 1.0)]),
        elem(&[("active", 1.0)]),
        elem(&[("active", 0.0)]),
    ]);
    let t = empty_tables();

    // count(status_effects where active > 0) > 2
    let guard = Expr::BinOp {
        op: BinOpKind::Gt,
        left: Box::new(Expr::CollectionCount {
            array_field: "status_effects".into(),
            predicate: Box::new(Expr::BinOp {
                op: BinOpKind::Gt,
                left:  Box::new(Expr::SigField("active".into())),
                right: Box::new(Expr::Num(0.0)),
            }),
        }),
        right: Box::new(Expr::Num(2.0)),
    };
    assert!(eval_bool(&guard, &ec(&ctx, &t)), "3 active effects > 2 → overload");
}

/// Sum + threshold: "total damage from projectiles > player HP"
#[test]
fn test_collection_sum_vs_context_field() {
    let mut ctx = ctx_with_array("incoming", vec![
        elem(&[("dmg", 20.0)]),
        elem(&[("dmg", 15.0)]),
    ]);
    ctx.set("hp", 30.0);
    let t = empty_tables();

    // sum(incoming.dmg) > context.hp → 35 > 30 → lethal
    let guard = Expr::BinOp {
        op: BinOpKind::Gt,
        left: Box::new(Expr::CollectionSum {
            array_field: "incoming".into(),
            sum_field: "dmg".into(),
        }),
        right: Box::new(Expr::CtxField("hp".into())),
    };
    assert!(eval_bool(&guard, &ec(&ctx, &t)), "35 > 30 → lethal");
}

// ── Context::set_array / get_array ───────────────────────────────────────

#[test]
fn test_context_array_field_operations() {
    let mut ctx = Context::default();
    ctx.set_array("items", vec![
        elem(&[("v", 1.0)]),
        elem(&[("v", 2.0)]),
    ]);
    assert_eq!(ctx.get_array("items").len(), 2);
    assert_eq!(ctx.get_array("missing").len(), 0); // absent → empty

    // Scalar fields still work independently
    ctx.set("x", 5.0);
    assert_eq!(ctx.get("x"), 5.0);
}
