// Main module file for TA-Lib bindings
mod ffi;
mod common;
mod oscillators;
mod overlaps;
mod patterns;
mod volume;
mod volatility;

// Re-export the main interface
pub use common::TaLibAbstract;
pub use ffi::TA_SUCCESS;
