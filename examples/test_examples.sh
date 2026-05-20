#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-dry-run}"
MUST="${MUST:-must}"
PASS=0
FAIL=0
SKIP=0
RESULTS=()

log_pass() { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); }
log_fail() { RESULTS+=("FAIL  $1: $2"); FAIL=$((FAIL + 1)); }
log_skip() { RESULTS+=("SKIP  $1 ($2)"); SKIP=$((SKIP + 1)); }

has_cmd() { command -v "$1" &>/dev/null; }

must_run() {
    local dir="$1"
    shift
    if [[ "$MODE" == "dry-run" ]]; then
        "$MUST" "$@" --file "$dir/Mustfile.toml" --dry-run 2>&1
    else
        "$MUST" "$@" --file "$dir/Mustfile.toml" 2>&1
    fi
}

cd "$(dirname "$0")"

for dir in */; do
    dir="${dir%/}"
    [[ -f "$dir/Mustfile.toml" ]] || continue

    recipe_types=$(grep -o 'type *= *"[^"]*"' "$dir/Mustfile.toml" | sed 's/type *= *"\([^"]*\)"/\1/' | sort -u)

    skip_reason=""
    while IFS= read -r t; do
        case "$t" in
            rust-bin|rust-lib|rust-test)   has_cmd rustc   || skip_reason="rustc" ;;
            go-bin|go-test)                has_cmd go      || skip_reason="go" ;;
            c-bin|c-lib)                   has_cmd cc      || skip_reason="cc" ;;
            ts-bin|ts-check|ts-lint|npm)   has_cmd npx     || skip_reason="node/npm" ;;
            py-bin|py-test|py-lint)        has_cmd python3 || skip_reason="python3" ;;
            zig-bin|zig-test)              has_cmd zig     || skip_reason="zig" ;;
            java-bin|java-test)            has_cmd java    || skip_reason="java" ;;
            kotlin-bin|kotlin-test)        has_cmd gradle  || skip_reason="gradle/kotlin" ;;
            swift-bin|swift-test)          has_cmd swift   || skip_reason="swift" ;;
            dotnet-build|dotnet-test|dotnet-publish) has_cmd dotnet || skip_reason="dotnet" ;;
            ruby-bin|ruby-test)            has_cmd ruby    || skip_reason="ruby" ;;
            dart-bin|dart-test)            has_cmd dart    || skip_reason="dart" ;;
            elixir-build|elixir-test)      has_cmd elixir  || skip_reason="elixir" ;;
            flutter-build|flutter-test)    has_cmd flutter || skip_reason="flutter" ;;
            nim-bin|nim-test)              has_cmd nim     || skip_reason="nim" ;;
            docker-build|docker-push)      has_cmd docker  || skip_reason="docker" ;;
            precompiled-bin)               ;;
            shell)                         ;;
            bridge)                        ;;
            plugin)                        ;;
        esac
        [[ -n "$skip_reason" ]] && break
    done <<< "$recipe_types"

    if [[ -n "$skip_reason" ]]; then
        log_skip "$dir" "$skip_reason"
        continue
    fi

    echo "=== $dir ==="

    build_ok=true
    if grep -q '\[recipe\.build\]' "$dir/Mustfile.toml"; then
        if ! must_run "$dir" build; then
            build_ok=false
        fi
    fi

    test_ok=true
    if $build_ok && grep -q '\[recipe\.test\]' "$dir/Mustfile.toml"; then
        if ! must_run "$dir" test; then
            test_ok=false
        fi
    fi

    lint_ok=true
    if $build_ok && grep -q '\[recipe\.lint\]' "$dir/Mustfile.toml"; then
        if ! must_run "$dir" run lint; then
            lint_ok=false
        fi
    fi

    if $build_ok && $test_ok && $lint_ok; then
        log_pass "$dir"
    else
        msg=""
        $build_ok || msg="build failed"
        $test_ok || msg="${msg:+$msg, }test failed"
        $lint_ok || msg="${msg:+$msg, }lint failed"
        log_fail "$dir" "$msg"
    fi
done

echo ""
echo "========================================="
for r in "${RESULTS[@]}"; do
    echo "$r"
done
echo "========================================="
echo "Mode: $MODE  Pass: $PASS  Fail: $FAIL  Skip: $SKIP"

[[ $FAIL -eq 0 ]]
