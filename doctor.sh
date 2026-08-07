#!/usr/bin/env bash

set -uo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
FULL_CHECK=0

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
    COLOR_BLUE='\033[34m'
    COLOR_GREEN='\033[32m'
    COLOR_YELLOW='\033[33m'
    COLOR_RED='\033[31m'
    COLOR_RESET='\033[0m'
else
    COLOR_BLUE=''
    COLOR_GREEN=''
    COLOR_YELLOW=''
    COLOR_RED=''
    COLOR_RESET=''
fi

usage() {
    /usr/bin/printf '%s\n' \
        'Usage: ./doctor.sh [--full]' \
        '' \
        'Checks the host toolchain, locked dependencies, and debug build.' \
        'Use --full to also run formatting, Clippy, tests, and the release build.'
}

while (( $# > 0 )); do
    case "$1" in
        --full)
            FULL_CHECK=1
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            /usr/bin/printf 'Unknown option: %s\n\n' "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

info() {
    /usr/bin/printf '%b==>%b %s\n' "$COLOR_BLUE" "$COLOR_RESET" "$1"
}

pass() {
    /usr/bin/printf '%b[ok]%b %s\n' "$COLOR_GREEN" "$COLOR_RESET" "$1"
}

warn() {
    /usr/bin/printf '%b[warn]%b %s\n' "$COLOR_YELLOW" "$COLOR_RESET" "$1"
}

fail() {
    /usr/bin/printf '%b[fail]%b %s\n' "$COLOR_RED" "$COLOR_RESET" "$1" >&2
}

check_command() {
    local command_name="$1"

    if command -v "$command_name" >/dev/null 2>&1; then
        pass "$command_name: $(command -v "$command_name")"
        return 0
    fi

    fail "$command_name is not installed or is not on PATH"
    return 1
}

run_check() {
    local label="$1"
    shift

    info "$label"
    if "$@"; then
        pass "$label"
        return 0
    fi

    fail "$label"
    exit 1
}

version_at_least() {
    local actual="${1%%-*}"
    local required="${2%%-*}"
    local actual_major=0
    local actual_minor=0
    local actual_patch=0
    local required_major=0
    local required_minor=0
    local required_patch=0

    IFS=. read -r actual_major actual_minor actual_patch <<< "$actual"
    IFS=. read -r required_major required_minor required_patch <<< "$required"

    actual_minor="${actual_minor:-0}"
    actual_patch="${actual_patch:-0}"
    required_minor="${required_minor:-0}"
    required_patch="${required_patch:-0}"

    (( actual_major > required_major )) ||
        (( actual_major == required_major && actual_minor > required_minor )) ||
        (( actual_major == required_major && actual_minor == required_minor && actual_patch >= required_patch ))
}

check_metadata() {
    cargo metadata --format-version 1 --no-deps --locked >/dev/null
}

check_binary() {
    local binary_path="$1"

    [[ -x "$binary_path" ]] && "$binary_path" --version
}

cd -- "$ROOT_DIR" || {
    fail "cannot enter repository root: $ROOT_DIR"
    exit 1
}

info "Required commands"
missing_commands=0
required_commands=(git rustup cargo rustc rustfmt rust-analyzer)

case "$(uname -s)" in
    Darwin | Linux)
        required_commands+=(cc make perl)
        ;;
esac

for command_name in "${required_commands[@]}"; do
    if ! check_command "$command_name"; then
        missing_commands=$((missing_commands + 1))
    fi
done

if (( missing_commands > 0 )); then
    fail "$missing_commands required command(s) missing"
    /usr/bin/printf '%s\n' \
        'Install Rust with rustup and the native build tools for your platform, then rerun doctor.' >&2
    exit 1
fi

info "Toolchain"
rustc_version_line="$(rustc --version)"
rustc_version="${rustc_version_line#rustc }"
rustc_version="${rustc_version%% *}"
required_rust="$(awk -F'"' '/^rust-version = / { print $2; exit }' Cargo.toml)"

pass "$rustc_version_line"
pass "$(cargo --version)"
pass "$(git --version)"
pass "active toolchain: $(rustup show active-toolchain)"

if [[ -z "$required_rust" ]]; then
    fail "Cargo.toml does not declare rust-version"
    exit 1
fi

if version_at_least "$rustc_version" "$required_rust"; then
    pass "Rust $rustc_version satisfies project minimum $required_rust"
else
    fail "Rust $rustc_version is older than project minimum $required_rust"
    exit 1
fi

run_check "rustfmt component" rustfmt --version
run_check "Clippy component" cargo clippy --version
run_check "rust-analyzer component" rust-analyzer --version

if cargo audit --version >/dev/null 2>&1; then
    pass "cargo-audit is available for security scans"
else
    warn "cargo-audit is optional; install with: cargo install cargo-audit --locked"
fi

run_check "Cargo.lock resolves without changes" check_metadata
run_check "all targets compile" cargo check --all-targets --all-features --locked
run_check "debug binary links" cargo build --locked
run_check "debug binary starts" check_binary "$ROOT_DIR/target/debug/git-monitor"

if (( FULL_CHECK == 1 )); then
    run_check "formatting" cargo fmt --all -- --check
    run_check "Clippy" cargo clippy --all-targets --all-features -- -D warnings
    run_check "test suite" cargo test --all-targets --all-features --locked
    run_check "release binary links" cargo build --release --locked
    run_check "release binary starts" check_binary "$ROOT_DIR/target/release/git-monitor"
fi

/usr/bin/printf '\n%bReady.%b ' "$COLOR_GREEN" "$COLOR_RESET"
if (( FULL_CHECK == 1 )); then
    /usr/bin/printf '%s\n' 'All local quality gates passed.'
else
    /usr/bin/printf '%s\n' 'The development toolchain and debug build are healthy.'
    /usr/bin/printf '%s\n' 'Run ./doctor.sh --full before publishing changes.'
fi
