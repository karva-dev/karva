use ruff_python_ast::{Expr, StmtFunctionDef};

/// Statically count the number of test cases a function will expand to once
/// its `@parametrize` decorators are applied.
///
/// Returns `Some(n)` for `n >= 1` when every parametrize-shaped decorator can
/// be counted from the AST alone (`argvalues` is a literal list or tuple).
/// Returns `None` when at least one parametrize decorator has a non-literal
/// `argvalues`; callers should treat the function as a single opaque unit.
///
/// A function with no parametrize decorators returns `Some(1)`.
pub fn count_parametrize_cases(stmt: &StmtFunctionDef) -> Option<usize> {
    let mut total: usize = 1;

    for decorator in &stmt.decorator_list {
        let Expr::Call(call) = &decorator.expression else {
            continue;
        };

        if !is_parametrize_call(call.func.as_ref()) {
            continue;
        }

        let argvalues = argvalues_arg(call)?;
        let count = literal_sequence_len(argvalues)?;

        total = total.checked_mul(count)?;
    }

    Some(total)
}

/// Returns true if `func` resolves to a parametrize reference.
///
/// Matches bare `parametrize`, attribute accesses ending in `parametrize`
/// (`pytest.mark.parametrize`, `karva.tags.parametrize`), and any chained
/// attribute on those.
fn is_parametrize_call(func: &Expr) -> bool {
    match func {
        Expr::Name(name) => name.id == "parametrize",
        Expr::Attribute(attr) => attr.attr.id == "parametrize",
        _ => false,
    }
}

/// Extract the `argvalues` argument from a parametrize call.
///
/// Accepts both `parametrize("x", [1, 2])` (positional) and
/// `parametrize(argnames="x", argvalues=[1, 2])` (keyword).
fn argvalues_arg(call: &ruff_python_ast::ExprCall) -> Option<&Expr> {
    if let Some(expr) = call.arguments.args.get(1) {
        return Some(expr);
    }
    call.arguments
        .keywords
        .iter()
        .find(|kw| kw.arg.as_ref().is_some_and(|id| id.as_str() == "argvalues"))
        .map(|kw| &kw.value)
}

/// Returns the element count of a list or tuple literal, or `None` if the
/// expression isn't a literal sequence we can count statically.
fn literal_sequence_len(expr: &Expr) -> Option<usize> {
    match expr {
        Expr::List(list) => Some(list.elts.len()),
        Expr::Tuple(tuple) => Some(tuple.elts.len()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::{Mod, Stmt};
    use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};

    use super::*;

    fn parse_function(source: &str) -> StmtFunctionDef {
        let parsed = parse_unchecked(source, ParseOptions::from(Mode::Module))
            .try_into_module()
            .expect("parse")
            .into_syntax();
        let Mod::Module(module) = Mod::Module(parsed) else {
            unreachable!()
        };
        module
            .body
            .into_iter()
            .find_map(|stmt| match stmt {
                Stmt::FunctionDef(f) => Some(f),
                _ => None,
            })
            .expect("function def")
    }

    #[test]
    fn no_decorators_returns_one() {
        let f = parse_function("def test_x(): pass\n");
        assert_eq!(count_parametrize_cases(&f), Some(1));
    }

    #[test]
    fn unrelated_decorator_returns_one() {
        let f = parse_function("@my_decorator\ndef test_x(): pass\n");
        assert_eq!(count_parametrize_cases(&f), Some(1));
    }

    #[test]
    fn pytest_mark_parametrize_list() {
        let f = parse_function("@pytest.mark.parametrize('x', [1, 2, 3])\ndef test_x(x): pass\n");
        assert_eq!(count_parametrize_cases(&f), Some(3));
    }

    #[test]
    fn karva_tags_parametrize_list_of_tuples() {
        let f = parse_function(
            "@karva.tags.parametrize('a, b', [(1, 2), (3, 4)])\ndef test_x(a, b): pass\n",
        );
        assert_eq!(count_parametrize_cases(&f), Some(2));
    }

    #[test]
    fn bare_parametrize_name() {
        let f = parse_function("@parametrize('x', [1, 2])\ndef test_x(x): pass\n");
        assert_eq!(count_parametrize_cases(&f), Some(2));
    }

    #[test]
    fn keyword_argvalues() {
        let f = parse_function(
            "@pytest.mark.parametrize(argnames='x', argvalues=[1, 2, 3, 4])\ndef test_x(x): pass\n",
        );
        assert_eq!(count_parametrize_cases(&f), Some(4));
    }

    #[test]
    fn stacked_decorators_multiply() {
        let f = parse_function(
            "@pytest.mark.parametrize('a', [1, 2, 3])\n\
             @pytest.mark.parametrize('b', [4, 5])\n\
             def test_x(a, b): pass\n",
        );
        assert_eq!(count_parametrize_cases(&f), Some(6));
    }

    #[test]
    fn tuple_argvalues_counts_outer_elements() {
        let f =
            parse_function("@parametrize('x', ((1, 2), (3, 4), (5, 6)))\ndef test_x(x): pass\n");
        assert_eq!(count_parametrize_cases(&f), Some(3));
    }

    #[test]
    fn dynamic_argvalues_returns_none() {
        let f = parse_function("@parametrize('x', load_cases())\ndef test_x(x): pass\n");
        assert_eq!(count_parametrize_cases(&f), None);
    }

    #[test]
    fn list_comprehension_argvalues_returns_none() {
        let f = parse_function("@parametrize('x', [v for v in values()])\ndef test_x(x): pass\n");
        assert_eq!(count_parametrize_cases(&f), None);
    }

    #[test]
    fn missing_argvalues_returns_none() {
        let f = parse_function("@parametrize('x')\ndef test_x(x): pass\n");
        assert_eq!(count_parametrize_cases(&f), None);
    }

    #[test]
    fn dynamic_decorator_propagates_none() {
        let f = parse_function(
            "@parametrize('a', [1, 2])\n\
             @parametrize('b', dynamic_values())\n\
             def test_x(a, b): pass\n",
        );
        assert_eq!(count_parametrize_cases(&f), None);
    }
}
