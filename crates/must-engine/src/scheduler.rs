use must_core::{BuildContext, Error, Recipe, Result};
use must_graph::Dag;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize)]
pub enum ProgressEvent {
    Starting { recipe: String, total: usize, completed: usize },
    Completed { recipe: String, success: bool, from_cache: bool, duration_ms: u64 },
    WaveDone { completed: usize, total: usize },
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub recipe_name: String,
    pub from_cache: bool,
    pub success: bool,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct ExecutionReport {
    pub results: Vec<ExecutionResult>,
    pub total_duration_ms: u64,
    pub success: bool,
}

impl ExecutionReport {
    pub fn built(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.success && !r.from_cache)
            .count()
    }

    pub fn cached(&self) -> usize {
        self.results.iter().filter(|r| r.from_cache).count()
    }

    pub fn failed(&self) -> usize {
        self.results.iter().filter(|r| !r.success).count()
    }
}

pub struct Engine {
    parallelism: usize,
    fail_fast: bool,
}

impl Engine {
    pub fn new(parallelism: usize, fail_fast: bool) -> Self {
        Self {
            parallelism,
            fail_fast,
        }
    }

    pub async fn execute(
        &self,
        dag: &Dag,
        recipes: &HashMap<String, Arc<dyn Recipe>>,
        ctx: &BuildContext,
    ) -> Result<ExecutionReport> {
        let start = std::time::Instant::now();
        let waves = dag.waves()?;
        let semaphore = Arc::new(Semaphore::new(self.parallelism));
        let mut all_results: Vec<ExecutionResult> = Vec::new();
        let mut failed = false;

        'waves: for wave in waves {
            if failed && self.fail_fast {
                break 'waves;
            }

            let mut handles = Vec::new();

            for recipe_name in wave {
                let recipe = match recipes.get(&recipe_name) {
                    Some(r) => Arc::clone(r),
                    None => return Err(Error::UnknownRecipe { name: recipe_name }),
                };
                let ctx = ctx.clone();
                let sem = Arc::clone(&semaphore);

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.expect("semaphore closed");
                    info!(recipe = %recipe.name(), "starting recipe");
                    let name = recipe.name().to_string();
                    let exec_start = std::time::Instant::now();
                    match recipe.execute(&ctx) {
                        Ok(output) => {
                            let duration_ms = exec_start.elapsed().as_millis() as u64;
                            info!(
                                recipe = %name,
                                from_cache = output.from_cache,
                                duration_ms,
                                "recipe complete"
                            );

                            if !output.stdout.is_empty() || !output.stderr.is_empty() {
                                let _ = std::fs::create_dir_all(&ctx.log_dir);
                                let log_path = ctx.log_dir.join(format!("{name}.log"));
                                let mut log_content = String::new();
                                if !output.stdout.is_empty() {
                                    log_content.push_str(&output.stdout);
                                }
                                if !output.stderr.is_empty() {
                                    if !log_content.is_empty() {
                                        log_content.push('\n');
                                    }
                                    log_content.push_str("--- stderr ---\n");
                                    log_content.push_str(&output.stderr);
                                }
                                let _ = std::fs::write(&log_path, &log_content);
                            }

                            ExecutionResult {
                                recipe_name: name,
                                from_cache: output.from_cache,
                                success: true,
                                duration_ms,
                                stdout: output.stdout,
                                stderr: output.stderr,
                                error: None,
                            }
                        }
                        Err(e) => {
                            let duration_ms = exec_start.elapsed().as_millis() as u64;
                            error!(recipe = %name, error = %e, "recipe failed");
                            ExecutionResult {
                                recipe_name: name,
                                from_cache: false,
                                success: false,
                                duration_ms,
                                stdout: String::new(),
                                stderr: String::new(),
                                error: Some(e.to_string()),
                            }
                        }
                    }
                });
                handles.push(handle);
            }

            // Wait for all tasks in this wave
            let mut wave_failed = false;
            for handle in handles {
                match handle.await {
                    Ok(result) => {
                        if !result.success {
                            wave_failed = true;
                            failed = true;
                        }
                        all_results.push(result);
                    }
                    Err(e) => {
                        warn!("task panicked: {e}");
                        failed = true;
                        wave_failed = true;
                        all_results.push(ExecutionResult {
                            recipe_name: "unknown".to_string(),
                            from_cache: false,
                            success: false,
                            duration_ms: 0,
                            stdout: String::new(),
                            stderr: String::new(),
                            error: Some(format!("task panicked: {e}")),
                        });
                    }
                }
            }

            if wave_failed && self.fail_fast {
                break 'waves;
            }
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;
        Ok(ExecutionReport {
            success: !failed,
            results: all_results,
            total_duration_ms,
        })
    }

    pub async fn execute_with_progress(
        &self,
        dag: &Dag,
        recipes: &HashMap<String, Arc<dyn Recipe>>,
        ctx: &BuildContext,
        progress_tx: tokio::sync::mpsc::Sender<ProgressEvent>,
    ) -> Result<ExecutionReport> {
        let start = std::time::Instant::now();
        let waves = dag.waves()?;
        let semaphore = Arc::new(Semaphore::new(self.parallelism));
        let mut all_results: Vec<ExecutionResult> = Vec::new();
        let mut failed = false;
        let total_recipes: usize = waves.iter().map(|w| w.len()).sum();
        let mut completed = 0usize;

        'waves: for wave in waves {
            if failed && self.fail_fast {
                break 'waves;
            }

            let mut handles = Vec::new();

            for recipe_name in wave {
                let recipe = match recipes.get(&recipe_name) {
                    Some(r) => Arc::clone(r),
                    None => return Err(Error::UnknownRecipe { name: recipe_name }),
                };
                let ctx = ctx.clone();
                let sem = Arc::clone(&semaphore);
                let tx = progress_tx.clone();

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.expect("semaphore closed");
                    let name = recipe.name().to_string();
                    let _ = tx.send(ProgressEvent::Starting {
                        recipe: name.clone(),
                        total: 0,
                        completed: 0,
                    }).await;
                    info!(recipe = %recipe.name(), "starting recipe");
                    let exec_start = std::time::Instant::now();
                    let result = match recipe.execute(&ctx) {
                        Ok(output) => {
                            let duration_ms = exec_start.elapsed().as_millis() as u64;
                            info!(
                                recipe = %name,
                                from_cache = output.from_cache,
                                duration_ms,
                                "recipe complete"
                            );

                            if !output.stdout.is_empty() || !output.stderr.is_empty() {
                                let _ = std::fs::create_dir_all(&ctx.log_dir);
                                let log_path = ctx.log_dir.join(format!("{name}.log"));
                                let mut log_content = String::new();
                                if !output.stdout.is_empty() {
                                    log_content.push_str(&output.stdout);
                                }
                                if !output.stderr.is_empty() {
                                    if !log_content.is_empty() {
                                        log_content.push('\n');
                                    }
                                    log_content.push_str("--- stderr ---\n");
                                    log_content.push_str(&output.stderr);
                                }
                                let _ = std::fs::write(&log_path, &log_content);
                            }

                            ExecutionResult {
                                recipe_name: name.clone(),
                                from_cache: output.from_cache,
                                success: true,
                                duration_ms,
                                stdout: output.stdout,
                                stderr: output.stderr,
                                error: None,
                            }
                        }
                        Err(e) => {
                            let duration_ms = exec_start.elapsed().as_millis() as u64;
                            error!(recipe = %name, error = %e, "recipe failed");
                            ExecutionResult {
                                recipe_name: name.clone(),
                                from_cache: false,
                                success: false,
                                duration_ms,
                                stdout: String::new(),
                                stderr: String::new(),
                                error: Some(e.to_string()),
                            }
                        }
                    };
                    let _ = tx.send(ProgressEvent::Completed {
                        recipe: name,
                        success: result.success,
                        from_cache: result.from_cache,
                        duration_ms: result.duration_ms,
                    }).await;
                    result
                });
                handles.push(handle);
            }

            let mut wave_failed = false;
            for handle in handles {
                match handle.await {
                    Ok(result) => {
                        completed += 1;
                        if !result.success {
                            wave_failed = true;
                            failed = true;
                        }
                        all_results.push(result);
                    }
                    Err(e) => {
                        completed += 1;
                        warn!("task panicked: {e}");
                        failed = true;
                        wave_failed = true;
                        all_results.push(ExecutionResult {
                            recipe_name: "unknown".to_string(),
                            from_cache: false,
                            success: false,
                            duration_ms: 0,
                            stdout: String::new(),
                            stderr: String::new(),
                            error: Some(format!("task panicked: {e}")),
                        });
                    }
                }
            }

            let _ = progress_tx.send(ProgressEvent::WaveDone {
                completed,
                total: total_recipes,
            }).await;

            if wave_failed && self.fail_fast {
                break 'waves;
            }
        }

        drop(progress_tx);
        let total_duration_ms = start.elapsed().as_millis() as u64;
        Ok(ExecutionReport {
            success: !failed,
            results: all_results,
            total_duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use must_core::{BuildContext, CacheKey, CacheStrategy, RecipeOutput};
    use std::collections::HashMap;
    use std::sync::Arc;

    struct SuccessRecipe {
        name: String,
    }

    impl must_core::Recipe for SuccessRecipe {
        fn name(&self) -> &str {
            &self.name
        }
        fn deps(&self) -> &[String] {
            &[]
        }
        fn inputs(
            &self,
            _: &must_core::BuildContext,
        ) -> must_core::Result<Vec<std::path::PathBuf>> {
            Ok(vec![])
        }
        fn outputs(
            &self,
            _: &must_core::BuildContext,
        ) -> must_core::Result<Vec<std::path::PathBuf>> {
            Ok(vec![])
        }
        fn cache_strategy(&self) -> CacheStrategy {
            CacheStrategy::Never
        }
        fn cache_key(&self, _: &must_core::BuildContext) -> must_core::Result<CacheKey> {
            Ok(CacheKey {
                recipe: self.name.clone(),
                target: "host".into(),
                profile: "default".into(),
                hash: "abc".into(),
            })
        }
        fn execute(&self, _: &must_core::BuildContext) -> must_core::Result<RecipeOutput> {
            Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 1,
            })
        }
    }

    struct FailRecipe {
        name: String,
    }

    impl must_core::Recipe for FailRecipe {
        fn name(&self) -> &str {
            &self.name
        }
        fn deps(&self) -> &[String] {
            &[]
        }
        fn inputs(
            &self,
            _: &must_core::BuildContext,
        ) -> must_core::Result<Vec<std::path::PathBuf>> {
            Ok(vec![])
        }
        fn outputs(
            &self,
            _: &must_core::BuildContext,
        ) -> must_core::Result<Vec<std::path::PathBuf>> {
            Ok(vec![])
        }
        fn cache_strategy(&self) -> CacheStrategy {
            CacheStrategy::Never
        }
        fn cache_key(&self, _: &must_core::BuildContext) -> must_core::Result<CacheKey> {
            Ok(CacheKey {
                recipe: self.name.clone(),
                target: "host".into(),
                profile: "default".into(),
                hash: "abc".into(),
            })
        }
        fn execute(&self, _: &must_core::BuildContext) -> must_core::Result<RecipeOutput> {
            Err(must_core::Error::RecipeFailed {
                name: self.name.clone(),
                code: 1,
                stderr: "oops".into(),
            })
        }
    }

    fn test_ctx() -> BuildContext {
        BuildContext {
            project_root: std::path::PathBuf::from("/tmp"),
            cache_dir: std::path::PathBuf::from("/tmp/.mustfile/cache"),
            log_dir: std::path::PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
        }
    }

    #[tokio::test]
    async fn test_engine_single_success() {
        let mut recipes: HashMap<String, Arc<dyn must_core::Recipe>> = HashMap::new();
        recipes.insert(
            "build".into(),
            Arc::new(SuccessRecipe {
                name: "build".into(),
            }),
        );
        let dag = must_graph::Dag::new([("build".to_string(), vec![])].into());
        let engine = Engine::new(1, false);
        let report = engine.execute(&dag, &recipes, &test_ctx()).await.unwrap();
        assert!(report.success);
        assert_eq!(report.built(), 1);
        assert_eq!(report.cached(), 0);
        assert_eq!(report.failed(), 0);
    }

    #[tokio::test]
    async fn test_engine_single_failure_no_fail_fast() {
        let mut recipes: HashMap<String, Arc<dyn must_core::Recipe>> = HashMap::new();
        recipes.insert(
            "build".into(),
            Arc::new(FailRecipe {
                name: "build".into(),
            }),
        );
        let dag = must_graph::Dag::new([("build".to_string(), vec![])].into());
        let engine = Engine::new(1, false);
        let report = engine.execute(&dag, &recipes, &test_ctx()).await.unwrap();
        assert!(!report.success);
        assert_eq!(report.failed(), 1);
    }

    #[tokio::test]
    async fn test_engine_fail_fast_stops_after_failure() {
        // wave 1: fail_recipe; wave 2: success_recipe (depends on fail_recipe)
        // With fail_fast, wave 2 should be skipped
        let mut recipes: HashMap<String, Arc<dyn must_core::Recipe>> = HashMap::new();
        recipes.insert("a".into(), Arc::new(FailRecipe { name: "a".into() }));
        recipes.insert("b".into(), Arc::new(SuccessRecipe { name: "b".into() }));
        // b depends on a, so they're in separate waves
        let dag = must_graph::Dag::new(
            [
                ("a".to_string(), vec![]),
                ("b".to_string(), vec!["a".to_string()]),
            ]
            .into(),
        );
        let engine = Engine::new(1, true); // fail_fast = true
        let report = engine.execute(&dag, &recipes, &test_ctx()).await.unwrap();
        assert!(!report.success);
        // b should NOT have been executed (fail_fast stopped after wave 1)
        assert!(!report.results.iter().any(|r| r.recipe_name == "b"));
    }

    #[tokio::test]
    async fn test_engine_two_successes_report_counts() {
        let mut recipes: HashMap<String, Arc<dyn must_core::Recipe>> = HashMap::new();
        recipes.insert("a".into(), Arc::new(SuccessRecipe { name: "a".into() }));
        recipes.insert("b".into(), Arc::new(SuccessRecipe { name: "b".into() }));
        let dag =
            must_graph::Dag::new([("a".to_string(), vec![]), ("b".to_string(), vec![])].into());
        let engine = Engine::new(2, false);
        let report = engine.execute(&dag, &recipes, &test_ctx()).await.unwrap();
        assert!(report.success);
        assert_eq!(report.built(), 2);
        assert_eq!(report.failed(), 0);
    }

    struct PanicRecipe {
        name: String,
    }
    impl must_core::Recipe for PanicRecipe {
        fn name(&self) -> &str {
            &self.name
        }
        fn deps(&self) -> &[String] {
            &[]
        }
        fn inputs(
            &self,
            _: &must_core::BuildContext,
        ) -> must_core::Result<Vec<std::path::PathBuf>> {
            Ok(vec![])
        }
        fn outputs(
            &self,
            _: &must_core::BuildContext,
        ) -> must_core::Result<Vec<std::path::PathBuf>> {
            Ok(vec![])
        }
        fn cache_strategy(&self) -> CacheStrategy {
            CacheStrategy::Never
        }
        fn cache_key(&self, _: &must_core::BuildContext) -> must_core::Result<CacheKey> {
            Ok(CacheKey {
                recipe: self.name.clone(),
                target: "host".into(),
                profile: "default".into(),
                hash: "xyz".into(),
            })
        }
        fn execute(&self, _: &must_core::BuildContext) -> must_core::Result<RecipeOutput> {
            panic!("deliberate panic in test")
        }
    }

    #[tokio::test]
    async fn test_engine_handles_panicking_recipe() {
        let mut recipes: HashMap<String, Arc<dyn must_core::Recipe>> = HashMap::new();
        recipes.insert(
            "panic-recipe".into(),
            Arc::new(PanicRecipe {
                name: "panic-recipe".into(),
            }),
        );
        let dag = must_graph::Dag::new([("panic-recipe".to_string(), vec![])].into());
        let engine = Engine::new(1, false);
        let report = engine.execute(&dag, &recipes, &test_ctx()).await.unwrap();
        assert!(
            !report.success,
            "panicking recipe should mark report as failed"
        );
        assert_eq!(report.failed(), 1);
    }

    #[tokio::test]
    async fn test_engine_unknown_recipe_in_dag_returns_error() {
        // DAG mentions "ghost" but recipe map only has "build"
        let mut recipes: HashMap<String, Arc<dyn must_core::Recipe>> = HashMap::new();
        recipes.insert(
            "build".into(),
            Arc::new(SuccessRecipe {
                name: "build".into(),
            }),
        );
        // Dag with "ghost" which is NOT in the recipes map
        let dag = must_graph::Dag::new([("ghost".to_string(), vec![])].into());
        let engine = Engine::new(1, false);
        let result = engine.execute(&dag, &recipes, &test_ctx()).await;
        assert!(result.is_err(), "unknown recipe in DAG should return Err");
    }

    #[tokio::test]
    async fn test_execution_report_cached_count() {
        let report = ExecutionReport {
            results: vec![
                ExecutionResult {
                    recipe_name: "a".into(),
                    from_cache: true,
                    success: true,
                    duration_ms: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                    error: None,
                },
                ExecutionResult {
                    recipe_name: "b".into(),
                    from_cache: false,
                    success: true,
                    duration_ms: 1,
                    stdout: String::new(),
                    stderr: String::new(),
                    error: None,
                },
            ],
            total_duration_ms: 5,
            success: true,
        };
        assert_eq!(report.cached(), 1);
        assert_eq!(report.built(), 1);
        assert_eq!(report.failed(), 0);
    }
}
