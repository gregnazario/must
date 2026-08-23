use must_import::import;

#[test]
fn fixture_simple_rules() {
    let input = include_str!("fixtures/simple-rules.mk");
    let result = import(input);
    assert!(
        result.toml.contains("[recipe.all]") || result.toml.contains("[recipe.\"all\"]"),
        "should have recipe 'all'"
    );
    assert!(
        result.toml.contains("[recipe.build]") || result.toml.contains("[recipe.\"build\"]"),
        "should have recipe 'build'"
    );
    assert!(
        result.toml.contains("[recipe.test]") || result.toml.contains("[recipe.\"test\"]"),
        "should have recipe 'test'"
    );
    assert_eq!(result.todo_count, 0, "simple rules have no TODOs");
}

#[test]
fn fixture_vars_only() {
    let input = include_str!("fixtures/vars-only.mk");
    let result = import(input);
    assert!(
        result.toml.contains("[env]"),
        "should have env section, got: {}",
        result.toml
    );
    assert!(!result.toml.contains("[env.global]"));
    assert!(result.toml.contains("CC ="), "should have CC var");
    assert_eq!(result.translated_count, 3, "3 env vars translated");
}

#[test]
fn fixture_phony() {
    let input = include_str!("fixtures/phony.mk");
    let result = import(input);
    assert!(
        result.toml.contains("phony = true"),
        "phony targets should be marked"
    );
    assert!(result.toml.contains("[recipe.clean]") || result.toml.contains("[recipe.\"clean\"]"));
}

#[test]
fn fixture_shell_substitution_passes_through() {
    let input = include_str!("fixtures/shell-substitution.mk");
    let result = import(input);
    // GIT_HASH should appear as an env var or in the TOML output
    assert!(
        result.toml.contains("GIT_HASH") || result.translated_count > 0,
        "shell substitution result should appear in output"
    );
}

#[test]
fn fixture_pattern_rules_become_todos() {
    let input = include_str!("fixtures/pattern-rules.mk");
    let result = import(input);
    assert!(
        result.todo_count >= 1,
        "pattern rules should become TODOs, got {}",
        result.todo_count
    );
    assert!(
        result.report.contains("Pattern rule") || result.report.contains("pattern"),
        "report should mention pattern rules"
    );
}

#[test]
fn fixture_includes_become_todos() {
    let input = include_str!("fixtures/includes.mk");
    let result = import(input);
    assert!(
        result.todo_count >= 2,
        "two include directives should become TODOs, got {}",
        result.todo_count
    );
    assert!(
        result.report.contains("Include") || result.report.contains("include"),
        "report should mention includes"
    );
}

#[test]
fn duplicate_rules_dedupe_and_deps_filter_end_to_end() {
    let input = "CC = gcc\n\
                 OBJS = main.o util.o\n\
                 \n\
                 all: build\n\
                 \n\
                 app: $(OBJS)\n\
                 \tgcc -o app main.c util.c\n\
                 \n\
                 app: test\n\
                 \techo rebuilt\n\
                 \n\
                 build:\n\
                 \tcargo build\n\
                 \n\
                 test:\n\
                 \tcargo test\n";
    let result = import(input);
    assert!(
        result.toml.contains("\n[env]\n"),
        "env must be a flat [env] table, got: {}",
        result.toml
    );
    assert_eq!(
        result.toml.matches("[recipe.\"app\"]").count(),
        1,
        "duplicate 'app' rules must emit one recipe table, got: {}",
        result.toml
    );
    assert!(
        result.toml.contains("deps = [\"test\"]"),
        "$(OBJS) is not a target and must be filtered from deps, got: {}",
        result.toml
    );
    assert!(
        result.toml.contains("echo rebuilt"),
        "last rule with recipe lines must win the script"
    );
    assert!(
        result.toml.contains("[recipe.\"all\"]\ntype = \"shell\"\ndeps = [\"build\"]\nphony = true\nscript = \"\"\"\ntrue\n\"\"\""),
        "aggregate 'all' target must become a phony no-op, got: {}",
        result.toml
    );
}

#[test]
fn blank_line_inside_recipe_kept_end_to_end() {
    let input = "build:\n\techo first\n\n\techo second\n";
    let result = import(input);
    assert!(
        result.toml.contains("echo first\necho second"),
        "script lines separated by a blank line must both be kept, got: {}",
        result.toml
    );
}

#[test]
fn make_recipe_prefixes_stripped_end_to_end() {
    let input = "all:\n\t@echo done\n\nbuild:\n\t-rm -rf out\n\t@-mkdir out\n\t+make -C sub all\n";
    let result = import(input);
    assert!(result.toml.contains("echo done"), "got: {}", result.toml);
    assert!(result.toml.contains("rm -rf out"), "got: {}", result.toml);
    assert!(result.toml.contains("mkdir out"), "got: {}", result.toml);
    assert!(
        result.toml.contains("make -C sub all"),
        "got: {}",
        result.toml
    );
    assert!(
        !result.toml.contains("@echo"),
        "`@` prefix must be stripped from make recipe lines, got: {}",
        result.toml
    );
    assert!(
        !result.toml.contains("\n-rm"),
        "`-` prefix must be stripped from make recipe lines, got: {}",
        result.toml
    );
    assert!(
        !result.toml.contains("@-mkdir"),
        "combined `@-` prefixes must be stripped, got: {}",
        result.toml
    );
    assert!(
        !result.toml.contains("+make"),
        "`+` prefix must be stripped from make recipe lines, got: {}",
        result.toml
    );
}
