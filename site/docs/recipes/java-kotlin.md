# Java & Kotlin Recipes

must provides four JVM recipe types: `java-bin`, `java-test`, `kotlin-bin`, and `kotlin-test`.

Both Java and Kotlin recipes use Gradle (`./gradlew`) as the build tool.

## Quick start

```
my-app/
├── build.gradle.kts
├── settings.gradle.kts
├── Mustfile.toml
└── src/
    ├── main/java/com/example/
    │   └── App.java
    └── test/java/com/example/
        └── AppTest.java
```

**`build.gradle.kts`**

```kotlin
plugins {
    application
}

application {
    mainClass.set("com.example.App")
}
```

**`src/main/java/com/example/App.java`**

```java
package com.example;

public class App {
    public static void main(String[] args) {
        System.out.println("Hello from must!");
    }
}
```

**`src/test/java/com/example/AppTest.java`**

```java
package com.example;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class AppTest {
    @Test
    void greetingIsNotEmpty() {
        assertNotEquals("", "Hello from must!");
    }
}
```

**`Mustfile.toml`**

```toml
[project]
name = "my-app"
version = "0.1.0"

[recipe.build]
type    = "java-bin"
package = "."

[recipe.test]
type    = "java-test"
package = "."
deps    = ["build"]
```

Build and test:

```
$ must build
● build  java-bin  ./gradlew build
  BUILD SUCCESSFUL

$ must test
● test  java-test  ./gradlew test
  BUILD SUCCESSFUL
```

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

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

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

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Shell Recipes](shell.md) — custom build scripts
