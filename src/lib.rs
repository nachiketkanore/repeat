// Declare the modules you want to make available internally/externally
mod analyzer;
mod config;
mod execution;
mod runner;
mod utils;

// Export the AnalysisTracker struct and any other items the tests need.
// Assuming AnalysisTracker is defined in analyzer.rs:
pub use analyzer::AnalysisTracker;
