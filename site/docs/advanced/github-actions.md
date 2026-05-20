# GitHub Actions

must provides a GitHub Actions composite action for CI.

## Basic usage

```yaml
- uses: anomalyco/must@main
  with:
    command: build
    profile: release
```

## Full workflow example

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Build
        uses: anomalyco/mustfile@main
        with:
          command: build

      - name: Test
        uses: anomalyco/mustfile@main
        with:
          command: test

      - name: Lint
        uses: anomalyco/mustfile@main
        with:
          command: lint
```

## Action inputs

| Input | Description | Default |
|-------|-------------|---------|
| `command` | must command to run (e.g., `build`, `test`) | `build` |
| `file` | Path to Mustfile.toml | Auto-detect |
| `profile` | Environment profile | `default` |
| `target` | Cross-compilation target | — |
| `fail-fast` | Cancel on first failure | `true` |
| `dry-run` | Plan without executing | `false` |

## Release workflow

```yaml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: anomalyco/must@main
        with:
          command: build
          profile: release
          target: release
```

## Caching in CI

must's cache is stored in `.must/cache/`. Cache it between runs:

```yaml
- uses: actions/cache@v4
  with:
    path: .must/cache
    key: must-${{ runner.os }}-${{ hashFiles('**/Mustfile.toml') }}
```
