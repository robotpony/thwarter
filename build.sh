#!/bin/sh

cargo build --release --target-dir ./target && cp ./target/release/thwart ./bin/