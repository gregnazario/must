use must_core::{BuildContext, Error, Recipe, Result};
use must_graph::Dag;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub recipe_name: String,
    pub from_cache: bool,
    pub success: bool,
    pub duration_ms: u64,
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
                            ExecutionResult {
                                recipe_name: name,
                                from_cache: output.from_cache,
                                success: true,
                                duration_ms,
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
}
