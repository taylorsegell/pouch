//! Core library for `unam`: configuration, sources, the catalog (scan,
//! frontmatter, cache, search, lookup), bundles, projects (root detection,
//! selection, index), resolution, hashing, typed filesystem operations,
//! adapters, observation, project state, planning, apply, and change
//! orchestration.

pub mod adapter;
pub mod apply;
pub mod bundle;
pub mod catalog;
pub mod change;
pub mod config;
pub mod doctor;
pub mod error;
pub mod fsops;
pub mod hash;
pub mod observe;
pub mod paths;
pub mod plan;
pub mod project;
pub mod reconcile;
pub mod resolve;
pub mod source;
pub mod state;
pub mod verify;
