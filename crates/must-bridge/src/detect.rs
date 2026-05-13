use must_config::schema::{CacheMode, Config, EnvMap, Project, Recipe, RecipeType};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeTool {
    Make,
    Npm,
    Gradle,
    Maven,
    Rake,
    Invoke,
    Cmake,
    CargoMake,
    Ant,
    Just,
    Bazel,
    Buck2,
    Pants,
    Meson,
    Yarn,
    Pnpm,
    Bun,
    Sbt,
    Gulp,
    Nx,
}

impl BridgeTool {
    pub fn indicator_files(&self) -> &[&str] {
        match self {
            BridgeTool::Make => &["Makefile", "GNUmakefile", "makefile"],
            BridgeTool::Npm => &["package.json"],
            BridgeTool::Gradle => &["build.gradle", "build.gradle.kts"],
            BridgeTool::Maven => &["pom.xml"],
            BridgeTool::Rake => &["Rakefile"],
            BridgeTool::Invoke => &["tasks.py"],
            BridgeTool::Cmake => &["CMakeLists.txt"],
            BridgeTool::CargoMake => &["Makefile.toml"],
            BridgeTool::Ant => &["build.xml"],
            BridgeTool::Just => &["justfile", "Justfile"],
            BridgeTool::Bazel => &["WORKSPACE", "WORKSPACE.bazel", "MODULE.bazel"],
            BridgeTool::Buck2 => &["BUCK"],
            BridgeTool::Pants => &["pants.toml"],
            BridgeTool::Meson => &["meson.build"],
            BridgeTool::Yarn => &["yarn.lock"],
            BridgeTool::Pnpm => &["pnpm-lock.yaml"],
            BridgeTool::Bun => &["bun.lockb", "bun.lock"],
            BridgeTool::Sbt => &["build.sbt"],
            BridgeTool::Gulp => &["gulpfile.js", "gulpfile.mjs", "gulpfile.ts"],
            BridgeTool::Nx => &["nx.json"],
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            BridgeTool::Make => "make",
            BridgeTool::Npm => "npm",
            BridgeTool::Gradle => "gradle",
            BridgeTool::Maven => "mvn",
            BridgeTool::Rake => "rake",
            BridgeTool::Invoke => "invoke",
            BridgeTool::Cmake => "cmake",
            BridgeTool::CargoMake => "cargo-make",
            BridgeTool::Ant => "ant",
            BridgeTool::Just => "just",
            BridgeTool::Bazel => "bazel",
            BridgeTool::Buck2 => "buck2",
            BridgeTool::Pants => "pants",
            BridgeTool::Meson => "meson",
            BridgeTool::Yarn => "yarn",
            BridgeTool::Pnpm => "pnpm",
            BridgeTool::Bun => "bun",
            BridgeTool::Sbt => "sbt",
            BridgeTool::Gulp => "gulp",
            BridgeTool::Nx => "nx",
        }
    }

    pub fn build_command(&self, target: &str) -> String {
        match self {
            BridgeTool::Make => {
                if target.is_empty() || target == "build" {
                    "make".to_string()
                } else {
                    format!("make {target}")
                }
            }
            BridgeTool::Npm => format!("npm run {target}"),
            BridgeTool::Gradle => format!("gradle {target}"),
            BridgeTool::Maven => format!("mvn {target}"),
            BridgeTool::Rake => format!("rake {target}"),
            BridgeTool::Invoke => format!("invoke {target}"),
            BridgeTool::Cmake => {
                if target == "build" || target.is_empty() {
                    "cmake --build build".to_string()
                } else if target == "test" {
                    "ctest --test-dir build".to_string()
                } else {
                    format!("cmake --build build --target {target}")
                }
            }
            BridgeTool::CargoMake => format!("cargo make {target}"),
            BridgeTool::Ant => {
                if target.is_empty() || target == "build" {
                    "ant".to_string()
                } else {
                    format!("ant {target}")
                }
            }
            BridgeTool::Just => format!("just {target}"),
            BridgeTool::Bazel => format!("bazel {target}"),
            BridgeTool::Buck2 => format!("buck2 {target}"),
            BridgeTool::Pants => format!("pants {target}"),
            BridgeTool::Meson => {
                if target == "build" || target.is_empty() {
                    "meson compile -C builddir".to_string()
                } else if target == "test" {
                    "meson test -C builddir".to_string()
                } else {
                    format!("meson compile -C builddir {target}")
                }
            }
            BridgeTool::Yarn => format!("yarn {target}"),
            BridgeTool::Pnpm => format!("pnpm {target}"),
            BridgeTool::Bun => format!("bun run {target}"),
            BridgeTool::Sbt => format!("sbt {target}"),
            BridgeTool::Gulp => format!("gulp {target}"),
            BridgeTool::Nx => format!("nx {target}"),
        }
    }

    pub fn default_recipes(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            BridgeTool::Make => vec![
                ("build", "build"),
                ("test", "test"),
                ("clean", "clean"),
                ("lint", "lint"),
                ("fmt", "fmt"),
            ],
            BridgeTool::Npm => vec![
                ("build", "build"),
                ("test", "test"),
                ("lint", "lint"),
                ("fmt", "format"),
            ],
            BridgeTool::Gradle => vec![
                ("build", "build"),
                ("test", "test"),
                ("clean", "clean"),
                ("lint", "check"),
                ("fmt", "spotlessApply"),
            ],
            BridgeTool::Maven => vec![
                ("build", "compile"),
                ("test", "test"),
                ("clean", "clean"),
                ("lint", "verify"),
                ("fmt", "fmt:format"),
            ],
            BridgeTool::Rake => vec![
                ("build", "build"),
                ("test", "test"),
                ("clean", "clean"),
                ("lint", "lint"),
                ("fmt", "format"),
            ],
            BridgeTool::Invoke => vec![
                ("build", "build"),
                ("test", "test"),
                ("lint", "lint"),
                ("fmt", "format"),
            ],
            BridgeTool::Cmake => vec![("build", "build"), ("test", "test"), ("clean", "clean")],
            BridgeTool::CargoMake => vec![
                ("build", "build"),
                ("test", "test"),
                ("clean", "clean"),
                ("lint", "lint"),
                ("fmt", "format"),
            ],
            BridgeTool::Ant => vec![("build", "build"), ("test", "test"), ("clean", "clean")],
            BridgeTool::Just => vec![
                ("build", "build"),
                ("test", "test"),
                ("clean", "clean"),
                ("lint", "lint"),
                ("fmt", "fmt"),
            ],
            BridgeTool::Bazel => vec![
                ("build", "build //..."),
                ("test", "test //..."),
                ("clean", "clean"),
                ("lint", "build //... --aspects-workers=4"),
            ],
            BridgeTool::Buck2 => vec![
                ("build", "build //..."),
                ("test", "test //..."),
                ("clean", "clean"),
            ],
            BridgeTool::Pants => vec![
                ("build", "package ::"),
                ("test", "test ::"),
                ("lint", "lint ::"),
                ("fmt", "fmt ::"),
                ("clean", "clean-all"),
            ],
            BridgeTool::Meson => vec![("build", "build"), ("test", "test"), ("clean", "clean")],
            BridgeTool::Yarn => vec![
                ("build", "build"),
                ("test", "test"),
                ("lint", "lint"),
                ("fmt", "format"),
            ],
            BridgeTool::Pnpm => vec![
                ("build", "build"),
                ("test", "test"),
                ("lint", "lint"),
                ("fmt", "format"),
            ],
            BridgeTool::Bun => vec![
                ("build", "build"),
                ("test", "test"),
                ("lint", "lint"),
                ("fmt", "format"),
            ],
            BridgeTool::Sbt => vec![("build", "compile"), ("test", "test"), ("clean", "clean")],
            BridgeTool::Gulp => vec![("build", "build"), ("test", "test"), ("clean", "clean")],
            BridgeTool::Nx => vec![
                ("build", "run-many --target=build --all"),
                ("test", "run-many --target=test --all"),
                ("lint", "run-many --target=lint --all"),
                ("fmt", "format"),
            ],
        }
    }
}

const ALL_TOOLS: &[BridgeTool] = &[
    BridgeTool::Make,
    BridgeTool::Npm,
    BridgeTool::Gradle,
    BridgeTool::Maven,
    BridgeTool::Rake,
    BridgeTool::Invoke,
    BridgeTool::Cmake,
    BridgeTool::CargoMake,
    BridgeTool::Ant,
    BridgeTool::Just,
    BridgeTool::Bazel,
    BridgeTool::Buck2,
    BridgeTool::Pants,
    BridgeTool::Meson,
    BridgeTool::Yarn,
    BridgeTool::Pnpm,
    BridgeTool::Bun,
    BridgeTool::Sbt,
    BridgeTool::Gulp,
    BridgeTool::Nx,
];

pub fn detect_bridges(project_root: &Path) -> Vec<BridgeTool> {
    let mut found = Vec::new();
    for tool in ALL_TOOLS {
        for filename in tool.indicator_files() {
            if project_root.join(filename).exists() {
                found.push(*tool);
                break;
            }
        }
    }
    found
}

pub fn auto_config(project_root: &Path) -> Option<Config> {
    let bridges = detect_bridges(project_root);
    if bridges.is_empty() {
        return None;
    }

    let project_name = project_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "auto".to_string());

    let mut recipes = HashMap::new();

    for tool in &bridges {
        for (recipe_name, target) in tool.default_recipes() {
            if recipes.contains_key(recipe_name) && bridges.len() > 1 {
                let prefixed = format!("{}-{}", tool.name(), recipe_name);
                recipes.insert(prefixed, make_bridge_recipe(tool, target));
            }
            recipes
                .entry(recipe_name.to_string())
                .or_insert_with(|| make_bridge_recipe(tool, target));
        }
    }

    Some(Config {
        project: Project {
            name: project_name,
            version: None,
            include: Vec::new(),
        },
        env: EnvMap {
            global: HashMap::new(),
        },
        targets: HashMap::new(),
        recipe: recipes,
    })
}

fn make_bridge_recipe(tool: &BridgeTool, target: &str) -> Recipe {
    Recipe {
        recipe_type: RecipeType::Bridge,
        script: Some(tool.build_command(target)),
        script_win: None,
        scripts: HashMap::new(),
        deps: vec![],
        inputs: vec![],
        outputs: vec![],
        cache: Some(CacheMode::None),
        phony: true,
        env: HashMap::new(),
        cross: HashMap::new(),
        package: Some(tool.name().to_string()),
        features: vec![],
        ldflags: None,
        sources: vec![],
        includes: vec![],
        link_libs: vec![],
        image: None,
        dockerfile: None,
        build_args: vec![],
        plugin: None,
        url: None,
        sha256: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detect_makefile() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("Makefile"), "all:\n\techo hi\n").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Make));
    }

    #[test]
    fn detect_package_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(
            tmp.path().join("package.json"),
            r#"{"scripts": {"build": "tsc"}}"#,
        )
        .unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Npm));
    }

    #[test]
    fn detect_gradle() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("build.gradle"), "").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Gradle));
    }

    #[test]
    fn detect_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.is_empty());
    }

    #[test]
    fn detect_multiple_tools() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("Makefile"), "all:\n\techo\n").unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert_eq!(bridges.len(), 2);
    }

    #[test]
    fn auto_config_returns_none_when_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(auto_config(tmp.path()).is_none());
    }

    #[test]
    fn auto_config_generates_recipes() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("Makefile"), "all:\n\techo\n").unwrap();
        let config = auto_config(tmp.path()).unwrap();
        assert!(config.recipe.contains_key("build"));
        assert!(config.recipe.contains_key("test"));
        assert_eq!(config.recipe["build"].recipe_type, RecipeType::Bridge);
    }

    #[test]
    fn auto_config_multi_tool_prefixes() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("Makefile"), "all:\n\techo\n").unwrap();
        fs::write(tmp.path().join("package.json"), "{}").unwrap();
        let config = auto_config(tmp.path()).unwrap();
        assert!(config.recipe.contains_key("build"));
        assert!(config.recipe.contains_key("npm-build"));
        assert!(config.recipe.contains_key("npm-test"));
    }

    #[test]
    fn bridge_tool_names() {
        assert_eq!(BridgeTool::Make.name(), "make");
        assert_eq!(BridgeTool::Npm.name(), "npm");
        assert_eq!(BridgeTool::Gradle.name(), "gradle");
        assert_eq!(BridgeTool::Maven.name(), "mvn");
        assert_eq!(BridgeTool::Rake.name(), "rake");
        assert_eq!(BridgeTool::Invoke.name(), "invoke");
        assert_eq!(BridgeTool::Cmake.name(), "cmake");
        assert_eq!(BridgeTool::CargoMake.name(), "cargo-make");
        assert_eq!(BridgeTool::Ant.name(), "ant");
        assert_eq!(BridgeTool::Just.name(), "just");
        assert_eq!(BridgeTool::Bazel.name(), "bazel");
        assert_eq!(BridgeTool::Buck2.name(), "buck2");
        assert_eq!(BridgeTool::Pants.name(), "pants");
        assert_eq!(BridgeTool::Meson.name(), "meson");
        assert_eq!(BridgeTool::Yarn.name(), "yarn");
        assert_eq!(BridgeTool::Pnpm.name(), "pnpm");
        assert_eq!(BridgeTool::Bun.name(), "bun");
        assert_eq!(BridgeTool::Sbt.name(), "sbt");
        assert_eq!(BridgeTool::Gulp.name(), "gulp");
        assert_eq!(BridgeTool::Nx.name(), "nx");
    }

    #[test]
    fn build_command_make() {
        assert_eq!(BridgeTool::Make.build_command("build"), "make");
        assert_eq!(BridgeTool::Make.build_command("test"), "make test");
    }

    #[test]
    fn build_command_npm() {
        assert_eq!(BridgeTool::Npm.build_command("build"), "npm run build");
        assert_eq!(BridgeTool::Npm.build_command("test"), "npm run test");
    }

    #[test]
    fn build_command_cmake() {
        assert_eq!(
            BridgeTool::Cmake.build_command("build"),
            "cmake --build build"
        );
        assert_eq!(
            BridgeTool::Cmake.build_command("test"),
            "ctest --test-dir build"
        );
    }

    #[test]
    fn auto_config_project_name_from_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("Makefile"), "all:\n\techo\n").unwrap();
        let config = auto_config(tmp.path()).unwrap();
        assert!(!config.project.name.is_empty());
    }

    #[test]
    fn auto_config_bridge_recipe_has_tool_in_package() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("Makefile"), "all:\n\techo\n").unwrap();
        let config = auto_config(tmp.path()).unwrap();
        assert_eq!(config.recipe["build"].package.as_deref(), Some("make"));
    }

    #[test]
    fn detect_justfile() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("justfile"), "build:\n    cargo build\n").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Just));
    }

    #[test]
    fn detect_rakefile() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("Rakefile"), "task :build do\nend\n").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Rake));
    }

    #[test]
    fn detect_maven() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("pom.xml"), "<project></project>").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Maven));
    }

    #[test]
    fn detect_ant() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("build.xml"), "<project></project>").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Ant));
    }

    #[test]
    fn detect_invoke() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("tasks.py"), "from invoke import task\n").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Invoke));
    }

    #[test]
    fn detect_bazel() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("WORKSPACE"), "").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Bazel));
    }

    #[test]
    fn detect_bazel_module() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("MODULE.bazel"), "").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Bazel));
    }

    #[test]
    fn detect_buck2() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("BUCK"), "").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Buck2));
    }

    #[test]
    fn detect_pants() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("pants.toml"), "").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Pants));
    }

    #[test]
    fn detect_meson() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("meson.build"), "").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Meson));
    }

    #[test]
    fn detect_yarn() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("yarn.lock"), "").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Yarn));
    }

    #[test]
    fn detect_pnpm() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("pnpm-lock.yaml"), "").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Pnpm));
    }

    #[test]
    fn detect_bun() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("bun.lockb"), "").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Bun));
    }

    #[test]
    fn detect_sbt() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("build.sbt"), "").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Sbt));
    }

    #[test]
    fn detect_gulp() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("gulpfile.js"), "").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Gulp));
    }

    #[test]
    fn detect_nx() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("nx.json"), "{}").unwrap();
        let bridges = detect_bridges(tmp.path());
        assert!(bridges.contains(&BridgeTool::Nx));
    }

    #[test]
    fn build_command_bazel() {
        assert_eq!(BridgeTool::Bazel.build_command("build"), "bazel build");
        assert_eq!(
            BridgeTool::Bazel.build_command("test //..."),
            "bazel test //..."
        );
    }

    #[test]
    fn build_command_meson() {
        assert_eq!(
            BridgeTool::Meson.build_command("build"),
            "meson compile -C builddir"
        );
        assert_eq!(
            BridgeTool::Meson.build_command("test"),
            "meson test -C builddir"
        );
    }

    #[test]
    fn build_command_yarn() {
        assert_eq!(BridgeTool::Yarn.build_command("build"), "yarn build");
        assert_eq!(BridgeTool::Yarn.build_command("test"), "yarn test");
    }

    #[test]
    fn build_command_bun() {
        assert_eq!(BridgeTool::Bun.build_command("build"), "bun run build");
    }

    #[test]
    fn build_command_nx() {
        assert_eq!(
            BridgeTool::Nx.build_command("run-many --target=build --all"),
            "nx run-many --target=build --all"
        );
    }
}
