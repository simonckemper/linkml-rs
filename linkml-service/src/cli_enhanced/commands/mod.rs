//! Command implementations

// Note: The following command modules are integrated into the main app.rs
// for simplicity. In a larger application, these would be separate modules:
// mod convert;
// mod diff;
// mod dump;
// mod generate;
// mod lint;
// mod load;
// mod merge;

pub mod serve;

// pub use convert::ConvertCommand;
// pub use diff::DiffCommand;
// pub use dump::DumpCommand;
// pub use generate::GenerateCommand;
// pub use lint::LintCommand;
// pub use load::LoadCommand;
// pub use merge::MergeCommand;
pub use serve::ServeCommand;
