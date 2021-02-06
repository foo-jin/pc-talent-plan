//! A simple key-value store backed by a Write Ahead Log. The commands
//! are serialized to the log using the
//! [bincode](https://github.com/servo/bincode) format. This format
//! was chosen because binary formats save space over textual formats,
//! and bincode is a highly reputable crate.

use crate::{KvsError, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Seek, SeekFrom, Write},
    path::PathBuf,
};

/// Factor by which the amount of redundant entries in the log
/// may exceed the amount of live entries before compaction.
const REDUNDANCY_THRESHOLD: u32 = 10;

#[derive(Serialize, Deserialize, Debug)]
enum Command {
    Set { key: String, value: String },
    Rm { key: String },
}

impl Command {
    fn set(key: String, value: String) -> Command {
        Command::Set { key, value }
    }

    fn remove(key: String) -> Command {
        Command::Rm { key }
    }
}

/// A key value store, backed by a Write Ahead Log.
#[derive(Debug)]
pub struct KvStore {
    home_path: PathBuf,
    log: File,
    index: HashMap<String, u64>,
    redundant: u32,
}

impl KvStore {
    /// Open the KvStore at the given path. Return the KvStore.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path = path.into();
        fs::create_dir_all(&path)?;

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
            use Command::*;
            match cmd {
                Set { key, .. } => {
                    index.insert(key, pos);
                }
                Rm { key } => {
                    if index.remove(&key).is_none() {
                        log::warn!(
                            "Log and index out of sync: missing key in index for Rm command."
                        );
                    }
                }
            };
        }

        Ok(KvStore {
            home_path: path,
            log,
            index,
            redundant: 0,
        })
    }

    /// Gets the string value of a string key. Returns `None` if the
    /// given key does not exist.
    ///
    /// # Errors
    ///
    /// Returns `KvsError::UnexpectedCommandType` if an
    /// unexpected command is found.
    pub fn get(&self, key: String) -> Result<Option<String>> {
        log::debug!("index: {:?}", self.index);
        match self.index.get(&key) {
            Some(pos) => {
                let mut reader = BufReader::new(&self.log);
                reader.seek(SeekFrom::Start(*pos))?;
                let logged_cmd: Command = bincode::deserialize_from(reader)?;
                use Command::*;
                match logged_cmd {
                    Set { value, .. } => Ok(Some(value)),
                    Rm { .. } => Err(KvsError::UnexpectedCommandType),
                }
            }
            None => Ok(None),
        }
    }

    /// Sets the value of a string key to a string. If the key already
    /// exists, the previous value will be overwritten.
    ///
    /// # Errors
    ///
    /// Errors encountered during I/O and serialization are
    /// propagated.
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        let pos = self.log.seek(SeekFrom::End(0))?;
        let old = self.index.insert(key.clone(), pos);

        let cmd = Command::set(key, value);
        bincode::serialize_into(&self.log, &cmd)?;

        if old.is_some() {
            self.redundant += 1;
            if self.redundancy() > REDUNDANCY_THRESHOLD {
                self.compact()?;
            }
        }
        Ok(())
    }

    /// Removes a given key.
    ///
    /// # Errors
    ///
    /// Returns `KvsError::NonExistentKey` if the given key is not
    /// found.
    ///
    /// Errors encountered during I/O or serialization are propagated.
    pub fn remove(&mut self, key: String) -> Result<()> {
        match self.index.remove(&key) {
            Some(_old) => {
                let cmd = Command::remove(key);
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

    /// Clears stale entries in the log.
    ///
    /// Compaction is carried out by creating a new log file, copying
    /// all the live commands as found in the index over to the new
    /// log, and replacing the old log with the new one.
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
        fs::rename(from, to)?;
        self.log = new_log;
        log::trace!("Compaction finished");
        Ok(())
    }
}
