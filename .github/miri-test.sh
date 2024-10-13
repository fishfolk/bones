#!/usr/bin/env bash
#
# Run workspace tests with miri.
#
# This script is executed by the CI workflow in `.github/workflows/ci.yml`. Miri
# must be run with a nightly toolchain, but the workflow configures this with a
# rustup override rather than using rustup's `+toolchain` on the CLI. To run
# this script locally use the rustup env var:
#
#     RUSTUP_TOOLCHAIN=nightly ./github/miri-test.sh
#
# The number of seed runs can also be configured with the `NUM_SEEDS` env var:
#
#     NUM_SEEDS=1 RUSTUP_TOOLCHAIN=nightly ./github/miri-test.sh
#

line() {
    printf -- "${1}%0.s" $(seq "$2")
    printf '\n'
}

header() {
    line "$1" "$2"
    echo "$3"
    line "$1" "$2"
}

get_default_workspace_members() {
    cargo metadata --quiet --no-deps \
      | sed -nE 's/.*"workspace_default_members":\[([^]]+)\].*/\1/p' \
      | tr ',' '\n' | awk -F/ '{print gensub(/([^#]+).*/, "\\1", 1, $NF)}'
}

# The maximum number of seeds with which to run the tests
[ -z "$NUM_SEEDS" ] && NUM_SEEDS=10

# The crates to test
declare -a CRATES

if (( $# > 0 )); then
CRATES=( "$@" )
else
CRATES=( $(get_default_workspace_members) )
fi

# Extra flags to pass to `cargo test` for crates
declare -A FLAGS

FLAGS[bones_ecs]='--no-default-features -F miri'

# Try multiple seeds to catch possible alignment issues
for SEED in $(seq "$NUM_SEEDS"); do
    export MIRIFLAGS="-Zmiri-seed=$SEED"

    echo
    header '#' 80 "MIRI TEST WORKSPACE"
    echo

    for (( i=0; i<${#CRATES[@]}; i++ )); do
        NAME="${CRATES[i]}"
        header '-' 70 "TEST CRATE: $NAME (seed $SEED/$NUM_SEEDS, crate $(( i+1 ))/${#CRATES[@]})"
        echo
        eval "cargo miri test --package $NAME ${FLAGS[$NAME]}" || { echo "Failing seed: $SEED"; exit 1; };
    done
done
