//! Type system module
//!
//! This module handles type checking and type inference.

pub mod type_def;
pub mod checker;

pub use type_def::TypeInfo;
pub use checker::TypeChecker;

