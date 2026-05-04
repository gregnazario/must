use clap::{Parser, Subcommand};
use must_config::load_config;
use must_config::schema::{CacheMode, Config, RecipeType};
use must_core::{BuildContext, CacheStrategy, Error};
use must_engine::{Engine, compose_env};
use must_graph::Dag;
use must_recipe_cc::{CBinRecipe, CLibRecipe};
use must_recipe_go::{GoBinRecipe, GoTestRecipe};
use must_recipe_rust::{RustBinRecipe, RustLibRecipe, RustTestRecipe};
use must_recipe_shell::ShellRecipe;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

#[derive(Parser)]
#[command(name = "must", about = "Polyglot build orchestrator", version)]
struct Cli {
    /// Path to Mustfile.toml (default: search upward from cwd)
    #[arg(long, global = true)]
    file: Option<PathBuf>,

    /// Cross-compile targets (can specify multiple, or a [targets] group name)
    #[arg(long, global = true)]
    target: Vec<String>,

    /// Apply [env.<profile>] overrides
    #[arg(long, global = true, default_value = "default")]
    profile: String,

    /// Parallelism (default = num_cpus)
    #[arg(short = 'j', global = true)]
    parallelism: Option<usize>,

    /// Plan without executing
    #[arg(long, global = true)]
    dry_run: bool,

    /// Cancel in-flight recipes on first failure
    #[arg(long, global = true)]
    fail_fast: bool,

    /// Verbosity (-v, -vv, -vvv)
    #[arg(short = 'v', action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the default 'build' recipe
    Build {
        /// Specific recipes to build (default: "build")
        recipes: Vec<String>,
    },
    /// Run one or more recipes (alias for build)
    Run {
        /// Recipes to run (default: "build")
        recipes: Vec<String>,
    },
    /// Run the default 'test' recipe
    Test {
        /// Specific recipes to test (default: "test")
        recipes: Vec<String>,
    },
    /// List all recipes
    List,
    /// Clean outputs
    Clean {
        /// Also clean the cache
        #[arg(long)]
        cache: bool,
    },
    /// Show why a recipe will or won't rebuild
    Explain {
        /// Recipe name to explain
        recipe: String,
    },
    /// Import a Makefile and produce a Mustfile.toml
    Import {
        /// Path to the Makefile to import
        #[arg(long, default_value = "Makefile")]
        makefile: PathBuf,

        /// Output path for the generated Mustfile.toml
        #[arg(long, default_value = "Mustfile.toml")]
        out: PathBuf,
    },
    /// Create a new Mustfile.toml in the current directory
    Init {
        /// Project name (default: current directory name)
        #[arg(long)]
        name: Option<String>,
    },
    /// Check environment health (toolchains, container runtime, cache)
    Doctor,
    /// Print the recipe dependency graph
    Graph {
        /// Output format: text, dot, or mermaid
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// Run a recipe by name directly (e.g. `must lint` → `must run lint`)
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Set up tracing
    let level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level)),
        )
        .init();

    if let Err(e) = run(cli).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> must_core::Result<()> {
    // Handle import before loading config — it doesn't need a Mustfile.toml
    if let Commands::Import { makefile, out } = cli.command {
        let input = std::fs::read_to_string(&makefile).map_err(must_core::Error::Io)?;
        let result = must_import::import(&input);

        std::fs::write(&out, &result.toml).map_err(must_core::Error::Io)?;

        let report_path = out.with_file_name("MUSTFILE_IMPORT_REPORT.md");
        std::fs::write(&report_path, &result.report).map_err(must_core::Error::Io)?;

        println!("Imported {} → {}", makefile.display(), out.display());
        println!(
            "  {} translated, {} TODO, {} skipped",
            result.translated_count, result.todo_count, result.skipped_count
        );
        println!("Report: {}", report_path.display());
        return Ok(());
    }

    if matches!(cli.command, Commands::Doctor) {
        run_doctor();
        return Ok(());
    }

    if let Commands::Init { name } = &cli.command {
        let out = cli
            .file
            .clone()
            .unwrap_or_else(|| PathBuf::from("Mustfile.toml"));
        run_init(&out, name.as_deref())?;
        return Ok(());
    }

    // Find and load Mustfile.toml
    let mustfile_path = cli
        .file
        .unwrap_or_else(|| find_mustfile().unwrap_or_else(|| PathBuf::from("Mustfile.toml")));

    let config = load_config(&mustfile_path)?;

    let targets = resolve_targets(&cli.target, &config);

    match cli.command {
        Commands::List => {
            println!("{:<20} {:<12} DEPS", "NAME", "TYPE");
            println!("{}", "-".repeat(60));
            let mut names: Vec<&String> = config.recipe.keys().collect();
            names.sort();
            for name in names {
                let recipe = &config.recipe[name];
                let type_str = format!("{:?}", recipe.recipe_type).to_lowercase();
                let deps = if recipe.deps.is_empty() {
                    String::new()
                } else {
                    recipe.deps.join(", ")
                };
                println!("{:<20} {:<12} {}", name, type_str, deps);
            }
        }
        Commands::Clean { cache } => {
            if cache {
                let cache_dir = mustfile_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(".mustfile")
                    .join("cache");
                if cache_dir.exists() {
                    std::fs::remove_dir_all(&cache_dir).map_err(must_core::Error::Io)?;
                    println!("cleaned cache at {}", cache_dir.display());
                }
            }
        }
        Commands::Build { recipes } | Commands::Run { recipes } => {
            let target_recipes = if recipes.is_empty() {
                vec!["build".to_string()]
            } else {
                recipes
            };
            execute_recipes(
                &config,
                &mustfile_path,
                RunOpts {
                    profile: &cli.profile,
                    parallelism: cli.parallelism,
                    dry_run: cli.dry_run,
                    fail_fast: cli.fail_fast,
                    target_recipes,
                    targets: &targets,
                },
            )
            .await?;
        }
        Commands::Test { recipes } => {
            let target_recipes = if recipes.is_empty() {
                vec!["test".to_string()]
            } else {
                recipes
            };
            execute_recipes(
                &config,
                &mustfile_path,
                RunOpts {
                    profile: &cli.profile,
                    parallelism: cli.parallelism,
                    dry_run: cli.dry_run,
                    fail_fast: cli.fail_fast,
                    target_recipes,
                    targets: &targets,
                },
            )
            .await?;
        }
        Commands::Explain { recipe } => {
            explain_recipe(&config, &mustfile_path, &cli.profile, &recipe)?;
        }
        Commands::Graph { format } => {
            print_graph(&config, &format)?;
        }
        Commands::Import { .. } => {
            // handled before config loading above
            unreachable!()
        }
        Commands::External(args) => {
            execute_recipes(
                &config,
                &mustfile_path,
                RunOpts {
                    profile: &cli.profile,
                    parallelism: cli.parallelism,
                    dry_run: cli.dry_run,
                    fail_fast: cli.fail_fast,
                    target_recipes: args,
                    targets: &targets,
                },
            )
            .await?;
        }
        Commands::Doctor | Commands::Init { .. } => {
            // handled before config loading above
            unreachable!()
        }
    }

    Ok(())
}

fn run_init(out: &Path, name: Option<&str>) -> must_core::Result<()> {
    if out.exists() {
        return Err(must_core::Error::Config {
            path: out.to_owned(),
            message:
                "file already exists; delete it first or use --file to choose a different path"
                    .to_string(),
        });
    }

    let project_name = name
        .map(|s| s.to_string())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        })
        .unwrap_or_else(|| "my-project".to_string());

    let contents = format!(
        r#"[project]
name = "{project_name}"

[recipe.build]
type = "shell"
script = "echo 'Building {project_name}'"
phony = true

[recipe.test]
type = "shell"
script = "echo 'Testing {project_name}'"
deps = ["build"]
phony = true
"#
    );

    std::fs::write(out, contents).map_err(must_core::Error::Io)?;
    println!("Created {}", out.display());
    Ok(())
}

struct RunOpts<'a> {
    profile: &'a str,
    parallelism: Option<usize>,
    dry_run: bool,
    fail_fast: bool,
    target_recipes: Vec<String>,
    targets: &'a [String],
}

async fn execute_recipes(
    config: &Config,
    mustfile_path: &Path,
    opts: RunOpts<'_>,
) -> must_core::Result<()> {
    let RunOpts {
        profile,
        parallelism,
        dry_run,
        fail_fast,
        target_recipes,
        targets,
    } = opts;
    let mustfile_abs = mustfile_path
        .canonicalize()
        .unwrap_or_else(|_| mustfile_path.to_owned());
    let project_root = mustfile_abs
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_owned();

    // Build the full DAG from all recipes
    let dep_map: HashMap<String, Vec<String>> = config
        .recipe
        .iter()
        .map(|(name, r)| (name.clone(), r.deps.clone()))
        .collect();
    let dag = Dag::new(dep_map);

    // Determine the reachable set from requested target recipes
    let mut reachable: std::collections::HashSet<String> = std::collections::HashSet::new();
    for target in &target_recipes {
        if !config.recipe.contains_key(target.as_str()) {
            return Err(Error::UnknownRecipe {
                name: target.clone(),
            });
        }
        for name in dag.reachable_from(target)? {
            reachable.insert(name);
        }
    }

    // Compose env for each recipe and build recipe objects
    let mut recipe_map: HashMap<String, Arc<dyn must_core::Recipe>> = HashMap::new();
    for (name, recipe_cfg) in &config.recipe {
        if !reachable.contains(name) {
            continue;
        }
        let env = compose_env(config, name, profile, &HashMap::new());
        match recipe_cfg.recipe_type {
            RecipeType::Shell => {
                let mut shell =
                    ShellRecipe::new(name.clone(), recipe_cfg.script.clone().unwrap_or_default());
                shell.deps = recipe_cfg.deps.clone();
                shell.inputs = recipe_cfg.inputs.clone();
                shell.outputs = recipe_cfg.outputs.clone();
                shell.env = env;
                if let Some(CacheMode::Hash) = &recipe_cfg.cache {
                    shell.cache = CacheStrategy::Hash;
                }
                recipe_map.insert(name.clone(), Arc::new(shell));
            }
            RecipeType::RustBin => {
                let mut r = RustBinRecipe::new(
                    name.clone(),
                    recipe_cfg.package.clone().unwrap_or_else(|| name.clone()),
                );
                r.deps = recipe_cfg.deps.clone();
                r.features = recipe_cfg.features.clone();
                r.release = profile == "release";
                r.env = env;
                recipe_map.insert(name.clone(), Arc::new(r));
            }
            RecipeType::RustLib => {
                let mut r = RustLibRecipe::new(
                    name.clone(),
                    recipe_cfg.package.clone().unwrap_or_else(|| name.clone()),
                );
                r.deps = recipe_cfg.deps.clone();
                r.features = recipe_cfg.features.clone();
                r.release = profile == "release";
                r.env = env;
                recipe_map.insert(name.clone(), Arc::new(r));
            }
            RecipeType::RustTest => {
                let mut r = RustTestRecipe::new(
                    name.clone(),
                    recipe_cfg.package.clone().unwrap_or_else(|| name.clone()),
                );
                r.deps = recipe_cfg.deps.clone();
                r.env = env;
                recipe_map.insert(name.clone(), Arc::new(r));
            }
            RecipeType::GoBin => {
                let r = GoBinRecipe {
                    name: name.clone(),
                    package: recipe_cfg
                        .package
                        .clone()
                        .unwrap_or_else(|| ".".to_string()),
                    deps: recipe_cfg.deps.clone(),
                    ldflags: recipe_cfg.ldflags.clone(),
                    build_tags: Vec::new(),
                    env,
                };
                recipe_map.insert(name.clone(), Arc::new(r));
            }
            RecipeType::GoTest => {
                let r = GoTestRecipe {
                    name: name.clone(),
                    package: recipe_cfg
                        .package
                        .clone()
                        .unwrap_or_else(|| "./...".to_string()),
                    deps: recipe_cfg.deps.clone(),
                    env,
                };
                recipe_map.insert(name.clone(), Arc::new(r));
            }
            RecipeType::CBin => {
                let r = CBinRecipe {
                    name: name.clone(),
                    deps: recipe_cfg.deps.clone(),
                    sources: recipe_cfg.sources.clone(),
                    includes: recipe_cfg.includes.clone(),
                    link_libs: recipe_cfg.link_libs.clone(),
                    cflags: Vec::new(),
                    env,
                    cross: recipe_cfg.cross.clone(),
                };
                recipe_map.insert(name.clone(), Arc::new(r));
            }
            RecipeType::CLib => {
                let r = CLibRecipe {
                    name: name.clone(),
                    deps: recipe_cfg.deps.clone(),
                    sources: recipe_cfg.sources.clone(),
                    includes: recipe_cfg.includes.clone(),
                    link_libs: recipe_cfg.link_libs.clone(),
                    cflags: Vec::new(),
                    env,
                    cross: recipe_cfg.cross.clone(),
                    static_lib: true, // default to static; could be from config in future
                };
                recipe_map.insert(name.clone(), Arc::new(r));
            }
        }
    }

    // Restrict DAG to the reachable subset
    let sub_dep_map: HashMap<String, Vec<String>> = config
        .recipe
        .iter()
        .filter(|(name, _)| reachable.contains(*name))
        .map(|(name, r)| {
            let filtered_deps: Vec<String> = r
                .deps
                .iter()
                .filter(|d| reachable.contains(*d))
                .cloned()
                .collect();
            (name.clone(), filtered_deps)
        })
        .collect();
    let sub_dag = Dag::new(sub_dep_map);

    let j = parallelism.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    });

    // Execute once per target
    for target in targets {
        if targets.len() > 1 {
            println!("\n[target: {target}]");
        }

        let mut ctx = BuildContext::new(project_root.clone());
        ctx.profile = profile.to_string();
        ctx.target = target.clone();
        ctx.dry_run = dry_run;
        ctx.parallelism = j;

        let engine = Engine::new(j, fail_fast);
        let report = engine.execute(&sub_dag, &recipe_map, &ctx).await?;

        // Print summary
        println!(
            "\n{} built, {} cached, {} failed — {}ms",
            report.built(),
            report.cached(),
            report.failed(),
            report.total_duration_ms
        );

        if !report.success {
            for result in &report.results {
                if !result.success
                    && let Some(err) = &result.error
                {
                    eprintln!("  FAILED {}: {}", result.recipe_name, err);
                }
            }
            return Err(Error::RecipeFailed {
                name: "build".to_string(),
                code: 1,
                stderr: "one or more recipes failed".to_string(),
            });
        }

        info!("all recipes succeeded for target {target}");
    }

    Ok(())
}

fn explain_recipe(
    config: &must_config::schema::Config,
    mustfile_path: &std::path::Path,
    profile: &str,
    recipe_name: &str,
) -> must_core::Result<()> {
    use must_cache::hash::{compute_hash, hash_file};
    use must_config::schema::RecipeType;
    use std::collections::BTreeMap;

    let recipe = config
        .recipe
        .get(recipe_name)
        .ok_or_else(|| must_core::Error::UnknownRecipe {
            name: recipe_name.to_string(),
        })?;

    let project_root = mustfile_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    println!("Recipe:   {recipe_name}");
    println!("Type:     {:?}", recipe.recipe_type);
    println!(
        "Strategy: {}",
        match &recipe.cache {
            Some(must_config::schema::CacheMode::Hash) => "hash",
            Some(must_config::schema::CacheMode::Mtime) => "mtime",
            Some(must_config::schema::CacheMode::None) => "none",
            None => match recipe.recipe_type {
                RecipeType::Shell => "mtime (default)",
                _ => "hash (default)",
            },
        }
    );

    if !recipe.deps.is_empty() {
        println!("Deps:     {}", recipe.deps.join(", "));
    }

    // Expand and show input files with their hashes
    let env = must_engine::compose_env(
        config,
        recipe_name,
        profile,
        &std::collections::HashMap::new(),
    );

    if !recipe.inputs.is_empty() {
        println!("\nInputs:");
        for pattern in &recipe.inputs {
            let full = project_root.join(pattern).to_string_lossy().into_owned();
            match glob::glob(&full) {
                Ok(paths) => {
                    for entry in paths.flatten() {
                        let h = hash_file(&entry);
                        println!("  {} — {}", entry.display(), &h[..12]);
                    }
                }
                Err(_) => println!("  {pattern} (invalid glob)"),
            }
        }
    } else {
        println!("\nInputs:   (none declared — cargo tracks internally)");
    }

    // Show env vars affecting the key (all non-PATH vars)
    let relevant_env: BTreeMap<&str, &str> = env
        .iter()
        .filter(|(k, _)| {
            !matches!(
                k.as_str(),
                "PATH" | "HOME" | "USER" | "SHELL" | "TERM" | "COLORTERM" | "TMPDIR"
            )
        })
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    if !relevant_env.is_empty() {
        println!("\nEnv (affects hash):");
        for (k, v) in &relevant_env {
            let display = if v.len() > 60 {
                format!("{}...", &v[..60])
            } else {
                v.to_string()
            };
            println!("  {k} = {display}");
        }
    }

    // Compute cache key
    let recipe_type_str = match recipe.recipe_type {
        RecipeType::Shell => "shell",
        RecipeType::RustBin => "rust-bin",
        RecipeType::RustLib => "rust-lib",
        RecipeType::RustTest => "rust-test",
        RecipeType::GoBin => "go-bin",
        RecipeType::GoTest => "go-test",
        RecipeType::CBin => "c-bin",
        RecipeType::CLib => "c-lib",
    };
    let env_btree: BTreeMap<String, String> = relevant_env
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let hash = compute_hash(
        recipe_name,
        recipe_type_str,
        &[],
        &env_btree,
        "",
        &BTreeMap::new(),
    );

    println!("\nCache key: {hash}");

    // Check if it's a cache hit
    let cache_dir = project_root.join(".mustfile").join("cache");
    let key = must_core::CacheKey {
        recipe: recipe_name.to_string(),
        target: "host".to_string(),
        profile: profile.to_string(),
        hash: hash.clone(),
    };
    if let Ok(cache) = must_cache::store::DiskCache::open(&cache_dir) {
        use must_core::Cache;
        match cache.lookup(&key) {
            Ok(must_core::CacheLookup::Hit) => println!("Status:    HIT — would skip"),
            Ok(must_core::CacheLookup::Stale) => println!("Status:    STALE — would rebuild"),
            _ => println!("Status:    MISS — would build"),
        }
    } else {
        println!("Status:    MISS — no cache yet");
    }

    Ok(())
}

fn resolve_targets(raw_targets: &[String], config: &Config) -> Vec<String> {
    if raw_targets.is_empty() {
        return vec!["host".to_string()];
    }
    let mut resolved = Vec::new();
    for t in raw_targets {
        if let Some(triples) = config.targets.get(t) {
            resolved.extend(triples.iter().cloned());
        } else {
            resolved.push(t.clone());
        }
    }
    // deduplicate preserving order
    let mut seen = std::collections::HashSet::new();
    resolved.retain(|t| seen.insert(t.clone()));
    resolved
}

fn print_check(label: &str, ok: bool, hint: &str) {
    let icon = if ok { "✓" } else { "✗" };
    println!("  {icon} {label:<20}");
    if !ok {
        println!("    hint: {hint}");
    }
}

fn dir_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_dir() {
            total += dir_size(&entry.path()).unwrap_or(0);
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}

fn run_doctor() {
    println!("must doctor — environment health check\n");

    // --- Rust/cargo (required) ---
    let cargo_ok = std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    print_check("cargo", cargo_ok, "Install from https://rustup.rs");

    // --- Go (optional) ---
    let go_ok = must_toolchain::discover::go_installed();
    print_check(
        "go (optional)",
        go_ok,
        &must_toolchain::discover::go_install_hint(),
    );

    // --- C compiler / host (optional) ---
    let cc_ok = std::process::Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || std::process::Command::new("gcc")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    print_check(
        "cc/gcc (optional)",
        cc_ok,
        "Install Xcode Command Line Tools (macOS) or build-essential (Linux)",
    );

    // --- Container runtime (optional) ---
    match must_toolchain::container::detect_runtime() {
        Some(r) => {
            println!("  ✓ {:<20} — found: {:?}", "Container runtime", r);
        }
        None => {
            println!("  ? {:<20} — not found", "Container runtime");
            println!(
                "    hint: Install Docker (https://docs.docker.com/get-docker/) or Podman (https://podman.io/)"
            );
        }
    }

    // --- Cache ---
    let cache_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".mustfile")
        .join("cache");
    if cache_dir.exists() {
        let bytes = dir_size(&cache_dir).unwrap_or(0);
        let mb = bytes as f64 / (1024.0 * 1024.0);
        println!(
            "  ✓ {:<20} — {:.1} MB at {}",
            "Cache",
            mb,
            cache_dir.display()
        );
    } else {
        println!("  ✓ {:<20} — empty (no cache yet)", "Cache");
    }

    println!();
    if cargo_ok {
        println!("All required tools present. Ready to build.");
    } else {
        println!("Some required tools are missing. See hints above.");
        std::process::exit(1);
    }
}

fn print_graph(config: &Config, format: &str) -> must_core::Result<()> {
    let dep_map: HashMap<String, Vec<String>> = config
        .recipe
        .iter()
        .map(|(name, r)| (name.clone(), r.deps.clone()))
        .collect();
    let dag = Dag::new(dep_map.clone());
    let order = dag.topo_sort()?;

    match format {
        "dot" => {
            println!("digraph mustfile {{");
            println!("  rankdir=LR;");
            for name in &order {
                if let Some(deps) = dep_map.get(name) {
                    for dep in deps {
                        println!("  \"{name}\" -> \"{dep}\";");
                    }
                }
            }
            println!("}}");
        }
        "mermaid" => {
            println!("graph LR");
            for name in &order {
                if let Some(deps) = dep_map.get(name) {
                    for dep in deps {
                        println!("  {name} --> {dep}");
                    }
                }
            }
        }
        _ => {
            // text (default)
            println!("Recipe dependency graph:\n");
            for name in &order {
                let deps = dep_map.get(name).map(|d| d.as_slice()).unwrap_or(&[]);
                if deps.is_empty() {
                    println!("  {name}");
                } else {
                    println!("  {name} <- [{}]", deps.join(", "));
                }
            }
        }
    }
    Ok(())
}

fn find_mustfile() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("Mustfile.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use must_config::schema::{CacheMode, EnvMap, EnvValue, Project, Recipe, RecipeType};
    use std::collections::HashMap;

    fn make_config() -> Config {
        Config {
            project: Project {
                name: "test".into(),
                version: None,
            },
            env: EnvMap {
                global: HashMap::new(),
            },
            targets: HashMap::new(),
            recipe: HashMap::new(),
        }
    }

    fn make_recipe(recipe_type: RecipeType) -> Recipe {
        Recipe {
            recipe_type,
            deps: vec![],
            inputs: vec![],
            outputs: vec![],
            script: Some("echo ok".into()),
            cache: None,
            phony: false,
            env: HashMap::new(),
            cross: HashMap::new(),
            package: None,
            features: vec![],
            ldflags: None,
            sources: vec![],
            includes: vec![],
            link_libs: vec![],
        }
    }

    // ── resolve_targets ───────────────────────────────────────────────────────

    #[test]
    fn test_resolve_targets_empty_returns_host() {
        let config = make_config();
        assert_eq!(resolve_targets(&[], &config), vec!["host"]);
    }

    #[test]
    fn test_resolve_targets_direct_triple_passthrough() {
        let config = make_config();
        let result = resolve_targets(&["x86_64-unknown-linux-gnu".to_string()], &config);
        assert_eq!(result, vec!["x86_64-unknown-linux-gnu"]);
    }

    #[test]
    fn test_resolve_targets_group_expansion() {
        let mut config = make_config();
        config.targets.insert(
            "linux".into(),
            vec![
                "x86_64-unknown-linux-gnu".into(),
                "aarch64-unknown-linux-gnu".into(),
            ],
        );
        let result = resolve_targets(&["linux".to_string()], &config);
        assert_eq!(
            result,
            vec!["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]
        );
    }

    #[test]
    fn test_resolve_targets_deduplicates_preserving_order() {
        let config = make_config();
        let result = resolve_targets(
            &["host".to_string(), "host".to_string(), "host".to_string()],
            &config,
        );
        assert_eq!(result, vec!["host"]);
    }

    #[test]
    fn test_resolve_targets_mixed_group_and_direct() {
        let mut config = make_config();
        config
            .targets
            .insert("linux".into(), vec!["x86_64-unknown-linux-gnu".into()]);
        let result = resolve_targets(&["linux".to_string(), "host".to_string()], &config);
        assert_eq!(result, vec!["x86_64-unknown-linux-gnu", "host"]);
    }

    // ── explain_recipe ────────────────────────────────────────────────────────

    #[test]
    fn test_explain_unknown_recipe_returns_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mustfile = tmp.path().join("Mustfile.toml");
        std::fs::write(&mustfile, "").unwrap();
        let config = make_config();
        let result = explain_recipe(&config, &mustfile, "default", "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_explain_shell_recipe_no_inputs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mustfile = tmp.path().join("Mustfile.toml");
        std::fs::write(&mustfile, "").unwrap();
        let mut config = make_config();
        config
            .recipe
            .insert("build".into(), make_recipe(RecipeType::Shell));
        assert!(explain_recipe(&config, &mustfile, "default", "build").is_ok());
    }

    #[test]
    fn test_explain_recipe_with_input_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("data.txt"), "hello").unwrap();
        let mustfile = tmp.path().join("Mustfile.toml");
        std::fs::write(&mustfile, "").unwrap();
        let mut config = make_config();
        let mut r = make_recipe(RecipeType::Shell);
        r.inputs = vec!["data.txt".into()];
        config.recipe.insert("build".into(), r);
        assert!(explain_recipe(&config, &mustfile, "default", "build").is_ok());
    }

    #[test]
    fn test_explain_recipe_with_env_vars_displays_them() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mustfile = tmp.path().join("Mustfile.toml");
        std::fs::write(&mustfile, "").unwrap();
        let mut config = make_config();
        config
            .recipe
            .insert("build".into(), make_recipe(RecipeType::Shell));
        // Add a global env var that should appear in "Env (affects hash)" section
        config
            .env
            .global
            .insert("MY_BUILD_VAR".into(), EnvValue::Scalar("some_value".into()));
        assert!(explain_recipe(&config, &mustfile, "default", "build").is_ok());
    }

    #[test]
    fn test_explain_recipe_with_deps() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mustfile = tmp.path().join("Mustfile.toml");
        std::fs::write(&mustfile, "").unwrap();
        let mut config = make_config();
        let mut r = make_recipe(RecipeType::Shell);
        r.deps = vec!["codegen".into()];
        config.recipe.insert("build".into(), r);
        assert!(explain_recipe(&config, &mustfile, "default", "build").is_ok());
    }

    #[test]
    fn test_explain_recipe_with_hash_cache_mode() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mustfile = tmp.path().join("Mustfile.toml");
        std::fs::write(&mustfile, "").unwrap();
        let mut config = make_config();
        let mut r = make_recipe(RecipeType::Shell);
        r.cache = Some(CacheMode::Hash);
        config.recipe.insert("build".into(), r);
        assert!(explain_recipe(&config, &mustfile, "default", "build").is_ok());
    }

    #[test]
    fn test_explain_recipe_with_mtime_cache_mode() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mustfile = tmp.path().join("Mustfile.toml");
        std::fs::write(&mustfile, "").unwrap();
        let mut config = make_config();
        let mut r = make_recipe(RecipeType::Shell);
        r.cache = Some(CacheMode::Mtime);
        config.recipe.insert("build".into(), r);
        assert!(explain_recipe(&config, &mustfile, "default", "build").is_ok());
    }

    #[test]
    fn test_explain_recipe_with_none_cache_mode() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mustfile = tmp.path().join("Mustfile.toml");
        std::fs::write(&mustfile, "").unwrap();
        let mut config = make_config();
        let mut r = make_recipe(RecipeType::Shell);
        r.cache = Some(CacheMode::None);
        config.recipe.insert("build".into(), r);
        assert!(explain_recipe(&config, &mustfile, "default", "build").is_ok());
    }

    #[test]
    fn test_explain_all_recipe_types() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mustfile = tmp.path().join("Mustfile.toml");
        std::fs::write(&mustfile, "").unwrap();
        // Exercise every branch of the recipe_type_str match
        let types = [
            RecipeType::Shell,
            RecipeType::RustBin,
            RecipeType::RustLib,
            RecipeType::RustTest,
            RecipeType::GoBin,
            RecipeType::GoTest,
            RecipeType::CBin,
            RecipeType::CLib,
        ];
        for rtype in types {
            let mut config = make_config();
            config.recipe.insert("r".into(), make_recipe(rtype));
            assert!(explain_recipe(&config, &mustfile, "default", "r").is_ok());
        }
    }

    #[test]
    fn test_explain_recipe_invalid_glob_does_not_panic() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mustfile = tmp.path().join("Mustfile.toml");
        std::fs::write(&mustfile, "").unwrap();
        let mut config = make_config();
        let mut r = make_recipe(RecipeType::Shell);
        // glob::glob returns Err for this pattern
        r.inputs = vec!["***".into()];
        config.recipe.insert("build".into(), r);
        // Should succeed even with unparseable glob
        assert!(explain_recipe(&config, &mustfile, "default", "build").is_ok());
    }

    #[test]
    fn test_explain_recipe_long_env_value_truncated() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mustfile = tmp.path().join("Mustfile.toml");
        std::fs::write(&mustfile, "").unwrap();
        let mut config = make_config();
        config
            .recipe
            .insert("build".into(), make_recipe(RecipeType::Shell));
        // A value > 60 chars should be truncated with "..."
        config
            .env
            .global
            .insert("LONG_VAR".into(), EnvValue::Scalar("x".repeat(80)));
        assert!(explain_recipe(&config, &mustfile, "default", "build").is_ok());
    }

    // ── find_mustfile ─────────────────────────────────────────────────────────

    #[test]
    fn test_find_mustfile_returns_something_or_none_without_panic() {
        // We can't guarantee the working directory, but the function should not panic.
        let _ = find_mustfile();
    }

    // ── dir_size ──────────────────────────────────────────────────────────────

    #[test]
    fn test_dir_size_empty_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert_eq!(dir_size(tmp.path()).unwrap(), 0);
    }

    #[test]
    fn test_dir_size_counts_file_bytes() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("f.txt"), "hello").unwrap();
        assert_eq!(dir_size(tmp.path()).unwrap(), 5);
    }

    #[test]
    fn test_dir_size_recursive() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("a.txt"), "abc").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "de").unwrap();
        assert_eq!(dir_size(tmp.path()).unwrap(), 5);
    }

    // ── print_check ───────────────────────────────────────────────────────────

    #[test]
    fn test_print_check_ok() {
        // Just ensure it doesn't panic
        print_check("cargo", true, "not needed");
    }

    #[test]
    fn test_print_check_fail() {
        print_check("cargo", false, "install from rustup.rs");
    }

    // ── print_graph ───────────────────────────────────────────────────────────

    #[test]
    fn test_print_graph_text_no_recipes() {
        let config = make_config();
        assert!(print_graph(&config, "text").is_ok());
    }

    #[test]
    fn test_print_graph_dot_single_recipe() {
        let mut config = make_config();
        config
            .recipe
            .insert("build".into(), make_recipe(RecipeType::Shell));
        assert!(print_graph(&config, "dot").is_ok());
    }

    #[test]
    fn test_print_graph_mermaid_with_deps() {
        let mut config = make_config();
        config
            .recipe
            .insert("build".into(), make_recipe(RecipeType::Shell));
        let mut test_recipe = make_recipe(RecipeType::Shell);
        test_recipe.deps = vec!["build".into()];
        config.recipe.insert("test".into(), test_recipe);
        assert!(print_graph(&config, "mermaid").is_ok());
    }

    #[test]
    fn test_print_graph_unknown_format_falls_back_to_text() {
        let mut config = make_config();
        config
            .recipe
            .insert("build".into(), make_recipe(RecipeType::Shell));
        assert!(print_graph(&config, "xml").is_ok()); // unknown format → text fallback
    }

    #[test]
    fn test_print_graph_cycle_returns_error() {
        let mut config = make_config();
        let mut a = make_recipe(RecipeType::Shell);
        a.deps = vec!["b".into()];
        let mut b = make_recipe(RecipeType::Shell);
        b.deps = vec!["a".into()];
        config.recipe.insert("a".into(), a);
        config.recipe.insert("b".into(), b);
        assert!(print_graph(&config, "text").is_err());
    }

    // ── import_roundtrip ──────────────────────────────────────────────────────

    #[test]
    fn test_import_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mk = tmp.path().join("Makefile");
        std::fs::write(&mk, "build:\n\tgcc -o app main.c\n").unwrap();
        // call must_import directly (not via CLI) to keep test simple
        let input = std::fs::read_to_string(&mk).unwrap();
        let result = must_import::import(&input);
        assert!(
            result.toml.contains("[recipe.\"build\"]"),
            "writer always quotes recipe names"
        );
        assert_eq!(result.todo_count, 0);
    }
}
