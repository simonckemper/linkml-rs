//! Excel generator for `LinkML` schemas.
//!
//! This module provides the Excel workbook generator that was previously
//! implemented as a single monolithic file. It has been split into focused
//! submodules to keep each file well under the 500 line guideline while
//! preserving functionality.

mod cast;
mod features;
mod generator;
mod pattern;
mod sheets;
mod workbook;

pub use features::ExcelFeatures;
pub use generator::ExcelGenerator;
