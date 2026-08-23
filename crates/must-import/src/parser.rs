use std::collections::HashSet;

/// The complete AST produced by parsing a sequence of lexer tokens.
#[derive(Debug, PartialEq, Clone)]
pub struct MakefileAst {
    pub nodes: Vec<AstNode>,
}

/// A single logical construct parsed from a Makefile.
#[derive(Debug, PartialEq, Clone)]
pub enum AstNode {
    /// A variable assignment.
    Variable {
        name: String,
        op: crate::lexer::AssignOp,
        value: String,
    },
    /// A build rule, possibly phony.
    Rule {
        target: String,
        deps: Vec<String>,
        recipe_lines: Vec<String>,
        phony: bool,
    },
    /// A pattern rule that cannot be translated; preserved for reporting.
    PatternRuleTodo { original: String },
    /// An `include` directive that cannot be inlined; preserved for reporting.
    IncludeTodo { path: String },
    /// A line that did not match any recognised pattern; preserved verbatim.
    Unrecognized { original: String },
}

/// Parse a flat sequence of lexer tokens into a [`MakefileAst`].
///
/// **Pass 1** — scan all tokens and collect every target name that appears in
/// a `PhonyDecl` into a `HashSet`.
///
/// **Pass 2** — walk the token slice in order and emit `AstNode` values:
/// - `VarAssign` → `Variable`
/// - `RuleHeader` → consume all immediately-following `RecipeLine` tokens,
///   then emit a `Rule` (with `phony` determined by the phony set)
/// - `PatternRule` → `PatternRuleTodo`
/// - `IncludeDirective` → `IncludeTodo`
/// - `Unrecognized` → `Unrecognized`
/// - `Comment`, `Blank`, `PhonyDecl` → discarded (no AST node)
pub fn parse(tokens: Vec<crate::lexer::Token>) -> MakefileAst {
    use crate::lexer::Token;

    // Pass 1: collect phony target names.
    let phony_set: HashSet<String> = tokens
        .iter()
        .filter_map(|t| {
            if let Token::PhonyDecl(targets) = t {
                Some(targets.iter().cloned())
            } else {
                None
            }
        })
        .flatten()
        .collect();

    // Pass 2: build AST nodes.
    let mut nodes = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::VarAssign { name, op, value } => {
                nodes.push(AstNode::Variable {
                    name: name.clone(),
                    op: op.clone(),
                    value: value.clone(),
                });
                i += 1;
            }
            Token::RuleHeader { target, deps } => {
                let target = target.clone();
                let deps = deps.clone();
                i += 1;

                // Consume following RecipeLine tokens.  Blank lines and
                // comments inside a recipe do not terminate it (GNU make
                // allows both); the recipe ends at a new rule, EOF, or any
                // other non-blank non-comment non-tab line.
                let mut recipe_lines = Vec::new();
                while i < tokens.len() {
                    match &tokens[i] {
                        Token::RecipeLine(line) => {
                            recipe_lines.push(line.clone());
                            i += 1;
                        }
                        Token::Blank | Token::Comment(_) => {
                            i += 1;
                        }
                        _ => break,
                    }
                }

                let phony = phony_set.contains(&target);
                nodes.push(AstNode::Rule {
                    target,
                    deps,
                    recipe_lines,
                    phony,
                });
            }
            Token::PatternRule { pattern, deps } => {
                let original = if deps.is_empty() {
                    pattern.clone()
                } else {
                    format!("{}: {}", pattern, deps.join(" "))
                };
                nodes.push(AstNode::PatternRuleTodo { original });
                i += 1;
            }
            Token::IncludeDirective(path) => {
                nodes.push(AstNode::IncludeTodo { path: path.clone() });
                i += 1;
            }
            Token::Unrecognized(s) => {
                nodes.push(AstNode::Unrecognized {
                    original: s.clone(),
                });
                i += 1;
            }
            // Comment, Blank, PhonyDecl → skip.
            Token::Comment(_) | Token::Blank | Token::PhonyDecl(_) => {
                i += 1;
            }
            // RecipeLine outside a rule context → skip.
            Token::RecipeLine(_) => {
                i += 1;
            }
        }
    }

    MakefileAst { nodes }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{AssignOp, Token};

    #[test]
    fn empty_tokens_empty_ast() {
        let ast = parse(vec![]);
        assert!(ast.nodes.is_empty());
    }

    #[test]
    fn variable_token_produces_variable_node() {
        let tokens = vec![Token::VarAssign {
            name: "FOO".to_string(),
            op: AssignOp::Simple,
            value: "bar".to_string(),
        }];
        let ast = parse(tokens);
        assert_eq!(
            ast.nodes,
            vec![AstNode::Variable {
                name: "FOO".to_string(),
                op: AssignOp::Simple,
                value: "bar".to_string(),
            }]
        );
    }

    #[test]
    fn rule_with_recipe_lines() {
        let tokens = vec![
            Token::RuleHeader {
                target: "build".to_string(),
                deps: vec![],
            },
            Token::RecipeLine("cargo build".to_string()),
            Token::RecipeLine("echo done".to_string()),
        ];
        let ast = parse(tokens);
        assert_eq!(ast.nodes.len(), 1);
        if let AstNode::Rule { recipe_lines, .. } = &ast.nodes[0] {
            assert_eq!(recipe_lines.len(), 2);
            assert_eq!(recipe_lines[0], "cargo build");
            assert_eq!(recipe_lines[1], "echo done");
        } else {
            panic!("expected Rule node");
        }
    }

    #[test]
    fn rule_with_no_recipe_lines() {
        let tokens = vec![
            Token::RuleHeader {
                target: "build".to_string(),
                deps: vec![],
            },
            Token::Blank,
        ];
        let ast = parse(tokens);
        assert_eq!(ast.nodes.len(), 1);
        if let AstNode::Rule { recipe_lines, .. } = &ast.nodes[0] {
            assert!(recipe_lines.is_empty());
        } else {
            panic!("expected Rule node");
        }
    }

    #[test]
    fn blank_line_inside_recipe_does_not_terminate_it() {
        let tokens = vec![
            Token::RuleHeader {
                target: "build".to_string(),
                deps: vec![],
            },
            Token::RecipeLine("echo first".to_string()),
            Token::Blank,
            Token::RecipeLine("echo second".to_string()),
        ];
        let ast = parse(tokens);
        assert_eq!(ast.nodes.len(), 1);
        if let AstNode::Rule { recipe_lines, .. } = &ast.nodes[0] {
            assert_eq!(
                recipe_lines,
                &["echo first".to_string(), "echo second".to_string()]
            );
        } else {
            panic!("expected Rule node");
        }
    }

    #[test]
    fn comment_inside_recipe_does_not_terminate_it() {
        let tokens = vec![
            Token::RuleHeader {
                target: "build".to_string(),
                deps: vec![],
            },
            Token::RecipeLine("echo first".to_string()),
            Token::Comment("mid-recipe comment".to_string()),
            Token::RecipeLine("echo second".to_string()),
        ];
        let ast = parse(tokens);
        assert_eq!(ast.nodes.len(), 1);
        if let AstNode::Rule { recipe_lines, .. } = &ast.nodes[0] {
            assert_eq!(
                recipe_lines,
                &["echo first".to_string(), "echo second".to_string()]
            );
        } else {
            panic!("expected Rule node");
        }
    }

    #[test]
    fn non_tab_line_after_blank_terminates_recipe() {
        let tokens = vec![
            Token::RuleHeader {
                target: "build".to_string(),
                deps: vec![],
            },
            Token::RecipeLine("echo first".to_string()),
            Token::Blank,
            Token::VarAssign {
                name: "X".to_string(),
                op: AssignOp::Simple,
                value: "1".to_string(),
            },
            Token::RecipeLine("echo orphan".to_string()),
        ];
        let ast = parse(tokens);
        assert_eq!(ast.nodes.len(), 2);
        if let AstNode::Rule { recipe_lines, .. } = &ast.nodes[0] {
            assert_eq!(recipe_lines, &["echo first".to_string()]);
        } else {
            panic!("expected Rule node");
        }
        assert!(matches!(ast.nodes[1], AstNode::Variable { .. }));
    }

    #[test]
    fn blank_line_between_rules_keeps_recipes_separate() {
        let tokens = vec![
            Token::RuleHeader {
                target: "a".to_string(),
                deps: vec![],
            },
            Token::RecipeLine("echo a".to_string()),
            Token::Blank,
            Token::RuleHeader {
                target: "b".to_string(),
                deps: vec![],
            },
            Token::RecipeLine("echo b".to_string()),
        ];
        let ast = parse(tokens);
        assert_eq!(ast.nodes.len(), 2);
        if let AstNode::Rule {
            target,
            recipe_lines,
            ..
        } = &ast.nodes[1]
        {
            assert_eq!(target, "b");
            assert_eq!(recipe_lines, &["echo b".to_string()]);
        } else {
            panic!("expected Rule node");
        }
    }

    #[test]
    fn phony_decl_sets_phony_true() {
        let tokens = vec![
            Token::PhonyDecl(vec!["clean".to_string()]),
            Token::RuleHeader {
                target: "clean".to_string(),
                deps: vec![],
            },
            Token::RecipeLine("rm -rf target".to_string()),
        ];
        let ast = parse(tokens);
        assert_eq!(ast.nodes.len(), 1);
        if let AstNode::Rule { phony, target, .. } = &ast.nodes[0] {
            assert_eq!(target, "clean");
            assert!(*phony, "expected phony=true for 'clean'");
        } else {
            panic!("expected Rule node");
        }
    }

    #[test]
    fn non_phony_rule_is_not_phony() {
        let tokens = vec![Token::RuleHeader {
            target: "output.o".to_string(),
            deps: vec!["output.c".to_string()],
        }];
        let ast = parse(tokens);
        assert_eq!(ast.nodes.len(), 1);
        if let AstNode::Rule { phony, .. } = &ast.nodes[0] {
            assert!(!*phony, "expected phony=false");
        } else {
            panic!("expected Rule node");
        }
    }

    #[test]
    fn pattern_rule_becomes_todo() {
        let tokens = vec![Token::PatternRule {
            pattern: "%.o".to_string(),
            deps: vec!["%.c".to_string()],
        }];
        let ast = parse(tokens);
        assert_eq!(
            ast.nodes,
            vec![AstNode::PatternRuleTodo {
                original: "%.o: %.c".to_string(),
            }]
        );
    }

    #[test]
    fn include_becomes_todo() {
        let tokens = vec![Token::IncludeDirective("config.mk".to_string())];
        let ast = parse(tokens);
        assert_eq!(
            ast.nodes,
            vec![AstNode::IncludeTodo {
                path: "config.mk".to_string(),
            }]
        );
    }

    #[test]
    fn comment_and_blank_are_skipped() {
        let tokens = vec![
            Token::Comment("this is a comment".to_string()),
            Token::Blank,
            Token::VarAssign {
                name: "X".to_string(),
                op: AssignOp::Immediate,
                value: "1".to_string(),
            },
        ];
        let ast = parse(tokens);
        assert_eq!(ast.nodes.len(), 1);
        assert!(matches!(ast.nodes[0], AstNode::Variable { .. }));
    }

    #[test]
    fn unrecognized_token_preserved() {
        let tokens = vec![Token::Unrecognized("ifeq ($(CC),gcc)".to_string())];
        let ast = parse(tokens);
        assert_eq!(
            ast.nodes,
            vec![AstNode::Unrecognized {
                original: "ifeq ($(CC),gcc)".to_string(),
            }]
        );
    }
}
