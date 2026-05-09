# Docker Recipes

must provides two Docker recipe types: `docker-build` and `docker-push`. Podman is used as a fallback if Docker is not available.

## `docker-build` — Build an image

```toml
[recipe.build]
type       = "docker-build"
image      = "myapp:latest"
dockerfile = "Dockerfile"
context    = "."
build_args = ["VERSION=1.0"]
```

Runs: `docker build -t myapp:latest -f Dockerfile --build-arg VERSION=1.0 .`

Cache key includes: image, dockerfile, context, build_args, env vars.

## `docker-push` — Push an image

```toml
[recipe.push]
type  = "docker-push"
image = "myregistry/myapp:latest"
deps  = ["build"]
```

Runs: `docker push myregistry/myapp:latest`

Cache strategy: `never` (push should always execute).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `image` | string | all | Docker image name and tag |
| `dockerfile` | string | build | Path to Dockerfile (default `.`) |
| `context` | string | build | Build context directory (default `.`) |
| `build_args` | string[] | build | Build arguments (`--build-arg`) |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"never"`), `phony` (always re-run), and `workdir` (working directory).

## Examples

```toml
[project]
name = "my-docker-project"

[recipe.build]
type       = "docker-build"
image      = "myapp:latest"
dockerfile = "Dockerfile.prod"
context    = "."
build_args = ["VERSION=2.0", "ENV=prod"]

[recipe.push]
type  = "docker-push"
image = "myregistry/myapp:latest"
deps  = ["build"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Shell Recipes](shell.md) — custom build scripts
