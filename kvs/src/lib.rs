#![deny(missing_docs)]
//! A simple key-value store backed by a Write Ahead Log. The commands
//! are serialized to the log using the [bincode](https://github.com/servo/bincode)
//! format. This format was chosen because binary formats save space over
//! textual formats, and bincode is a highly reputable crate.

use io::{BufWriter, SeekFrom};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, BufReader, Seek, Write},
    path::PathBuf,
};
use thiserror::Error;

/// Factor by which the amount of redundant entries in the log
/// may exceed the amount of live entries before compaction.
const REDUNDANCY_THRESHOLD: u32 = 10;

/// Convenience alias for `Result<T, KvsError>`.
pub type Result<T> = std::result::Result<T, KvsError>;

/// All errors that can be encountered by the KvStore.
#[derive(Error, Debug)]
pub enum KvsError {
    /// IO error
    #[error("IO error")]
    Io(#[from] io::Error),
    /// (De)Serialize error
    #[error("(De)Serialize error")]
    Serde(#[from] bincode::Error),
    /// Indicates invocation of an command with a non-existent key.
    #[error("No such key: `{0}`")]
    NonExistentKey(String),
}

/// A key value store, backed by a Write Ahead Log.
#[derive(Debug)]
pub struct KvStore {
    home_path: PathBuf,
    log: File,
    index: HashMap<String, u64>,
    redundant: u32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Command {
    key: String,
    kind: CommandKind,
}

#[derive(Serialize, Deserialize, Debug)]
enum CommandKind {
    Set(String),
    Get,
    Rm,
}

impl KvStore {
    /// Open the KvStore at the given path. Return the KvStore.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path = path.into();
        let log_path = path.join("log");
        let log = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(log_path)?;

        let mut reader = BufReader::new(&log);
        let mut index = HashMap::new();
        let end = reader.seek(SeekFrom::End(0))?;
        let _ = reader.seek(SeekFrom::Start(0))?;
        loop {
            let pos = reader.seek(SeekFrom::Current(0))?;
            if pos >= end {
                break;
            }

            let cmd: Command = bincode::deserialize_from(&mut reader)?;
            use CommandKind::*;
            match cmd.kind {
                Set(_) => {
                    index.insert(cmd.key, pos);
                }
                Rm => {
                    if index.remove(&cmd.key).is_none() {
                        log::warn!(
                            "Log and index out of sync: missing key in index for Rm command."
                        );
                    }
                }
                Get => unreachable!(),
            };
        }

        Ok(KvStore {
            home_path: path,
            log,
            index,
            redundant: 0,
        })
    }

    /// Get the string value of a string key. If the key does not exist, return None.
    /// Return an error if the value is not read successfully.
    pub fn get(&self, key: String) -> Result<Option<String>> {
        log::debug!("index: {:?}", self.index);
        match self.index.get(&key) {
            Some(pos) => {
                let mut reader = BufReader::new(&self.log);
                reader.seek(SeekFrom::Start(*pos))?;
                let logged_cmd: Command = bincode::deserialize_from(reader)?;
                use CommandKind::*;
                match logged_cmd.kind {
                    Set(val) => Ok(Some(val)),
                    Get | Rm => panic!(
							"Internal index is in an invalid state: encountered {:?} while retrieving latest Set command",
							logged_cmd.kind
						),
                }
            }
            None => Ok(None),
        }
    }

    /// Set the value of a string key to a string.
    /// Return an error if the value is not written successfully.
    pub fn set(&mut self, key: String, val: String) -> Result<()> {
        let pos = self.log.seek(SeekFrom::End(0))?;
        let old = self.index.insert(key.clone(), pos);

        let cmd = Command {
            key,
            kind: CommandKind::Set(val),
        };
        bincode::serialize_into(&self.log, &cmd)?;

        if old.is_some() {
            self.redundant += 1;
            if self.redundancy() > REDUNDANCY_THRESHOLD {
                self.compact()?;
            }
        }
        Ok(())
    }

    /// Removes the value corresponding to the supplied key.
    pub fn remove(&mut self, key: String) -> Result<()> {
        match self.index.remove(&key) {
            Some(_old) => {
                let cmd = Command {
                    key,
                    kind: CommandKind::Rm,
                };
                bincode::serialize_into(&self.log, &cmd)?;

                self.redundant += 2;
                if self.redundancy() > REDUNDANCY_THRESHOLD {
                    self.compact()?;
                }

                Ok(())
            }
            None => return Err(KvsError::NonExistentKey(key)),
        }
    }

    fn redundancy(&self) -> u32 {
        let divisor = u32::max(self.index.len() as u32, 1);
        self.redundant / divisor
    }

    /// Compaction is carried out by creating a new log file,
    /// copying all the live commands as found in the index over to the new log,
    /// and replacing the old log with the new one.
    fn compact(&mut self) -> Result<()> {
        log::trace!("Start compaction, index size: {}", self.index.len());
        let new_log = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(self.home_path.join("log.new"))?;

        {
            let mut writer = BufWriter::new(&new_log);
            let mut index: Vec<(String, u64)> = self.index.drain().collect();
            index.sort();
            for (key, pos) in index {
                self.log.seek(SeekFrom::Start(pos))?;
                let new_pos = writer.seek(SeekFrom::Current(0))?;
                let cmd: Command = bincode::deserialize_from(&self.log)?;
                bincode::serialize_into(&mut writer, &cmd)?;
                self.index.insert(key, new_pos);
            }
            writer.flush()?;
        }

        let from = self.home_path.join("log.new");
        let to = self.home_path.join("log");
        std::fs::rename(from, to)?;
        self.log = new_log;
        log::trace!("Compaction finished");
        Ok(())
    }
}
