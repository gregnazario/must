# .NET Recipes

must provides three .NET recipe types: `dotnet-build`, `dotnet-test`, and `dotnet-publish`.

## Quick start

Project structure:

```
my-app/
├── Mustfile.toml
├── MyApp.csproj
└── Program.cs
```

`MyApp.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
</Project>
```

`Program.cs`:

```csharp
static string Greet(string name) => $"Hello, {name}!";

Console.WriteLine(Greet("world"));
```

`Mustfile.toml`:

```toml
[project]
name = "my-app"
version = "0.1.0"

[recipe.build]
type    = "dotnet-build"
package = "MyApp.csproj"

[recipe.test]
type    = "dotnet-test"
package = "MyApp.csproj"
deps    = ["build"]

[recipe.publish]
type    = "dotnet-publish"
package = "MyApp.csproj"
deps    = ["test"]
```

Build the project:

```
$ must build
● build dotnet-build MyApp.csproj
  Microsoft (R) Build Engine version 17.8.0
  Build succeeded.
    0 Warning(s)
    0 Error(s)
✓ build (1.2s)
```

Run tests:

```
$ must test
● build dotnet-build MyApp.csproj (cached)
● test dotnet-test MyApp.csproj
  Passed!  - Failed: 0, Passed: 1, Skipped: 0, Total: 1
✓ test (0.8s)
```

## `dotnet-build` — Build a project

```toml
[recipe.build]
type    = "dotnet-build"
package = "MyApp.csproj"
```

Runs: `dotnet build MyApp.csproj`

Cache key includes: package, env vars.

## `dotnet-test` — Run tests

```toml
[recipe.test]
type    = "dotnet-test"
package = "tests/MyApp.Tests"
deps    = ["build"]
```

Runs: `dotnet test tests/MyApp.Tests`

Cache strategy: `never` (test results should always be fresh).

## `dotnet-publish` — Publish a project

```toml
[recipe.publish]
type    = "dotnet-publish"
package = "src/MyApp"
deps    = ["build"]
```

Runs: `dotnet publish src/MyApp -c Release`

Cache key includes: package, env vars.

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | all | Project path or `.csproj` file |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

## Examples

```toml
[project]
name = "my-dotnet-solution"

[recipe.build]
type    = "dotnet-build"
package = "src/MyApp"

[recipe.test]
type    = "dotnet-test"
package = "tests/MyApp.Tests"
deps    = ["build"]

[recipe.publish]
type    = "dotnet-publish"
package = "src/MyApp"
deps    = ["test"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Shell Recipes](shell.md) — custom build scripts
