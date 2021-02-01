#![deny(missing_docs)]
//! A simple in-memory key-value store.

use std::{collections::HashMap, path::PathBuf};

/// All errors that can be encountered by the KvStore.
#[derive(Debug)]
pub struct KvsError;

/// Convenience alias for results.
pub type Result<T> = std::result::Result<T, KvsError>;

/// A key value store, backed by a `HashMap<String, String>`.
#[derive(Clone, Debug, Default)]
pub struct KvStore {
    map: HashMap<String, String>,
}

impl KvStore {
    /// Open the KvStore at the given path. Return the KvStore.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        Ok(KvStore::default())
    }

    /// Get the string value of a string key. If the key does not exist, return None.
    /// Return an error if the value is not read successfully.
    pub fn get(&self, key: String) -> Result<Option<String>> {
        let value = self.map.get(&key).map(String::to_owned);
        Ok(value)
    }

    /// Set the value of a string key to a string.
    /// Return an error if the value is not written successfully.
    pub fn set(&mut self, key: String, val: String) -> Result<()> {
        self.map.insert(key, val);
        Ok(())
    }

    /// Removes the value corresponding to the supplied key.
    pub fn remove(&mut self, key: String) -> Result<()> {
        self.map.remove(&key);
        Ok(())
    }
}
