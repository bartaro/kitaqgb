#!/bin/sh
set -eu
project_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cargo build --locked --release --manifest-path "$project_root/Cargo.toml" --bin kitaqgb-zx0
