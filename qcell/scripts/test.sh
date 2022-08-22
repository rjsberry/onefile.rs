#!/usr/bin/env bash

set -e
trap 'echo -e "\033[36m${BASH_COMMAND}\033[0m"' DEBUG

cargo +stable test --verbose -p qcell -- --quiet
cargo +nightly miri test --verbose -p qcell -- --quiet

RUSTFLAGS="-Clink-args=-lc $RUSTFLAGS" cargo +stable run --verbose --profile nopanic -p qcell-nopanic --bin qcell_nopanic

RUSTFLAGS="-Zsanitizer=address $RUSTFLAGS" cargo +nightly run --verbose -p qcell-san --bin qcell_san
RUSTFLAGS="-Zsanitizer=thread $RUSTFLAGS" cargo +nightly run --verbose -p qcell-san --bin qcell_san
