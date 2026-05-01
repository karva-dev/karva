//! Compute the set of executable line numbers for a Python source file.
//!
//! "Executable" here means: the lines coverage.py would put in the `Stmts`
//! column. Each statement contributes its start line; the leading docstring
//! of every body is skipped, since `CPython` stores docstrings as bytecode
//! constants rather than executable statements.

use std::collections::HashSet;
use std::path::Path;

use ruff_python_ast::Stmt;
use ruff_python_ast::helpers::is_docstring_stmt;
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use ruff_source_file::LineIndex;
use ruff_text_size::Ranged;

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
    let mut lines = HashSet::new();
    let module = parsed.into_syntax();
    collect_body_lines(&module.body, &line_index, &mut lines);
    lines
}

/// Recursively walk a statement body, recording the line of every statement
/// that is not the body's leading docstring.
fn collect_body_lines(stmts: &[Stmt], line_index: &LineIndex, lines: &mut HashSet<u32>) {
    for (i, stmt) in stmts.iter().enumerate() {
        if i == 0 && is_docstring_stmt(stmt) {
            continue;
        }
        if let Ok(line) = u32::try_from(line_index.line_index(stmt.range().start()).get()) {
            lines.insert(line);
        }
        match stmt {
            Stmt::FunctionDef(s) => collect_body_lines(&s.body, line_index, lines),
            Stmt::ClassDef(s) => collect_body_lines(&s.body, line_index, lines),
            Stmt::If(s) => {
                collect_body_lines(&s.body, line_index, lines);
                for clause in &s.elif_else_clauses {
                    collect_body_lines(&clause.body, line_index, lines);
                }
            }
            Stmt::While(s) => {
                collect_body_lines(&s.body, line_index, lines);
                collect_body_lines(&s.orelse, line_index, lines);
            }
            Stmt::For(s) => {
                collect_body_lines(&s.body, line_index, lines);
                collect_body_lines(&s.orelse, line_index, lines);
            }
            Stmt::With(s) => collect_body_lines(&s.body, line_index, lines),
            Stmt::Try(s) => {
                collect_body_lines(&s.body, line_index, lines);
                for handler in &s.handlers {
                    let ruff_python_ast::ExceptHandler::ExceptHandler(h) = handler;
                    collect_body_lines(&h.body, line_index, lines);
                }
                collect_body_lines(&s.orelse, line_index, lines);
                collect_body_lines(&s.finalbody, line_index, lines);
            }
            Stmt::Match(s) => {
                for case in &s.cases {
                    collect_body_lines(&case.body, line_index, lines);
                }
            }
            _ => {}
        }
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
        assert_eq!(lines(src), vec![1, 2, 4]);
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
        assert_eq!(lines(src), vec![1, 2, 4, 6]);
    }

    /// Comprehensive snapshot of which lines we currently consider executable
    /// for every kind of statement and clause. The numbered comments mark the
    /// line numbers we expect (or notably do not) record. This test is the
    /// regression baseline: when we extend the walker to count decorator
    /// lines, `elif`/`else`/`except`/`finally`/`case` headers, etc., the
    /// expected set here is what must change.
    #[test]
    fn every_kind_of_statement_baseline() {
        let src = "\
\"\"\"module docstring (skipped, line 1)\"\"\"

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
    \"\"\"docstring (skipped)\"\"\"
    return 1

async def coro():
    await coro2()
async def coro2():
    pass

class C:
    \"\"\"class docstring (skipped)\"\"\"
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
        let actual = lines(src);

        // Lines we DO record today: every Stmt's start line plus nested
        // bodies, minus leading docstrings.
        let expected = vec![
            3,  // import os
            4,  // from sys import argv
            5,  // import os as o, sys as s
            7,  // x = 1
            8,  // x: int = 2
            9,  // x += 3
            10, // del x
            12, // def deco(f):
            13, //     return f
            15, // @deco (NOTE: today recorded as the FunctionDef start, which is at the @ line)
            19, // return 1 inside `decorated`
            21, // async def coro
            22, //     await coro2()
            23, // async def coro2
            24, //     pass
            26, // class C:
            28, //     attr = 1
            29, //     def method
            30, //         return self.attr
            32, // if True:
            33, //     a = 1
            // 34: elif (NOT recorded today)
            35, //     b = 2
            // 36: else (NOT recorded today)
            37, //     c = 3
            39, // while False:
            40, //     pass
            // 41: else (NOT recorded today)
            42, //     pass
            44, // for i in range(1):
            45, //     pass
            // 46: else (NOT recorded today)
            47, //     pass
            49, // async def with_async_for
            50, //     async for ...
            51, //         pass
            53, // with open ...
            54, //     pass
            56, // async def with_async_with
            57, //     async with ...
            58, //         pass
            60, // try:
            61, //     raise
            // 62: except ValueError (NOT recorded today)
            63, //     pass
            // 64: except (KeyError, TypeError) (NOT recorded today)
            65, //     pass
            // 66: else (NOT recorded today)
            67, //     pass
            // 68: finally (NOT recorded today)
            69, //     pass
            71, // try:
            72, //     raise
            // 73: except* (NOT recorded today)
            74, //     pass
            76, // match x:
            // 77: case 1 (NOT recorded today)
            78, //     pass
            // 79: case _ (NOT recorded today)
            80, //     pass
            82, // global g
            83, // nonlocal n
            84, // assert True
            85, // raise RuntimeError
            86, // return None
            87, // yield 1 (Stmt::Expr around a Yield)
            88, // pass
            89, // break
            90, // continue
            91, // type Alias = int
        ];

        assert_eq!(
            actual, expected,
            "executable-line set drifted from baseline; if this is intentional, update the expected list above"
        );
    }
}
