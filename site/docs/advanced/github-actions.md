# GitHub Actions

must ships no composite GitHub Action; CI installs the CLI, then calls `must`
commands directly.

## Installing in a workflow

```yaml
- name: Install must
  run: |
    curl -fsSL https://github.com/gregnazario/must/releases/latest/download/install.sh \
      -o install.sh
    sh install.sh && rm install.sh
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

      - name: Install must
        run: |
          curl -fsSL https://github.com/gregnazario/must/releases/latest/download/install.sh \
            -o install.sh
          sh install.sh && rm install.sh

      - name: Build
        run: must build

      - name: Test
        run: must test

      - name: Lint
        run: must lint
```

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

      - name: Install must
        run: |
          curl -fsSL https://github.com/gregnazario/must/releases/latest/download/install.sh \
            -o install.sh
          sh install.sh && rm install.sh

      - name: Build
        run: must --profile release build
```

## Caching in CI

must's cache is stored in `.must/cache/`. Cache it between runs:

```yaml
- uses: actions/cache@v4
  with:
    path: .must/cache
    key: must-${{ runner.os }}-${{ hashFiles('**/Mustfile.toml') }}
```
