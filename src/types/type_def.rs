//! Type definitions
//!
//! This module defines the type system for Lux.

/// Type information
#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    Int,
    Float,
    String,
    Bool,
    Nil,
    // To be expanded
}

