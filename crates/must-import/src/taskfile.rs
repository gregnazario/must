use crate::translate::{MustfileOutput, OutputRecipe};

pub(crate) fn parse_taskfile(input: &str) -> MustfileOutput {
    let mut output = MustfileOutput {
        env: std::collections::BTreeMap::new(),
        recipes: Vec::new(),
        todos: Vec::new(),
        skipped: Vec::new(),
    };

    let tasks = extract_tasks(input);

    for (name, task_content) in tasks {
        let mut deps: Vec<String> = Vec::new();
        let mut script_lines: Vec<String> = Vec::new();
        let mut desc: Option<String> = None;

        for field in extract_fields(&task_content) {
            match field.key.as_str() {
                "deps" => {
                    deps = parse_string_list(&field.value);
                }
                "cmds" => {
                    script_lines = parse_cmd_lines(&field.value);
                }
                "desc" | "description" => {
                    desc = Some(field.value.trim().trim_matches('"').to_string());
                }
                "dir" | "vars" | "env" | "sources" | "generates" | "status"
                | "preconditions" | "silent" | "interactive" | "internal"
                | "method" | "prefix" | "ignore_error" | "run" => {
                    let _ = desc;
                }
                _ => {}
            }
        }

        let script = script_lines.join("\n");
        output.recipes.push(OutputRecipe {
            name,
            deps,
            script,
            phony: true,
        });
    }

    if let Some(version) = extract_field(input, "version") {
        output.skipped.push(format!("version: {version}"));
    }

    output
}

struct Field {
    key: String,
    value: String,
}

fn extract_tasks(input: &str) -> Vec<(String, String)> {
    let mut tasks = Vec::new();
    let mut in_tasks = false;
    let mut task_name = String::new();
    let mut task_lines: Vec<String> = Vec::new();
    let mut task_indent: usize = 0;

    for line in input.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if !in_tasks {
            if line.starts_with("tasks:") {
                in_tasks = true;
            } else if line.starts_with("version:")
                || line.starts_with("env:")
                || line.starts_with("vars:")
                || line.starts_with("includes:")
            {
                continue;
            }
            continue;
        }

        let indent = line.len() - line.trim_start().len();

        if !task_name.is_empty() && indent <= task_indent && !trimmed.starts_with('-') {
            if !task_lines.is_empty() {
                tasks.push((task_name.clone(), task_lines.join("\n")));
            }
            task_name = String::new();
            task_lines = Vec::new();
        }

        if indent == 2 && trimmed.ends_with(':') && !trimmed.starts_with('-') {
            task_name = trimmed.trim_end_matches(':').to_string();
            task_indent = indent;
            task_lines = Vec::new();
        } else if !task_name.is_empty() {
            task_lines.push(line.to_string());
        }
    }

    if !task_name.is_empty() && !task_lines.is_empty() {
        tasks.push((task_name, task_lines.join("\n")));
    }

    tasks
}

fn extract_fields(content: &str) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut current_key = String::new();
    let mut current_value_lines: Vec<String> = Vec::new();
    let mut base_indent: usize = 0;
    let mut first = true;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let indent = line.len() - line.trim_start().len();

        if first {
            base_indent = indent;
            first = false;
        }

        if indent == base_indent {
            if !current_key.is_empty() {
                fields.push(Field {
                    key: current_key.clone(),
                    value: current_value_lines.join("\n"),
                });
            }
            if let Some(colon_pos) = trimmed.find(':') {
                current_key = trimmed[..colon_pos].to_string();
                let rest = trimmed[colon_pos + 1..].trim();
                current_value_lines = if rest.is_empty() {
                    Vec::new()
                } else {
                    vec![rest.to_string()]
                };
            }
        } else if !current_key.is_empty() {
            current_value_lines.push(trimmed.to_string());
        }
    }

    if !current_key.is_empty() {
        fields.push(Field {
            key: current_key.clone(),
            value: current_value_lines.join("\n"),
        });
    }

    fields
}

fn parse_string_list(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let inner = &trimmed[1..trimmed.len() - 1];
        return inner
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    let mut items = Vec::new();
    for line in value.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("- ") {
            let item = stripped.trim().trim_matches('"').to_string();
            if !item.is_empty() {
                items.push(item);
            }
        }
    }
    items
}

fn parse_cmd_lines(value: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for line in value.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("- ") {
            let cmd = stripped.trim().trim_matches('"').to_string();
            if !cmd.is_empty() {
                lines.push(cmd);
            }
        }
    }
    lines
}

fn extract_field(input: &str, field_name: &str) -> Option<String> {
    for line in input.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&format!("{field_name}:")) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let output = parse_taskfile("");
        assert!(output.recipes.is_empty());
    }

    #[test]
    fn simple_task() {
        let input = "version: '3'\n\ntasks:\n  build:\n    cmds:\n      - cargo build";
        let output = parse_taskfile(input);
        assert_eq!(output.recipes.len(), 1);
        assert_eq!(output.recipes[0].name, "build");
        assert_eq!(output.recipes[0].script, "cargo build");
    }

    #[test]
    fn task_with_deps() {
        let input = "version: '3'\n\ntasks:\n  test:\n    deps: [build]\n    cmds:\n      - cargo test";
        let output = parse_taskfile(input);
        assert_eq!(output.recipes.len(), 1);
        assert_eq!(output.recipes[0].deps, vec!["build"]);
    }

    #[test]
    fn task_with_deps_list() {
        let input = "version: '3'\n\ntasks:\n  release:\n    deps:\n      - build\n      - test\n    cmds:\n      - cargo build --release";
        let output = parse_taskfile(input);
        assert_eq!(output.recipes[0].deps, vec!["build", "test"]);
    }

    #[test]
    fn multiple_tasks() {
        let input = r#"version: '3'

tasks:
  build:
    cmds:
      - cargo build
  test:
    deps: [build]
    cmds:
      - cargo test
  clean:
    cmds:
      - cargo clean"#;
        let output = parse_taskfile(input);
        assert_eq!(output.recipes.len(), 3);
        assert_eq!(output.recipes[0].name, "build");
        assert_eq!(output.recipes[1].name, "test");
        assert_eq!(output.recipes[2].name, "clean");
    }

    #[test]
    fn multiline_cmds() {
        let input = "version: '3'\n\ntasks:\n  release:\n    cmds:\n      - cargo build --release\n      - cp target/release/app dist/";
        let output = parse_taskfile(input);
        assert_eq!(output.recipes[0].script, "cargo build --release\ncp target/release/app dist/");
    }

    #[test]
    fn version_skipped() {
        let input = "version: '3'\n\ntasks:\n  build:\n    cmds:\n      - echo hi";
        let output = parse_taskfile(input);
        assert!(output.skipped.iter().any(|s| s.contains("version")));
    }

    #[test]
    fn no_tasks_block() {
        let input = "version: '3'\n";
        let output = parse_taskfile(input);
        assert!(output.recipes.is_empty());
    }
}
