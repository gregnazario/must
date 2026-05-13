use crate::translate::MustfileOutput;

pub(crate) fn write_toml(output: &MustfileOutput) -> String {
    let mut out = String::new();

    out.push_str("[project]\nname = \"imported\"\n");

    if !output.env.is_empty() {
        out.push_str("\n[env.global]\n");
        for (k, v) in &output.env {
            let escaped = v.replace('\\', "\\\\").replace('"', "\\\"");
            let key = toml_key(k);
            out.push_str(&format!("{key} = \"{escaped}\"\n"));
        }
    }

    for recipe in &output.recipes {
        // Quote the recipe name so targets like "dist/app" or "output.o" produce
        // valid TOML table keys (bare keys allow only [A-Za-z0-9_-]).
        let quoted_name = recipe.name.replace('\\', "\\\\").replace('"', "\\\"");
        out.push_str(&format!("\n[recipe.\"{}\"]\n", quoted_name));
        out.push_str("type = \"shell\"\n");
        if !recipe.deps.is_empty() {
            let deps: Vec<String> = recipe
                .deps
                .iter()
                .map(|d| {
                    let escaped = d.replace('\\', "\\\\").replace('"', "\\\"");
                    format!("\"{}\"", escaped)
                })
                .collect();
            out.push_str(&format!("deps = [{}]\n", deps.join(", ")));
        }
        if recipe.phony {
            out.push_str("phony = true\n");
        }
        if !recipe.script.is_empty() {
            // escape any """ in script content
            let safe_script = recipe.script.replace("\"\"\"", "\\\"\\\"\\\"");
            out.push_str(&format!("script = \"\"\"\n{safe_script}\n\"\"\"\n"));
        }
    }

    out
}

fn toml_key(key: &str) -> String {
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        let escaped = key.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        key.to_string()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translate::{MustfileOutput, OutputRecipe};
    use std::collections::BTreeMap;

    fn empty_output() -> MustfileOutput {
        MustfileOutput {
            env: BTreeMap::new(),
            recipes: vec![],
            todos: vec![],
            skipped: vec![],
        }
    }

    #[test]
    fn empty_output_has_project_section() {
        let toml = write_toml(&empty_output());
        assert!(toml.contains("[project]"));
        assert!(toml.contains("name = \"imported\""));
    }

    #[test]
    fn env_section_only_when_nonempty() {
        let toml = write_toml(&empty_output());
        assert!(!toml.contains("[env.global]"));
    }

    #[test]
    fn env_value_escaping() {
        let mut o = empty_output();
        o.env.insert("PATH".into(), r"C:\tools\".into());
        let toml = write_toml(&o);
        assert!(toml.contains(r#"PATH = "C:\\tools\\""#));
    }

    #[test]
    fn recipe_name_with_slash_is_quoted() {
        let mut o = empty_output();
        o.recipes.push(OutputRecipe {
            name: "dist/app".into(),
            deps: vec![],
            script: String::new(),
            phony: false,
        });
        let toml = write_toml(&o);
        assert!(toml.contains(r#"[recipe."dist/app"]"#));
    }

    #[test]
    fn dep_name_with_special_chars_is_escaped() {
        let mut o = empty_output();
        o.recipes.push(OutputRecipe {
            name: "build".into(),
            deps: vec!["dist/\"output\"".into(), r"C:\lib".into()],
            script: String::new(),
            phony: false,
        });
        let toml = write_toml(&o);
        assert!(
            toml.contains(r#""dist/\"output\"""#),
            "double-quotes inside dep name must be escaped"
        );
        assert!(
            toml.contains(r#""C:\\lib""#),
            "backslashes inside dep name must be escaped"
        );
    }

    #[test]
    fn recipe_with_deps_and_phony() {
        let mut o = empty_output();
        o.recipes.push(OutputRecipe {
            name: "clean".into(),
            deps: vec!["build".into()],
            script: "rm -rf dist".into(),
            phony: true,
        });
        let toml = write_toml(&o);
        assert!(toml.contains("[recipe.\"clean\"]"));
        assert!(toml.contains("phony = true"));
        assert!(toml.contains("deps = [\"build\"]"));
        assert!(toml.contains("rm -rf dist"));
    }

    #[test]
    fn env_key_with_dot_prefix_is_quoted() {
        let mut o = empty_output();
        o.env.insert(".DEFAULT_GOAL".into(), "help".into());
        let toml = write_toml(&o);
        assert!(
            toml.contains("\".DEFAULT_GOAL\" = \"help\""),
            "dot-prefixed keys must be quoted, got: {toml}"
        );
    }

    #[test]
    fn env_key_with_dot_in_middle_is_quoted() {
        let mut o = empty_output();
        o.env.insert("my.var".into(), "value".into());
        let toml = write_toml(&o);
        assert!(
            toml.contains("\"my.var\" = \"value\""),
            "keys with dots must be quoted, got: {toml}"
        );
    }

    #[test]
    fn env_key_plain_alphanumeric_is_bare() {
        let mut o = empty_output();
        o.env.insert("CC".into(), "gcc".into());
        let toml = write_toml(&o);
        assert!(
            toml.contains("CC = \"gcc\""),
            "plain keys should stay bare, got: {toml}"
        );
    }

    #[test]
    fn env_key_with_dash_is_bare() {
        let mut o = empty_output();
        o.env.insert("my-var".into(), "val".into());
        let toml = write_toml(&o);
        assert!(
            toml.contains("my-var = \"val\""),
            "keys with dashes should stay bare, got: {toml}"
        );
    }

    #[test]
    fn env_key_with_spaces_is_quoted() {
        let mut o = empty_output();
        o.env.insert("my key".into(), "val".into());
        let toml = write_toml(&o);
        assert!(
            toml.contains("\"my key\" = \"val\""),
            "keys with spaces must be quoted, got: {toml}"
        );
    }

    #[test]
    fn toml_key_unit_tests() {
        assert_eq!(toml_key("CC"), "CC");
        assert_eq!(toml_key("my-var"), "my-var");
        assert_eq!(toml_key("my_var"), "my_var");
        assert_eq!(toml_key("abc123"), "abc123");
        assert_eq!(toml_key(".DEFAULT_GOAL"), "\".DEFAULT_GOAL\"");
        assert_eq!(toml_key("my.var"), "\"my.var\"");
        assert_eq!(toml_key("my key"), "\"my key\"");
        assert_eq!(toml_key(""), "\"\"");
    }
}
