//! Per-source catalog cache (spec 4.3, DIS-9): `<cache dir>/catalog/<alias>.json`.
//!
//! A cache, not a user contract. Any invalid, corrupt, wrong-version, or
//! relocated cache is a silent miss.

use super::scan::{FingerprintEntry, ScannedSkill};
use super::{RelPath, Skill, SkillId};
use crate::error::{Diagnostic, Result};
use crate::fsops::write_atomic;
use crate::hash::Hash;
use crate::paths::Dirs;
use crate::source::SourceAlias;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CACHE_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    alias: String,
    source_path: String,
    scanned_at: String,
    skills: Vec<CachedSkill>,
}

/// A serialized `ScannedSkill`: the `Skill` fields plus its fingerprint.
#[derive(Debug, Serialize, Deserialize)]
struct CachedSkill {
    rel_path: String,
    name: String,
    description: String,
    frontmatter: serde_json::Map<String, serde_json::Value>,
    supporting: Vec<String>,
    tree_hash: Hash,
    diagnostics: Vec<Diagnostic>,
    fingerprint: Vec<FingerprintEntry>,
}

/// A valid cache read: prior skills by relative path, and when it was written.
#[derive(Debug, Clone)]
pub struct CacheHit {
    pub prior: HashMap<String, ScannedSkill>,
    pub scanned_at: String,
}

/// Read the cache for `alias`; `None` on any kind of miss.
pub fn read(dirs: &Dirs, alias: &SourceAlias, source_path: &str) -> Option<CacheHit> {
    let text = std::fs::read_to_string(dirs.catalog_cache_file(alias.as_str())).ok()?;
    let file: CacheFile = serde_json::from_str(&text).ok()?;
    if file.version != CACHE_VERSION
        || file.alias != alias.as_str()
        || file.source_path != source_path
    {
        return None;
    }
    let mut prior = HashMap::new();
    for c in file.skills {
        let rel_path = RelPath::parse(&c.rel_path).ok()?;
        let category = rel_path.parent_str().to_string();
        let skill = Skill {
            id: SkillId {
                source: alias.clone(),
                rel_path,
            },
            name: c.name,
            description: c.description,
            category,
            frontmatter: c.frontmatter,
            supporting: c.supporting,
            tree_hash: c.tree_hash,
            diagnostics: c.diagnostics,
        };
        prior.insert(
            c.rel_path,
            ScannedSkill {
                skill,
                fingerprint: c.fingerprint,
            },
        );
    }
    Some(CacheHit {
        prior,
        scanned_at: file.scanned_at,
    })
}

/// Write the cache for `alias` atomically.
pub fn write(
    dirs: &Dirs,
    alias: &SourceAlias,
    source_path: &str,
    scanned_at: &str,
    skills: &[ScannedSkill],
) -> Result<()> {
    let file = CacheFile {
        version: CACHE_VERSION,
        alias: alias.to_string(),
        source_path: source_path.to_string(),
        scanned_at: scanned_at.to_string(),
        skills: skills
            .iter()
            .map(|s| CachedSkill {
                rel_path: s.skill.id.rel_path.to_string(),
                name: s.skill.name.clone(),
                description: s.skill.description.clone(),
                frontmatter: s.skill.frontmatter.clone(),
                supporting: s.skill.supporting.clone(),
                tree_hash: s.skill.tree_hash.clone(),
                diagnostics: s.skill.diagnostics.clone(),
                fingerprint: s.fingerprint.clone(),
            })
            .collect(),
    };
    let bytes = serde_json::to_vec_pretty(&file).map_err(|e| {
        crate::error::Error::io("cannot serialize catalog cache", std::io::Error::other(e))
    })?;
    write_atomic(&dirs.catalog_cache_file(alias.as_str()), &bytes)
}
