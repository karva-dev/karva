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

- `test(<matcher>)` — evaluated against the fully qualified test name,
  e.g. `mod::sub::test_login`.
- `tag(<matcher>)` — evaluated against each custom tag on the test;
  matches if *any* tag matches.

## Matchers

A matcher describes how a predicate's argument is compared against the
value it is evaluated over. There are four matcher kinds, distinguished
by a single-character prefix:

- `=foo` — exact: the value must equal the pattern exactly.
- `~foo` — substring: the pattern must appear anywhere in the value.
- `/foo/` — regex: the value must match the [Rust regex].
- `#foo` — glob: the value must match the [glob pattern].
- no prefix — defaults to substring for `test()` and exact for `tag()`.

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

- `&` (or `and`) — logical AND, e.g. `tag(slow) & test(~login)`
- `|` (or `or`) — logical OR, e.g. `tag(slow) or tag(fast)`
- `not` (or `!`) — logical NOT, e.g. `not tag(flaky)`
- `-` — difference, shorthand for "and not", e.g. `tag(slow) - tag(flaky)`
- `( … )` — grouping, e.g. `(tag(a) or tag(b)) and tag(c)`

Precedence, from tightest to loosest: grouping, then `not`, then `&` and
`-`, then `|`. So `tag(a) | tag(b) & tag(c)` parses as
`tag(a) | (tag(b) & tag(c))`.

The `-` operator is shorthand for *and not*: `A - B` is equivalent to
`A & not B`. It is especially convenient for subtracting flaky or
platform-gated tests from a broader selection.

## Migration from `-t` and `-m`

Older releases of karva exposed separate `-t` / `--tag` and `-m` /
`--match` flags. Both have been replaced by `-E` / `--filter`:

- `-t slow` becomes `-E 'tag(slow)'`
- `-t 'not slow'` becomes `-E 'not tag(slow)'`
- `-t 'slow and integration'` becomes `-E 'tag(slow) & tag(integration)'`
- `-t 'slow or integration'` becomes `-E 'tag(slow) or tag(integration)'`
- `-t '(slow or fast) and not flaky'` becomes `-E '(tag(slow) or tag(fast)) - tag(flaky)'`
- `-m auth` becomes `-E 'test(/auth/)'`
- `-m '^test::test_login'` becomes `-E 'test(/^test::test_login/)'`
- `-m 'slow|fast'` becomes `-E 'test(/slow|fast/)'`
- `-t slow -m auth` becomes `-E 'tag(slow) & test(/auth/)'`

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

[glob pattern]: https://docs.rs/globset/latest/globset/#syntax
[nextest's filtersets]: https://nexte.st/docs/filtersets/
[rust regex]: https://docs.rs/regex/latest/regex/#syntax
