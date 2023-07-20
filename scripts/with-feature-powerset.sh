#!/bin/bash

set -e -o pipefail

# Run commands to build and test with a feature powerset. Requires `cargo-hack` to be installed.

EXCLUDED_FEATURES=(internal-docs)
EXCLUDED_FEATURES_NO_STD=(std internal-docs default)


run_build() {
    check_cargo_hack

    echo_err "Running non-dev build" >&2
    run_cargo_hack build

    echo_err "Running dev build"
    run_cargo_hack build --all-targets
}

run_test() {
    check_cargo_hack

    echo_err "Running non-doc tests"
    run_cargo_hack test --lib --bins --tests --benches --examples

    echo_err "Running doctests"
    cargo test --all-features --doc
}

run_build_no_std() {
    local build_target="$1"

    echo_err "Building for no-std target ${build_target}"
    run_cargo_hack_no_std build --target "${build_target}"
}

echo_err() {
    echo "$@" >&2
}

check_cargo_hack() {
    if ! cargo hack --version >/dev/null 2>&1; then
        echo_err "cargo-hack not installed. Install it with:"
        echo_err "    cargo install cargo-hack"
        exit 1
    fi
}

run_cargo_hack() {
    joined_excluded_features=$(printf ",%s" "${EXCLUDED_FEATURES[@]}")
    # Strip leading comma
    joined_excluded_features=${joined_excluded_features:1}

    cargo hack --feature-powerset --exclude-features "$joined_excluded_features" "$@"
}

run_cargo_hack_no_std() {
    joined_excluded_features=$(printf ",%s" "${EXCLUDED_FEATURES_NO_STD[@]}")
    # Strip leading comma
    joined_excluded_features=${joined_excluded_features:1}

    cargo hack --feature-powerset --exclude-features "$joined_excluded_features" "$@"
}

while [[ "$#" -gt 0 ]]; do
    case $1 in
        b|build) run_build ;;
        t|test) run_test ;;
        build-no-std)
            shift;
            case $1 in
                --target) build_target="$2"; shift ;;
                "") echo_err "Usage: with-feature-powerset.sh build-no-std --target <target>"; exit 1 ;;
            esac

            if [[ -z "${build_target:-}" ]]; then
                echo_err "Usage: with-feature-powerset.sh build-no-std --target <target>"
                exit 1
            fi

            run_build_no_std "$build_target" ;;
        -h|--help) echo "Usage: with-feature-powerset.sh [b|build|t|test|test-no-std]"; exit 0 ;;
        *) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done
