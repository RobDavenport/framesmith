//! FSPK (Framesmith Pack) binary export adapter
//!
//! This module exports character data to the FSPK binary format.
//! The format is engine-agnostic and optimized for no_std/WASM runtimes.

mod builders;
mod export;
mod moves;
mod packing;
mod properties;
mod sections;
mod types;
mod utils;

pub use export::export_fspk;
