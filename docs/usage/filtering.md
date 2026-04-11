# Filtering tests

Karva selects which tests to run with **filter expressions**, a small
language inspired by [nextest's filtersets]. A single `-E` / `--filter`
flag composes name matching, tag matching, and boolean logic into one
expression — there are no separate `--tag` or `--match` flags.

```bash
karva test -E 'tag(slow)'
karva test -E 'test(/^auth::/) & not tag(flaky)'
karva test -E '(tag(fast) | tag(unit)) - tag(flaky)'
```

When `-E` is passed more than once, a test runs if it matches **any** of
the expressions (OR across flags):

```bash
karva test -E 'tag(slow)' -E 'tag(integration)'
```

Expressions are evaluated against every discovered test. A test is run
iff the expression evaluates to true for it; otherwise it is skipped.

## Predicates

A filter expression is built from predicates combined with boolean
operators. Karva currently supports two predicates:

| Predicate         | Evaluated against            |
| ----------------- | ---------------------------- |
| `test(<matcher>)` | The fully qualified test name, e.g. `mod::sub::test_login` |
| `tag(<matcher>)`  | Each custom tag on the test; matches if *any* tag matches  |

## Matchers

A matcher describes how a predicate's argument is compared against the
value it is evaluated over. There are four matcher kinds, distinguished
by a single-character prefix:

| Prefix        | Kind       | Meaning                                 |
| ------------- | ---------- | --------------------------------------- |
| `=`           | Exact      | Value must equal the pattern exactly.   |
| `~`           | Substring  | Pattern must appear anywhere in value.  |
| `/.../`       | Regex      | Value must match the [Rust regex].      |
| `#`           | Glob       | Value must match the [glob pattern].    |
| *(no prefix)* | Default    | Substring for `test()`, exact for `tag()`. |

Examples:

```text
test(=mod::test_login)   # exact test name
test(~login)             # any test whose name contains "login"
test(/^mod::test_log/)   # regex — all tests in mod:: starting with test_log
test(#*_login_*)         # glob — matches wildcards like _login_
tag(slow)                # tag exactly named "slow"
tag(~slo)                # any tag containing "slo"
tag(#py3*)               # any tag matching the glob "py3*"
```

Regex defaults to partial matching (Rust `regex::is_match`), so anchors
like `^` and `$` must be written explicitly when you want a full match.

Strings may be quoted with `"..."` when they contain spaces or reserved
characters:

```text
tag(="my tag")
test(="mod::test with space")
```

Inside a quoted string or a regex literal, the delimiter can be escaped
with a backslash (`\"` and `\/` respectively); other backslashes are
preserved as-is so that regex metacharacters like `\d` or `\b` round-trip
without needing double escaping.

## Operators

| Operator                 | Meaning         | Example                            |
| ------------------------ | --------------- | ---------------------------------- |
| `&` or `and`             | Logical AND     | `tag(slow) & test(~login)`         |
| <code>&#124;</code> or `or` | Logical OR    | `tag(slow) or tag(fast)`           |
| `not` or `!`             | Logical NOT     | `not tag(flaky)`                   |
| `-`                      | Difference (and-not) | `tag(slow) - tag(flaky)`      |
| `( … )`                  | Grouping        | `(tag(a) | tag(b)) & tag(c)`       |

Precedence, from tightest to loosest: grouping → `not` → `&` / `-` → `|`.
So `tag(a) | tag(b) & tag(c)` parses as `tag(a) | (tag(b) & tag(c))`.

The `-` operator is shorthand for *and not*: `A - B` is equivalent to
`A & not B`. It is especially convenient for subtracting flaky or
platform-gated tests from a broader selection.

## Migration from `-t` and `-m`

Older releases of karva exposed separate `-t` / `--tag` and `-m` /
`--match` flags. Both have been replaced by `-E` / `--filter`:

| Before                                    | After                                    |
| ----------------------------------------- | ---------------------------------------- |
| `-t slow`                                 | `-E 'tag(slow)'`                         |
| `-t 'not slow'`                           | `-E 'not tag(slow)'`                     |
| `-t 'slow and integration'`               | `-E 'tag(slow) & tag(integration)'`     |
| `-t 'slow or integration'`                | `-E 'tag(slow) | tag(integration)'`     |
| `-t '(slow or fast) and not flaky'`       | `-E '(tag(slow) | tag(fast)) - tag(flaky)'` |
| `-m auth`                                 | `-E 'test(/auth/)'`                      |
| `-m '^test::test_login'`                  | `-E 'test(/^test::test_login/)'`        |
| `-m 'slow|fast'`                          | `-E 'test(/slow|fast/)'`                |
| `-t slow -m auth`                         | `-E 'tag(slow) & test(/auth/)'`         |

Everything the old flags supported is expressible with the new syntax,
and the new syntax also adds exact, substring, and glob matchers.

## Grammar

For reference, the full grammar:

```text
filterset   ::= or_expr
or_expr     ::= and_expr (('|' | 'or') and_expr)*
and_expr    ::= unary_expr (('&' | 'and' | '-') unary_expr)*
unary_expr  ::= ('!' | 'not') unary_expr | atom
atom        ::= '(' or_expr ')' | predicate
predicate   ::= ('test' | 'tag') '(' matcher ')'
matcher     ::= '=' body      # exact
              | '~' body      # substring
              | '#' body      # glob
              | regex
              | body          # default
body        ::= identifier | string
regex       ::= '/' … '/'
string      ::= '"' … '"'
identifier  ::= [A-Za-z0-9_.:*?\[\]{}^$]+
```

[nextest's filtersets]: https://nexte.st/docs/filtersets/
[Rust regex]: https://docs.rs/regex/latest/regex/#syntax
[glob pattern]: https://docs.rs/globset/latest/globset/#syntax
