# Java & Kotlin Recipes

must provides four JVM recipe types: `java-bin`, `java-test`, `kotlin-bin`, and `kotlin-test`.

Both Java and Kotlin recipes use Gradle (`./gradlew`) as the build tool.

## `java-bin` — Build a Java project

```toml
[recipe.build]
type    = "java-bin"
package = "."
```

Runs: `./gradlew build` (in package directory)

Cache key includes: package, env vars.

## `java-test` — Run Java tests

```toml
[recipe.test]
type    = "java-test"
package = "."
deps    = ["build"]
```

Runs: `./gradlew test` (in package directory)

Cache strategy: `never` (test results should always be fresh).

## `kotlin-bin` — Build a Kotlin project

```toml
[recipe.build]
type    = "kotlin-bin"
package = "."
```

Runs: `./gradlew build` (in package directory)

Cache key includes: package, env vars.

## `kotlin-test` — Run Kotlin tests

```toml
[recipe.test]
type    = "kotlin-test"
package = "."
deps    = ["build"]
```

Runs: `./gradlew test` (in package directory)

Cache strategy: `never` (test results should always be fresh).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | all | Project directory path (default `.`) |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

## Examples

```toml
[project]
name = "my-jvm-workspace"

[recipe.build-api]
type    = "java-bin"
package = "services/api"

[recipe.test-api]
type    = "java-test"
package = "services/api"
deps    = ["build-api"]

[recipe.build-core]
type    = "kotlin-bin"
package = "libs/core"

[recipe.test-core]
type    = "kotlin-test"
package = "libs/core"
deps    = ["build-core"]
```
