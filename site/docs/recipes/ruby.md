# Ruby Recipes

must provides two Ruby recipe types: `ruby-bin` and `ruby-test`.

## Quick start

### Project structure

```
myapp/
├── Mustfile.toml
├── Gemfile
├── lib/
│   └── myapp.rb
└── test/
    └── test_myapp.rb
```

### `Gemfile`

```ruby
source "https://rubygems.org"

gem "minitest"
```

### `lib/myapp.rb`

```ruby
module MyApp
  def self.greet(name)
    "Hello, #{name}!"
  end
end
```

### `test/test_myapp.rb`

```ruby
require "minitest/autorun"
require_relative "../lib/myapp"

class TestMyApp < Minitest::Test
  def test_greet
    assert_equal "Hello, world!", MyApp.greet("world")
  end
end
```

### `Mustfile.toml`

```toml
[project]
name = "myapp"
version = "0.1.0"

[recipe.build]
type    = "ruby-bin"
package = "."

[recipe.test]
type    = "ruby-test"
package = "."
deps    = ["build"]
```

### Build and test

```
$ must build
● build (ruby-bin) → bundle install
  Fetching gem metadata from https://rubygems.org/
  Resolving dependencies...
  Using minitest 5.25.4
  ✓ build done (1.2s)

$ must test
● build (ruby-bin) → cache hit
● test  (ruby-test) → bundle exec rspec
  1 runs, 1 assertions, 0 failures, 0 errors, 0 skips
  ✓ test done (0.8s)
```

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

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

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

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Shell Recipes](shell.md) — custom build scripts
