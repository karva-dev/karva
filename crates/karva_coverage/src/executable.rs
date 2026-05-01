//! Compute the set of executable line numbers for a Python source file.
//!
//! "Executable" here means: the lines coverage.py would put in the `Stmts`
//! column. Built on ruff's [`SourceOrderVisitor`] so we get every node kind
//! (statement, decorator, `elif`/`else` clause, `except` handler, `match`
//! case) for free, and a single hook (`visit_body`) for skipping the
//! leading docstring of every body — `CPython` stores docstrings as
//! bytecode constants rather than executable statements.

use std::collections::HashSet;
use std::path::Path;

use ruff_python_ast::helpers::is_docstring_stmt;
use ruff_python_ast::visitor::source_order::{
    SourceOrderVisitor, walk_decorator, walk_elif_else_clause, walk_except_handler,
    walk_match_case, walk_stmt,
};
use ruff_python_ast::{Decorator, ElifElseClause, ExceptHandler, MatchCase, Stmt};
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use ruff_source_file::LineIndex;
use ruff_text_size::{Ranged, TextSize};

/// Parse `path` and return the set of line numbers that contain a statement.
pub fn executable_lines(path: &Path) -> HashSet<u32> {
    let Ok(source) = std::fs::read_to_string(path) else {
        return HashSet::new();
    };
    executable_lines_for_source(&source)
}

/// Compute executable line numbers from a source string. Exposed separately
/// so unit tests can avoid touching the filesystem.
pub fn executable_lines_for_source(source: &str) -> HashSet<u32> {
    let Some(parsed) = parse_unchecked(source, ParseOptions::from(Mode::Module)).try_into_module()
    else {
        return HashSet::new();
    };
    let line_index = LineIndex::from_source_text(source);
    let module = parsed.into_syntax();
    let mut visitor = ExecutableLineVisitor {
        line_index: &line_index,
        lines: HashSet::new(),
    };
    visitor.visit_body(&module.body);
    visitor.lines
}

struct ExecutableLineVisitor<'a> {
    line_index: &'a LineIndex,
    lines: HashSet<u32>,
}

impl ExecutableLineVisitor<'_> {
    fn record(&mut self, offset: TextSize) {
        if let Ok(line) = u32::try_from(self.line_index.line_index(offset).get()) {
            self.lines.insert(line);
        }
    }
}

impl<'a> SourceOrderVisitor<'a> for ExecutableLineVisitor<'_> {
    /// Skip the leading docstring (if any) before walking the rest of a body.
    fn visit_body(&mut self, body: &'a [Stmt]) {
        let start = usize::from(body.first().is_some_and(is_docstring_stmt));
        for stmt in &body[start..] {
            self.visit_stmt(stmt);
        }
    }

    /// Record each statement's start line. For function and class
    /// definitions the start of `Stmt::FunctionDef` / `Stmt::ClassDef`
    /// includes any decorators (the range begins at the first `@`); we use
    /// the name's range instead so the reported line is the `def` / `class`
    /// keyword line, matching coverage.py. The decorators themselves are
    /// recorded separately via `visit_decorator`.
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let offset = match stmt {
            Stmt::FunctionDef(s) => s.name.range().start(),
            Stmt::ClassDef(s) => s.name.range().start(),
            _ => stmt.range().start(),
        };
        self.record(offset);
        walk_stmt(self, stmt);
    }

    fn visit_decorator(&mut self, decorator: &'a Decorator) {
        self.record(decorator.range().start());
        walk_decorator(self, decorator);
    }

    /// `elif <expr>:` evaluates a test expression and emits its own
    /// bytecode, so it counts as an executable line. A bare `else:` has no
    /// bytecode of its own — coverage.py and `CPython`'s `co_lines()` skip
    /// it — so we skip it here too.
    fn visit_elif_else_clause(&mut self, clause: &'a ElifElseClause) {
        if clause.test.is_some() {
            self.record(clause.range().start());
        }
        walk_elif_else_clause(self, clause);
    }

    fn visit_except_handler(&mut self, handler: &'a ExceptHandler) {
        self.record(handler.range().start());
        walk_except_handler(self, handler);
    }

    fn visit_match_case(&mut self, case: &'a MatchCase) {
        self.record(case.range().start());
        walk_match_case(self, case);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(source: &str) -> Vec<u32> {
        let mut v: Vec<u32> = executable_lines_for_source(source).into_iter().collect();
        v.sort_unstable();
        v
    }

    /// Render `source` with each line prefixed by `[recorded]` if the
    /// visitor counted it as executable, or whitespace if not. Makes
    /// snapshots line up next to the original Python so a reviewer can
    /// scan and confirm the markers fall on the right keywords.
    fn annotate(source: &str) -> String {
        let recorded = executable_lines_for_source(source);
        source
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let lineno = u32::try_from(i).unwrap_or(u32::MAX).saturating_add(1);
                let marker = if recorded.contains(&lineno) {
                    "[recorded]"
                } else {
                    "          "
                };
                format!("{marker} {lineno:>3} | {line}")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn skips_module_docstring() {
        let src = "\
\"\"\"module doc\"\"\"

x = 1
";
        assert_eq!(lines(src), vec![3]);
    }

    #[test]
    fn skips_function_docstring() {
        let src = "\
def f():
    \"\"\"doc\"\"\"
    return 1
";
        assert_eq!(lines(src), vec![1, 3]);
    }

    #[test]
    fn walks_nested_class_methods() {
        let src = "\
class C:
    def m(self):
        return 1
";
        assert_eq!(lines(src), vec![1, 2, 3]);
    }

    #[test]
    fn walks_if_else_bodies() {
        let src = "\
if True:
    a = 1
else:
    b = 2
";
        // `else:` is not a separate executable line — it has no bytecode.
        assert_eq!(lines(src), vec![1, 2, 4]);
    }

    #[test]
    fn records_elif_but_not_else() {
        let src = "\
if a:
    x = 1
elif b:
    x = 2
else:
    x = 3
";
        // `elif b:` evaluates `b` (own bytecode) — recorded. Bare `else:`
        // has no bytecode and is not recorded.
        assert_eq!(lines(src), vec![1, 2, 3, 4, 6]);
    }

    #[test]
    fn walks_try_except_finally() {
        let src = "\
try:
    a = 1
except ValueError:
    b = 2
finally:
    c = 3
";
        // `except ValueError:` line is now recorded; `finally:` is not
        // (no AST node for the `finally` keyword in ruff).
        assert_eq!(lines(src), vec![1, 2, 3, 4, 6]);
    }

    #[test]
    fn records_each_decorator() {
        let src = "\
def deco(f):
    return f

@deco
@deco
def target():
    return 1
";
        assert_eq!(lines(src), vec![1, 2, 4, 5, 6, 7]);
    }

    #[test]
    fn records_match_case_headers() {
        let src = "\
match x:
    case 1:
        a = 1
    case _:
        a = 2
";
        assert_eq!(lines(src), vec![1, 2, 3, 4, 5]);
    }

    /// Annotated snapshot covering every kind of statement and clause.
    /// Reviewing the snapshot is a direct way to spot lines we record
    /// that we shouldn't, or lines we miss that we should.
    #[test]
    fn every_kind_of_statement() {
        let src = "\
\"\"\"module docstring\"\"\"

import os
from sys import argv as a
import os as o, sys as s

x = 1
x: int = 2
x += 3
del x

def deco(f):
    return f

@deco
@deco
def decorated():
    \"\"\"docstring\"\"\"
    return 1

async def coro():
    await coro2()
async def coro2():
    pass

class C:
    \"\"\"class docstring\"\"\"
    attr = 1
    def method(self):
        return self.attr

if True:
    a = 1
elif False:
    b = 2
else:
    c = 3

while False:
    pass
else:
    pass

for i in range(1):
    pass
else:
    pass

async def with_async_for():
    async for i in agen():
        pass

with open('x') as f:
    pass

async def with_async_with():
    async with cm() as c:
        pass

try:
    raise ValueError
except ValueError:
    pass
except (KeyError, TypeError):
    pass
else:
    pass
finally:
    pass

try:
    raise
except* ValueError:
    pass

match x:
    case 1:
        pass
    case _:
        pass

global g
nonlocal n
assert True
raise RuntimeError
return None
yield 1
pass
break
continue
type Alias = int
";
        insta::assert_snapshot!(annotate(src), @r#"
                     1 | """module docstring"""
                     2 | 
        [recorded]   3 | import os
        [recorded]   4 | from sys import argv as a
        [recorded]   5 | import os as o, sys as s
                     6 | 
        [recorded]   7 | x = 1
        [recorded]   8 | x: int = 2
        [recorded]   9 | x += 3
        [recorded]  10 | del x
                    11 | 
        [recorded]  12 | def deco(f):
        [recorded]  13 |     return f
                    14 | 
        [recorded]  15 | @deco
        [recorded]  16 | @deco
        [recorded]  17 | def decorated():
                    18 |     """docstring"""
        [recorded]  19 |     return 1
                    20 | 
        [recorded]  21 | async def coro():
        [recorded]  22 |     await coro2()
        [recorded]  23 | async def coro2():
        [recorded]  24 |     pass
                    25 | 
        [recorded]  26 | class C:
                    27 |     """class docstring"""
        [recorded]  28 |     attr = 1
        [recorded]  29 |     def method(self):
        [recorded]  30 |         return self.attr
                    31 | 
        [recorded]  32 | if True:
        [recorded]  33 |     a = 1
        [recorded]  34 | elif False:
        [recorded]  35 |     b = 2
                    36 | else:
        [recorded]  37 |     c = 3
                    38 | 
        [recorded]  39 | while False:
        [recorded]  40 |     pass
                    41 | else:
        [recorded]  42 |     pass
                    43 | 
        [recorded]  44 | for i in range(1):
        [recorded]  45 |     pass
                    46 | else:
        [recorded]  47 |     pass
                    48 | 
        [recorded]  49 | async def with_async_for():
        [recorded]  50 |     async for i in agen():
        [recorded]  51 |         pass
                    52 | 
        [recorded]  53 | with open('x') as f:
        [recorded]  54 |     pass
                    55 | 
        [recorded]  56 | async def with_async_with():
        [recorded]  57 |     async with cm() as c:
        [recorded]  58 |         pass
                    59 | 
        [recorded]  60 | try:
        [recorded]  61 |     raise ValueError
        [recorded]  62 | except ValueError:
        [recorded]  63 |     pass
        [recorded]  64 | except (KeyError, TypeError):
        [recorded]  65 |     pass
                    66 | else:
        [recorded]  67 |     pass
                    68 | finally:
        [recorded]  69 |     pass
                    70 | 
        [recorded]  71 | try:
        [recorded]  72 |     raise
        [recorded]  73 | except* ValueError:
        [recorded]  74 |     pass
                    75 | 
        [recorded]  76 | match x:
        [recorded]  77 |     case 1:
        [recorded]  78 |         pass
        [recorded]  79 |     case _:
        [recorded]  80 |         pass
                    81 | 
        [recorded]  82 | global g
        [recorded]  83 | nonlocal n
        [recorded]  84 | assert True
        [recorded]  85 | raise RuntimeError
        [recorded]  86 | return None
        [recorded]  87 | yield 1
        [recorded]  88 | pass
        [recorded]  89 | break
        [recorded]  90 | continue
        [recorded]  91 | type Alias = int
        "#);
    }
}
