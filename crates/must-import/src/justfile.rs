use crate::translate::{MustfileOutput, OutputRecipe};

pub(crate) fn parse_justfile(input: &str) -> MustfileOutput {
    let mut output = MustfileOutput {
        env: std::collections::BTreeMap::new(),
        recipes: Vec::new(),
        todos: Vec::new(),
        skipped: Vec::new(),
    };

    let mut current_recipe: Option<OutputRecipe> = None;
    let mut in_recipe = false;

    for raw_line in input.lines() {
        let line = raw_line;

        if line.trim_start().starts_with('#') {
            continue;
        }

        if line.starts_with("export ") {
            if let Some(rest) = line.strip_prefix("export ")
                && let Some((key, value)) = rest.split_once('=')
            {
                let key = key.trim().to_string();
                let value = value.trim().trim_matches('"').to_string();
                if !key.is_empty() && !value.is_empty() {
                    output.env.insert(key, value);
                }
            }
            continue;
        }

        if line.starts_with("set ") {
            output.skipped.push(line.to_string());
            continue;
        }

        if line.starts_with("alias ") || line.starts_with("mod ") {
            output.skipped.push(line.to_string());
            continue;
        }

        if !line.starts_with(' ')
            && !line.starts_with('\t')
            && !line.is_empty()
            && line.contains(':')
            && !line.contains("::=")
        {
            if let Some(recipe) = current_recipe.take() {
                output.recipes.push(recipe);
            }

            let (target_part, deps_part) = if let Some(idx) = line.find(':') {
                (&line[..idx], &line[idx + 1..])
            } else {
                (line, "")
            };

            let target = target_part
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();

            if target.is_empty() || target == "default" {
                if target == "default" {
                    output.skipped.push(line.to_string());
                }
                in_recipe = false;
                continue;
            }

            let mut deps: Vec<String> = deps_part
                .split_whitespace()
                .filter(|d| !d.starts_with('"') && !d.contains('='))
                .map(|d| d.trim_matches('"').to_string())
                .collect();

            deps.retain(|d| !d.is_empty());

            in_recipe = true;
            current_recipe = Some(OutputRecipe {
                name: target,
                deps,
                script: String::new(),
                phony: true,
            });
            continue;
        }

        if in_recipe {
            if let Some(ref mut recipe) = current_recipe {
                let script_line = line.trim_start();
                if !script_line.is_empty() {
                    if !recipe.script.is_empty() {
                        recipe.script.push('\n');
                    }
                    recipe.script.push_str(script_line);
                }
            }
        } else if !line.trim().is_empty() {
            let trimmed = line.trim();
            if !trimmed.starts_with('#')
                && !trimmed.starts_with("import")
                && !trimmed.is_empty()
            {
                output.skipped.push(trimmed.to_string());
            }
        }
    }

    if let Some(recipe) = current_recipe.take() {
        output.recipes.push(recipe);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let output = parse_justfile("");
        assert!(output.recipes.is_empty());
        assert!(output.env.is_empty());
    }

    #[test]
    fn simple_recipe() {
        let input = "build:\n    cargo build";
        let output = parse_justfile(input);
        assert_eq!(output.recipes.len(), 1);
        assert_eq!(output.recipes[0].name, "build");
        assert_eq!(output.recipes[0].script, "cargo build");
    }

    #[test]
    fn recipe_with_deps() {
        let input = "test: build\n    cargo test";
        let output = parse_justfile(input);
        assert_eq!(output.recipes.len(), 1);
        assert_eq!(output.recipes[0].name, "test");
        assert_eq!(output.recipes[0].deps, vec!["build"]);
    }

    #[test]
    fn multiple_recipes() {
        let input = "build:\n    cargo build\n\ntest: build\n    cargo test\n\nclean:\n    cargo clean";
        let output = parse_justfile(input);
        assert_eq!(output.recipes.len(), 3);
        assert_eq!(output.recipes[0].name, "build");
        assert_eq!(output.recipes[1].name, "test");
        assert_eq!(output.recipes[2].name, "clean");
    }

    #[test]
    fn export_env() {
        let input = "export RUST_LOG = \"warn\"\n\nbuild:\n    cargo build";
        let output = parse_justfile(input);
        assert_eq!(output.env.get("RUST_LOG").unwrap(), "warn");
    }

    #[test]
    fn comments_ignored() {
        let input = "# This is a comment\nbuild:\n    cargo build";
        let output = parse_justfile(input);
        assert_eq!(output.recipes.len(), 1);
    }

    #[test]
    fn default_skipped() {
        let input = "default: build\n\nbuild:\n    cargo build";
        let output = parse_justfile(input);
        assert_eq!(output.recipes.len(), 1);
        assert_eq!(output.recipes[0].name, "build");
    }

    #[test]
    fn multiline_script() {
        let input = "release:\n    cargo build --release\n    cp target/release/app dist/";
        let output = parse_justfile(input);
        assert_eq!(output.recipes[0].script, "cargo build --release\ncp target/release/app dist/");
    }

    #[test]
    fn set_skipped() {
        let input = "set shell := [\"bash\", \"-c\"]\n\nbuild:\n    echo hi";
        let output = parse_justfile(input);
        assert_eq!(output.recipes.len(), 1);
        assert!(output.skipped.iter().any(|s| s.starts_with("set ")));
    }
}
