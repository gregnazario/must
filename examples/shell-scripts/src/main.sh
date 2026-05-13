#!/usr/bin/env bash

main() {
    echo "${APP_NAME} v${VERSION}"
    echo "Running from $(pwd)"
}

main "$@"
