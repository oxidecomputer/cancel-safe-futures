#!/bin/bash

set -e -o pipefail

# Run commands to build and test with a feature powerset. Requires `cargo-hack` to be installed.

CARGO="${CARGO:-cargo}"
EXCLUDED_FEATURES=(internal-docs)
EXCLUDED_FEATURES_NO_STD=(std internal-docs default)

trap 'echo_err "Error occurred at $0 command: $BASH_COMMAND"' ERR
trap 'echo_err "Exiting $0"' EXIT

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
}

run_nextest() {
    check_cargo_hack

    echo_err "Running non-doc tests"
    run_cargo_hack nextest run --all-targets
}

run_miri() {
    echo_err "Running miri"
    run_cargo_std miri nextest run --all-targets -j2
}

run_doctest() {
    echo_err "Running doctests with all features"
    cargo test --all-features --doc
}

run_coverage() {
    echo_err "Running coverage (requires nightly)"
    run_cargo_std llvm-cov nextest --all-targets --lcov --output-path lcov.info
    run_cargo_std llvm-cov test --doc --lcov --output-path lcov-doctest.info
    echo_err "Wrote output to lcov.info and lcov-doctest.info"
}

run_build_no_std() {
    local build_target="$1"

    check_cargo_hack

    echo_err "Building for no-std target ${build_target}"
    run_cargo_hack_no_std build --target "${build_target}"
}

echo_err() {
    echo "$@" >&2
}

check_cargo_hack() {
    if ! $CARGO hack --version >/dev/null 2>&1; then
        echo_err "cargo-hack not installed. Install it with:"
        echo_err "    cargo install cargo-hack"
        exit 1
    fi
}

run_cargo_hack() {
    joined_excluded_features=$(printf ",%s" "${EXCLUDED_FEATURES[@]}")
    # Strip leading comma
    joined_excluded_features=${joined_excluded_features:1}

    $CARGO hack --feature-powerset --exclude-features "$joined_excluded_features" "$@"
}

run_cargo_hack_no_std() {
    joined_excluded_features=$(printf ",%s" "${EXCLUDED_FEATURES_NO_STD[@]}")
    # Strip leading comma
    joined_excluded_features=${joined_excluded_features:1}

    $CARGO hack --feature-powerset --exclude-features "$joined_excluded_features" "$@"
}

run_cargo_std() {
    $CARGO "$@" --features std
}

if [[ $# -eq 0 ]]; then
    echo_err "Usage: commands.sh [b|build|t|test|nt|nextest|dt|doctest|miri|coverage|build-no-std]"
    exit 1
fi

while [[ "$#" -gt 0 ]]; do
    case $1 in
        +*) CARGO="$CARGO $1" ;;
        b|build) run_build ;;
        t|test) run_test ;;
        nt|nextest) run_nextest ;;
        miri) run_miri ;;
        coverage) run_coverage ;;
        dt|doctest) run_doctest ;;
        build-no-std)
            shift;
            case $1 in
                --target) build_target="$2"; shift ;;
                "") echo_err "Usage: commands.sh build-no-std --target <target>"; exit 1 ;;
            esac

            if [[ -z "${build_target:-}" ]]; then
                echo_err "Usage: commands.sh build-no-std --target <target>"
                exit 1
            fi

            run_build_no_std "$build_target" ;;
        -h|--help) echo "Usage: commands.sh [b|build|t|test|nt|nextest|dt|doctest|miri|coverage|build-no-std]"; exit 0 ;;
        *) echo "Unknown parameter passed: $1"; exit 1 ;;
    esac
    shift
done
