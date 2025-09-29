#!/bin/bash

cargo build --release && perf record --call-graph dwarf target/release/$(basename $(pwd))
