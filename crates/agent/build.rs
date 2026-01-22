//! Build script for claude-tunnel-agent
//!
//! Uses thunk-rs on Windows to embed runtime thunks for better compatibility.

use std::env;

fn main() {
    // Only apply thunk on Windows builds
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        thunk::thunk();
    }
}
