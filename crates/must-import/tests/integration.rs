use must_import::import;

#[test]
fn fixture_simple_rules() {
    let input = include_str!("fixtures/simple-rules.mk");
    let result = import(input);
    assert!(result.toml.contains("[recipe.all]") || result.toml.contains("[recipe.\"all\"]"),
        "should have recipe 'all'");
    assert!(result.toml.contains("[recipe.build]") || result.toml.contains("[recipe.\"build\"]"),
        "should have recipe 'build'");
    assert!(result.toml.contains("[recipe.test]") || result.toml.contains("[recipe.\"test\"]"),
        "should have recipe 'test'");
    assert_eq!(result.todo_count, 0, "simple rules have no TODOs");
}

#[test]
fn fixture_vars_only() {
    let input = include_str!("fixtures/vars-only.mk");
    let result = import(input);
    assert!(result.toml.contains("[env.global]"), "should have env section");
    assert!(result.toml.contains("CC ="), "should have CC var");
    assert_eq!(result.translated_count, 3, "3 env vars translated");
}

#[test]
fn fixture_phony() {
    let input = include_str!("fixtures/phony.mk");
    let result = import(input);
    assert!(result.toml.contains("phony = true"), "phony targets should be marked");
    assert!(result.toml.contains("[recipe.clean]") || result.toml.contains("[recipe.\"clean\"]"));
}

#[test]
fn fixture_shell_substitution_passes_through() {
    let input = include_str!("fixtures/shell-substitution.mk");
    let result = import(input);
    // GIT_HASH should appear as an env var or in the TOML output
    assert!(result.toml.contains("GIT_HASH") || result.translated_count > 0,
        "shell substitution result should appear in output");
}

#[test]
fn fixture_pattern_rules_become_todos() {
    let input = include_str!("fixtures/pattern-rules.mk");
    let result = import(input);
    assert!(result.todo_count >= 1, "pattern rules should become TODOs, got {}", result.todo_count);
    assert!(result.report.contains("Pattern rule") || result.report.contains("pattern"),
        "report should mention pattern rules");
}

#[test]
fn fixture_includes_become_todos() {
    let input = include_str!("fixtures/includes.mk");
    let result = import(input);
    assert!(result.todo_count >= 2, "two include directives should become TODOs, got {}", result.todo_count);
    assert!(result.report.contains("Include") || result.report.contains("include"),
        "report should mention includes");
}
