#!/usr/bin/env bash

set -eu
trap 'echo -e "\033[36m${BASH_COMMAND}\033[0m"' DEBUG

cargo +stable test -q -p qptr
cargo +nightly miri test -q -p qptr

RUSTFLAGS="-C link-args=-lc" cargo +stable run -q --profile nopanic -p qptr-nopanic --bin qptr_shared_nopanic
RUSTFLAGS="-C link-args=-lc" cargo +stable run -q --profile nopanic -p qptr-nopanic --bin qptr_unique_nopanic

RUSTFLAGS="-Z sanitizer=address" cargo +nightly run -q -p qptr-san --bin qptr_shared_san
RUSTFLAGS="-Z sanitizer=address" cargo +nightly run -q -p qptr-san --bin qptr_unique_san
