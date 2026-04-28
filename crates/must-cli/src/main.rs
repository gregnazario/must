use clap::{Parser, Subcommand};
use must_config::load_config;
use must_config::schema::{CacheMode, Config, RecipeType};
use must_core::{BuildContext, CacheStrategy, Error};
use must_engine::{compose_env, Engine};
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
        Commands::Build { recipes } => {
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
    }

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
    let RunOpts { profile, parallelism, dry_run, fail_fast, target_recipes, targets } = opts;
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
                    package: recipe_cfg.package.clone().unwrap_or_else(|| ".".to_string()),
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
                    package: recipe_cfg.package.clone().unwrap_or_else(|| "./...".to_string()),
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
                if !result.success {
                    if let Some(err) = &result.error {
                        eprintln!("  FAILED {}: {}", result.recipe_name, err);
                    }
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

    let recipe = config.recipe.get(recipe_name).ok_or_else(|| {
        must_core::Error::UnknownRecipe { name: recipe_name.to_string() }
    })?;

    let project_root = mustfile_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    println!("Recipe:   {recipe_name}");
    println!("Type:     {:?}", recipe.recipe_type);
    println!("Strategy: {}", match &recipe.cache {
        Some(must_config::schema::CacheMode::Hash) => "hash",
        Some(must_config::schema::CacheMode::Mtime) => "mtime",
        Some(must_config::schema::CacheMode::None) => "none",
        None => match recipe.recipe_type {
            RecipeType::Shell => "mtime (default)",
            _ => "hash (default)",
        },
    });

    if !recipe.deps.is_empty() {
        println!("Deps:     {}", recipe.deps.join(", "));
    }

    // Expand and show input files with their hashes
    let env = must_engine::compose_env(config, recipe_name, profile, &std::collections::HashMap::new());

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
    let relevant_env: BTreeMap<&str, &str> = env.iter()
        .filter(|(k, _)| !matches!(k.as_str(), "PATH" | "HOME" | "USER" | "SHELL" | "TERM" | "COLORTERM" | "TMPDIR"))
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    if !relevant_env.is_empty() {
        println!("\nEnv (affects hash):");
        for (k, v) in &relevant_env {
            let display = if v.len() > 60 { format!("{}...", &v[..60]) } else { v.to_string() };
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
    let env_btree: BTreeMap<String, String> = relevant_env.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let hash = compute_hash(recipe_name, recipe_type_str, &[], &env_btree, "", &BTreeMap::new());

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
            Ok(must_core::CacheLookup::Hit)   => println!("Status:    HIT — would skip"),
            Ok(must_core::CacheLookup::Stale) => println!("Status:    STALE — would rebuild"),
            _                                  => println!("Status:    MISS — would build"),
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
