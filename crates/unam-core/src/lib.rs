//! Core library for `unam`: configuration, sources, the catalog (scan,
//! frontmatter, cache, search, lookup), hashing, and atomic writes.

pub mod catalog;
pub mod config;
pub mod error;
pub mod fsops;
pub mod hash;
pub mod paths;
pub mod source;
