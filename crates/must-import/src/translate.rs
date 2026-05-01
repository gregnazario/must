use crate::parser::{AstNode, MakefileAst};
use std::collections::BTreeMap;

pub struct ImportResult {
    pub toml: String,
    pub report: String,
    pub translated_count: usize,
    pub skipped_count: usize,
    pub todo_count: usize,
}

// Private intermediate representation
pub(crate) struct MustfileOutput {
    pub env: BTreeMap<String, String>, // sorted for stable TOML output
    pub recipes: Vec<OutputRecipe>,
    pub todos: Vec<TodoItem>,
    pub skipped: Vec<String>,
}

pub(crate) struct OutputRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub script: String, // recipe lines joined by \n
    pub phony: bool,
}

pub(crate) struct TodoItem {
    pub kind: TodoKind,
    pub description: String,
}

pub(crate) enum TodoKind {
    PatternRule,
    Include,
}

pub fn translate(ast: MakefileAst) -> ImportResult {
    let mut output = MustfileOutput {
        env: BTreeMap::new(),
        recipes: Vec::new(),
        todos: Vec::new(),
        skipped: Vec::new(),
    };

    for node in ast.nodes {
        match node {
            AstNode::Variable { name, value, .. } => {
                if value.is_empty() {
                    output
                        .skipped
                        .push(format!("{name} = (empty value — skipped)"));
                } else {
                    output.env.insert(name, value);
                }
            }
            AstNode::Rule {
                target,
                deps,
                recipe_lines,
                phony,
            } => {
                let script = recipe_lines.join("\n");
                output.recipes.push(OutputRecipe {
                    name: target,
                    deps,
                    script,
                    phony,
                });
            }
            AstNode::PatternRuleTodo { original } => {
                output.todos.push(TodoItem {
                    kind: TodoKind::PatternRule,
                    description: original,
                });
            }
            AstNode::IncludeTodo { path } => {
                output.todos.push(TodoItem {
                    kind: TodoKind::Include,
                    description: path,
                });
            }
            AstNode::Unrecognized { original } => {
                output.skipped.push(original);
            }
        }
    }

    let translated_count = output.env.len() + output.recipes.len();
    let todo_count = output.todos.len();
    let skipped_count = output.skipped.len();
    let toml = crate::writer::write_toml(&output);
    let report = crate::report::write_report(&output, translated_count, todo_count, skipped_count);

    ImportResult {
        toml,
        report,
        translated_count,
        todo_count,
        skipped_count,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::AssignOp;
    use crate::parser::{AstNode, MakefileAst};

    fn ast(nodes: Vec<AstNode>) -> MakefileAst {
        MakefileAst { nodes }
    }

    #[test]
    fn empty_ast_zero_counts() {
        let r = translate(ast(vec![]));
        assert_eq!(r.translated_count, 0);
        assert_eq!(r.todo_count, 0);
        assert_eq!(r.skipped_count, 0);
    }

    #[test]
    fn variable_node_goes_into_env() {
        let r = translate(ast(vec![AstNode::Variable {
            name: "CC".into(),
            op: AssignOp::Simple,
            value: "gcc".into(),
        }]));
        assert_eq!(r.translated_count, 1);
        assert!(r.toml.contains("CC = \"gcc\""));
    }

    #[test]
    fn empty_value_var_is_skipped() {
        let r = translate(ast(vec![AstNode::Variable {
            name: "EMPTY".into(),
            op: AssignOp::Simple,
            value: "".into(),
        }]));
        assert_eq!(r.translated_count, 0);
        assert_eq!(
            r.skipped_count, 1,
            "empty-value variable should appear in skipped_count"
        );
        assert!(
            r.report.contains("EMPTY"),
            "skipped variable name should appear in report"
        );
    }

    #[test]
    fn rule_node_goes_into_recipes() {
        let r = translate(ast(vec![AstNode::Rule {
            target: "build".into(),
            deps: vec![],
            recipe_lines: vec!["gcc -o app main.c".into()],
            phony: false,
        }]));
        assert_eq!(r.translated_count, 1);
        assert!(r.toml.contains("[recipe.\"build\"]"));
    }

    #[test]
    fn pattern_rule_increments_todo() {
        let r = translate(ast(vec![AstNode::PatternRuleTodo {
            original: "%.o: %.c".into(),
        }]));
        assert_eq!(r.todo_count, 1);
        assert!(r.report.contains("Pattern rule"));
    }

    #[test]
    fn include_increments_todo() {
        let r = translate(ast(vec![AstNode::IncludeTodo {
            path: "common.mk".into(),
        }]));
        assert_eq!(r.todo_count, 1);
        assert!(r.report.contains("Include"));
    }

    #[test]
    fn unrecognized_increments_skipped() {
        let r = translate(ast(vec![AstNode::Unrecognized {
            original: "ifeq ($(OS),Windows)".into(),
        }]));
        assert_eq!(r.skipped_count, 1);
        assert_eq!(r.translated_count, 0);
        assert_eq!(r.todo_count, 0);
        assert!(r.report.contains("ifeq"));
    }
}
