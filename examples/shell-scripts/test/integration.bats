#!/usr/bin/env bats

@test "bundle is executable" {
    skip "requires build first"
    [ -x dist/bundle.sh ]
}

@test "header sets APP_NAME" {
    source src/header.sh
    [ "${APP_NAME}" = "shell-scripts" ]
}

@test "main function runs without error" {
    source src/header.sh
    source src/main.sh
    run main
    [ "$status" -eq 0 ]
}
