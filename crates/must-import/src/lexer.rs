/// Assignment operators supported in Makefiles.
#[derive(Debug, PartialEq, Clone)]
pub enum AssignOp {
    /// Recursively-expanded assignment (`=`)
    Simple,
    /// Simply-expanded assignment (`:=`)
    Immediate,
    /// Conditional assignment (`?=`)
    Conditional,
    /// Append assignment (`+=`)
    Append,
}

/// A single logical line from a Makefile, classified by kind.
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    /// A comment line (`# …`).  The stored string is the comment body, trimmed.
    Comment(String),
    /// A completely blank line.
    Blank,
    /// A variable assignment line.
    VarAssign {
        name: String,
        op: AssignOp,
        value: String,
    },
    /// A `.PHONY:` declaration listing one or more targets.
    PhonyDecl(Vec<String>),
    /// A normal rule header (`target: deps…`).
    RuleHeader { target: String, deps: Vec<String> },
    /// A recipe line (starts with a tab character).  The stored string has the
    /// leading tab stripped.
    RecipeLine(String),
    /// An `include` directive.  The stored string is the path, trimmed.
    IncludeDirective(String),
    /// A pattern rule (`%.o: %.c`).
    PatternRule { pattern: String, deps: Vec<String> },
    /// Any line that did not match a recognised pattern.
    Unrecognized(String),
}

/// Tokenize a Makefile source string into a sequence of [`Token`]s.
///
/// Each physical line of `input` produces exactly one token.  Continuation
/// lines (`\`) are **not** joined — that complexity is left to the parser.
pub fn tokenize(input: &str) -> Vec<Token> {
    input.lines().map(classify_line).collect()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn classify_line(raw: &str) -> Token {
    // Trim only *trailing* whitespace; preserve leading tab for recipe detection.
    let line = raw.trim_end();

    // 1. Blank line
    if line.is_empty() {
        return Token::Blank;
    }

    // 2. Comment
    if let Some(rest) = line.strip_prefix('#') {
        let body = rest.trim().to_string();
        return Token::Comment(body);
    }

    // 3. Recipe (leading tab)
    if let Some(rest) = line.strip_prefix('\t') {
        return Token::RecipeLine(rest.to_string());
    }

    // 4. .PHONY directive
    if let Some(rest) = line.strip_prefix(".PHONY:") {
        let targets: Vec<String> = rest
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        return Token::PhonyDecl(targets);
    }

    // 5. include directive
    if let Some(rest) = line.strip_prefix("include ") {
        let path = rest.trim().to_string();
        return Token::IncludeDirective(path);
    }

    // 6. Variable assignment  —  check operators longest-first to avoid
    //    `?=` being matched as `=`, etc.
    if let Some(tok) = try_var_assign(line) {
        return tok;
    }

    // 7. Rule header (normal or pattern)
    if let Some(tok) = try_rule(line) {
        return tok;
    }

    // 8. Fallback
    Token::Unrecognized(line.to_string())
}

/// Attempt to parse `line` as a variable assignment.
///
/// Operators are tried in order: `?=`, `:=`, `+=`, `=`.
fn try_var_assign(line: &str) -> Option<Token> {
    // Each entry: (operator string, AssignOp variant)
    const OPS: &[(&str, AssignOp)] = &[
        ("?=", AssignOp::Conditional),
        (":=", AssignOp::Immediate),
        ("+=", AssignOp::Append),
        ("=", AssignOp::Simple),
    ];

    for (op_str, op_kind) in OPS {
        if let Some(pos) = find_op(line, op_str) {
            let name = line[..pos].trim().to_string();
            if name.is_empty() {
                continue; // Not a valid variable name — keep trying.
            }
            // A valid Makefile variable name contains only word chars, hyphens, dots
            // (no parens, dollar signs, whitespace, or slashes).
            // Lines like `$(eval $(call TEMPLATE,foo=bar))` contain `=` inside a
            // macro expression; the extracted "name" will contain `$(` which makes
            // it invalid — fall through to try_rule / Unrecognized instead.
            if name
                .chars()
                .any(|c| matches!(c, '(' | ')' | '$' | ' ' | '\t'))
            {
                continue;
            }
            let value = line[pos + op_str.len()..].trim().to_string();
            return Some(Token::VarAssign {
                name,
                op: op_kind.clone(),
                value,
            });
        }
    }
    None
}

/// Find the byte position of `op` in `line`, being careful not to confuse
/// `=` with `:=`, `?=`, or `+=`.
fn find_op(line: &str, op: &str) -> Option<usize> {
    match op {
        "=" => {
            // Find a bare `=` that is not preceded by `:`, `?`, or `+`.
            let bytes = line.as_bytes();
            for (i, &b) in bytes.iter().enumerate() {
                if b == b'=' {
                    let prev = if i > 0 { bytes[i - 1] } else { 0 };
                    if prev != b':' && prev != b'?' && prev != b'+' {
                        return Some(i);
                    }
                }
            }
            None
        }
        other => line.find(other),
    }
}

/// Attempt to parse `line` as a rule header.
///
/// Splits on the first `:` (treating `::` like `:`), then decides between
/// [`Token::PatternRule`] and [`Token::RuleHeader`] based on whether the
/// target contains `%`.
fn try_rule(line: &str) -> Option<Token> {
    // Find the first `:`.
    let colon_pos = line.find(':')?;

    let target = line[..colon_pos].trim().to_string();
    if target.is_empty() {
        return None;
    }

    // Skip a second `:` if present (double-colon rule — treat as ordinary).
    let after_colon = {
        let rest = &line[colon_pos + 1..];
        if let Some(stripped) = rest.strip_prefix(':') {
            stripped
        } else {
            rest
        }
    };

    let deps: Vec<String> = after_colon
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    if target.contains('%') {
        Some(Token::PatternRule {
            pattern: target,
            deps,
        })
    } else {
        Some(Token::RuleHeader { target, deps })
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: tokenize a single line.
    fn tok(line: &str) -> Token {
        let mut v = tokenize(line);
        assert_eq!(v.len(), 1, "expected exactly 1 token for input {:?}", line);
        v.remove(0)
    }

    // 1. Empty string → empty vec
    #[test]
    fn empty_input_yields_empty_vec() {
        assert_eq!(tokenize(""), Vec::<Token>::new());
    }

    // 2. Blank line → [Blank]
    // A blank line inside a multi-line string produces a Blank token.
    // Note: tokenize("") returns an empty vec (test 1); a blank *line* must
    // come from a string that contains at least one line, e.g. a single space
    // or a newline-terminated empty string.
    #[test]
    fn blank_line() {
        // A single space (only whitespace) trims to empty → Blank.
        assert_eq!(tok(" "), Token::Blank);
    }

    // 3. Comment
    #[test]
    fn comment_line() {
        assert_eq!(tok("# hello"), Token::Comment("hello".to_string()));
    }

    // 3b. Comment with no space after #
    #[test]
    fn comment_no_space() {
        assert_eq!(tok("#hello"), Token::Comment("hello".to_string()));
    }

    // 4. Simple assignment
    #[test]
    fn var_assign_simple() {
        assert_eq!(
            tok("FOO = bar"),
            Token::VarAssign {
                name: "FOO".to_string(),
                op: AssignOp::Simple,
                value: "bar".to_string(),
            }
        );
    }

    // 5. Immediate assignment with multi-word value
    #[test]
    fn var_assign_immediate() {
        assert_eq!(
            tok("FOO := bar baz"),
            Token::VarAssign {
                name: "FOO".to_string(),
                op: AssignOp::Immediate,
                value: "bar baz".to_string(),
            }
        );
    }

    // 6. Conditional assignment
    #[test]
    fn var_assign_conditional() {
        assert_eq!(
            tok("FOO ?= default"),
            Token::VarAssign {
                name: "FOO".to_string(),
                op: AssignOp::Conditional,
                value: "default".to_string(),
            }
        );
    }

    // 7. Append assignment
    #[test]
    fn var_assign_append() {
        assert_eq!(
            tok("FOO += extra"),
            Token::VarAssign {
                name: "FOO".to_string(),
                op: AssignOp::Append,
                value: "extra".to_string(),
            }
        );
    }

    // 8. .PHONY with targets
    #[test]
    fn phony_with_targets() {
        assert_eq!(
            tok(".PHONY: all test clean"),
            Token::PhonyDecl(vec![
                "all".to_string(),
                "test".to_string(),
                "clean".to_string()
            ])
        );
    }

    // 9. .PHONY with no targets
    #[test]
    fn phony_empty() {
        assert_eq!(tok(".PHONY:"), Token::PhonyDecl(vec![]));
    }

    // 10. Rule header with deps
    #[test]
    fn rule_header_with_deps() {
        assert_eq!(
            tok("all: build test"),
            Token::RuleHeader {
                target: "all".to_string(),
                deps: vec!["build".to_string(), "test".to_string()],
            }
        );
    }

    // 11. Rule header with no deps
    #[test]
    fn rule_header_no_deps() {
        assert_eq!(
            tok("all:"),
            Token::RuleHeader {
                target: "all".to_string(),
                deps: vec![],
            }
        );
    }

    // 12. Recipe line
    #[test]
    fn recipe_line() {
        assert_eq!(
            tok("\techo hello"),
            Token::RecipeLine("echo hello".to_string())
        );
    }

    // 13. include directive
    #[test]
    fn include_directive() {
        assert_eq!(
            tok("include foo.mk"),
            Token::IncludeDirective("foo.mk".to_string())
        );
    }

    // 14. Pattern rule
    #[test]
    fn pattern_rule() {
        assert_eq!(
            tok("%.o: %.c"),
            Token::PatternRule {
                pattern: "%.o".to_string(),
                deps: vec!["%.c".to_string()],
            }
        );
    }

    // 15. Multi-line input
    #[test]
    fn multi_line() {
        let input = "FOO = bar\n\nall:\n\techo done";
        let tokens = tokenize(input);
        assert_eq!(tokens.len(), 4);
        assert_eq!(
            tokens[0],
            Token::VarAssign {
                name: "FOO".to_string(),
                op: AssignOp::Simple,
                value: "bar".to_string(),
            }
        );
        assert_eq!(tokens[1], Token::Blank);
        assert_eq!(
            tokens[2],
            Token::RuleHeader {
                target: "all".to_string(),
                deps: vec![],
            }
        );
        assert_eq!(tokens[3], Token::RecipeLine("echo done".to_string()));
    }

    // 16. Unrecognized: conditional directive
    #[test]
    fn unrecognized_ifeq() {
        assert_eq!(
            tok("ifeq ($(CC),gcc)"),
            Token::Unrecognized("ifeq ($(CC),gcc)".to_string())
        );
    }

    // Extra: variable with $(VAR) in value passes through untouched
    #[test]
    fn var_assign_with_expansion() {
        assert_eq!(
            tok("CFLAGS := -Wall $(EXTRA)"),
            Token::VarAssign {
                name: "CFLAGS".to_string(),
                op: AssignOp::Immediate,
                value: "-Wall $(EXTRA)".to_string(),
            }
        );
    }

    // Extra: double-colon rule treated as ordinary rule
    #[test]
    fn double_colon_rule() {
        assert_eq!(
            tok("all:: build"),
            Token::RuleHeader {
                target: "all".to_string(),
                deps: vec!["build".to_string()],
            }
        );
    }

    // Extra: line with `=` inside a macro expression is NOT a VarAssign
    #[test]
    fn macro_line_with_equals_is_unrecognized() {
        // The `=` inside `$(call TEMPLATE,foo=bar)` must not cause the line to
        // be classified as a variable assignment with a garbage name.
        let t = tok("$(eval $(call TEMPLATE,foo=bar))");
        assert_eq!(
            t,
            Token::Unrecognized("$(eval $(call TEMPLATE,foo=bar))".to_string())
        );
    }

    // Extra: trailing whitespace stripped
    #[test]
    fn trailing_whitespace_stripped() {
        assert_eq!(
            tok("FOO = bar   "),
            Token::VarAssign {
                name: "FOO".to_string(),
                op: AssignOp::Simple,
                value: "bar".to_string(),
            }
        );
    }
}
