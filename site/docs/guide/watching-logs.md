# Watching & Logs

## Watch mode

Automatically rebuild when files change:

```bash
must watch             # watch and rebuild on changes
must watch build test  # watch specific recipes
```

must uses filesystem watchers (via `notify` crate) to detect changes. When a change is detected:

1. Debounce for 200ms (avoid thrashing on multi-file saves)
2. Rebuild all watched recipes
3. Print results
4. Wait for next change

Press `Ctrl+C` to stop.

## Build logs

must stores build output for each recipe execution.

### View a log

```bash
must log build          # show last build output for "build"
must log test           # show last test output
```

### Follow in real time

```bash
must log build --follow
```

Streams new output as it arrives (like `tail -f`).

### List all logs

```bash
must log
```

Output:

```
NAME         SIZE
build        1.2 KB
test         4.5 KB
lint         256 B
```

### Clear logs

```bash
must log --clear
```

## Log location

Logs are stored in `.mustfile/logs/` as `<recipe-name>.log` files.

Add to `.gitignore`:

```
.mustfile/
```
