//! Core library for `unam`: configuration, sources, the catalog (scan,
//! frontmatter, cache, search, lookup), bundles, projects (root detection,
//! selection, index), resolution, hashing, and atomic writes.

pub mod bundle;
pub mod catalog;
pub mod config;
pub mod error;
pub mod fsops;
pub mod hash;
pub mod paths;
pub mod project;
pub mod resolve;
pub mod source;
