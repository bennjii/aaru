//! Identifier primitives for the Overture Maps codec.
//!
//! Overture features are keyed by **GERS** strings (Global Entity Reference
//! System), but the routing graph — and the [`Entry`] contract — is built on
//! `i64`. [`Interner`] maps each distinct GERS id to a unique, dense `i64`
//! during ingestion; the serialized graph stores only those `i64`s.
//!
//! Unlike hashing the string, interning is collision-free: two different GERS
//! ids can never map to the same [`OvertureEntryId`].

use core::hash::{Hash, Hasher};
use std::collections::HashMap;

use routers_network::Entry;
use serde::{Deserialize, Serialize};

/// Sentinel used for "no id"; never produced by the [`Interner`].
const OVERTURE_NULL_SENTINEL: i64 = -1;

/// A dense, interned identifier for an Overture connector or segment.
///
/// Transparent over an `i64` so it is cheap to copy, hash and store as a
/// `petgraph` node key.
#[derive(Clone, Copy, Debug, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct OvertureEntryId {
    pub identifier: i64,
}

impl OvertureEntryId {
    #[inline]
    pub const fn new(identifier: i64) -> Self {
        Self { identifier }
    }

    #[inline]
    pub const fn null() -> Self {
        Self {
            identifier: OVERTURE_NULL_SENTINEL,
        }
    }

    #[inline]
    pub const fn is_null(&self) -> bool {
        self.identifier == OVERTURE_NULL_SENTINEL
    }
}

impl Entry for OvertureEntryId {
    #[inline]
    fn identifier(&self) -> i64 {
        self.identifier
    }
}

impl Default for OvertureEntryId {
    fn default() -> Self {
        OvertureEntryId::null()
    }
}

impl From<i64> for OvertureEntryId {
    fn from(value: i64) -> Self {
        OvertureEntryId::new(value)
    }
}

impl PartialEq for OvertureEntryId {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier
    }
}

impl Hash for OvertureEntryId {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identifier.hash(state);
    }
}

/// Maps GERS strings to dense [`OvertureEntryId`]s during ingestion.
///
/// Identifiers start at `0` and increment; the `-1` null sentinel is never
/// produced, so it stays distinct from every interned id.
#[derive(Debug, Default)]
pub struct Interner {
    map: HashMap<String, i64>,
    next: i64,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the id for `gers`, allocating a fresh one on first sight.
    #[inline]
    pub fn intern(&mut self, gers: &str) -> OvertureEntryId {
        if let Some(&id) = self.map.get(gers) {
            return OvertureEntryId::new(id);
        }

        let id = self.next;
        self.next += 1;
        self.map.insert(gers.to_owned(), id);
        OvertureEntryId::new(id)
    }

    /// Returns the id for `gers` if it has already been interned.
    #[inline]
    pub fn get(&self, gers: &str) -> Option<OvertureEntryId> {
        self.map.get(gers).copied().map(OvertureEntryId::new)
    }

    /// Number of distinct ids interned so far.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
