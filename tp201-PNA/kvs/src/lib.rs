#![deny(missing_docs)]
//! A simple in-memory key-value store.

use std::collections::HashMap;

/// A key value store, backed by a `HashMap<String, String>`.
#[derive(Clone, Debug, Default)]
pub struct KvStore {
    map: HashMap<String, String>,
}

impl KvStore {
    /// Constructs a new, empty `KVStore`
    pub fn new() -> KvStore {
        KvStore::default()
    }

    /// Returns a copy of the value corresponding to the supplied key,
    /// if it is present in the map.
    pub fn get(&self, key: String) -> Option<String> {
        self.map.get(&key).map(String::to_owned)
    }

    /// Sets the `val` as the value corresponding to `key` in the store.
    pub fn set(&mut self, key: String, val: String) {
        self.map.insert(key, val);
    }

    /// Removes the value corresponding to the supplied key.
    pub fn remove(&mut self, key: String) {
        self.map.remove(&key);
    }
}
