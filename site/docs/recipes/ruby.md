# Ruby Recipes

must provides two Ruby recipe types: `ruby-bin` and `ruby-test`.

## `ruby-bin` — Install dependencies

```toml
[recipe.build]
type    = "ruby-bin"
package = "."
```

Runs: `bundle install` (in package directory)

Cache key includes: package, env vars.

## `ruby-test` — Run tests

```toml
[recipe.test]
type    = "ruby-test"
package = "."
deps    = ["build"]
```

Runs: `bundle exec rspec` (in package directory)

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
name = "my-ruby-project"

[recipe.build]
type    = "ruby-bin"
package = "."

[recipe.test]
type    = "ruby-test"
package = "."
deps    = ["build"]
```
