//! A simple key-value store backed by a Write Ahead Log. The commands
//! are serialized to the log using the
//! [MsgPack](https://github.com/3Hren/msgpack-rust) format.

use crate::{
    io::{BufReaderWithPos, BufWriterWithPos},
    KvsError, Result,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
    path::PathBuf,
};

/// Amount of "wasted" bytes before a compaction is triggered after an operation.
const COMPACTION_THRESHOLD: u64 = 1024 * 1024;

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

/// The position and length of a serialized command in the log.
#[derive(Copy, Clone, Debug)]
struct CommandPos {
    pos: u64,
    len: u64,
}

impl From<Range<u64>> for CommandPos {
    fn from(range: Range<u64>) -> Self {
        CommandPos {
            pos: range.start,
            len: range.end - range.start,
        }
    }
}

/// The `KvStore` stores string key/value pairs.
///
/// Key/value pairs are persisted to disk in log file(s). The log file
/// is named 'kvs.log' or 'kvs.log.new' if compaction is in progress.
/// A `BTreeMap` in memory stores the keys and the value locations for
/// fast query.
///
/// ```rust
/// # use kvs::{KvStore, Result};
/// # fn try_main() -> Result<()> {
/// use std::env::current_dir;
/// let mut store = KvStore::open(current_dir()?)?;
/// store.set("key".to_owned(), "value".to_owned())?;
/// let val = store.get("key".to_owned())?;
/// assert_eq!(val, Some("value".to_owned()));
/// # Ok(())
/// # }
/// ```
pub struct KvStore {
    // directory for the log data
    path: PathBuf,
    // log file reader
    reader: BufReaderWithPos<File>,
    // log file writer
    writer: BufWriterWithPos<File>,
    index: BTreeMap<String, CommandPos>,
    // number of bytes occupied by "stale" commands that could be
    // deleted during a compaction.
    uncompacted: u64,
}

impl KvStore {
    /// Opens a `KvStore` with the given path.
    ///
    /// This will create a new directory if the given one does not exist.
    ///
    /// # Errors
    ///
    /// It propagates I/O or deserialization errors during the log replay.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let dir = path.into();
        fs::create_dir_all(&dir)?;

        let path = dir.join("kvs.log");
        let log = OpenOptions::new().create(true).write(true).open(&path)?;
        let mut reader = BufReaderWithPos::new(File::open(&path)?)?;
        let mut index = BTreeMap::new();
        let uncompacted = load(&mut reader, &mut index)?;

        let mut writer = BufWriterWithPos::new(log)?;
        writer.seek(SeekFrom::End(0))?;

        Ok(KvStore {
            path: dir,
            reader,
            writer,
            index,
            uncompacted,
        })
    }

    /// Gets the string value of a string key. Returns `None` if the
    /// given key does not exist.
    ///
    /// # Errors
    ///
    /// Returns `KvsError::UnexpectedCommandType` if an
    /// unexpected command is found.
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        match self.index.get(&key) {
            Some(cmd_pos) => {
                let reader = &mut self.reader;
                reader.seek(SeekFrom::Start(cmd_pos.pos))?;
                let mut cmd_reader = reader.take(cmd_pos.len);
                if let Command::Set { value, .. } = rmp_serde::from_read(&mut cmd_reader)? {
                    Ok(Some(value))
                } else {
                    Err(KvsError::UnexpectedCommandType)
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
        let pos = self.writer.pos();
        // let old = self.index.insert(key.clone(), pos);
        let cmd = Command::set(key, value);
        rmp_serde::encode::write(&mut self.writer, &cmd)?;
        self.writer.flush()?;

        if let Command::Set { key, .. } = cmd {
            if let Some(old_cmd) = self.index.insert(key, (pos..self.writer.pos()).into()) {
                self.uncompacted += old_cmd.len;
            }
        } else {
            unreachable!()
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
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
        let pos = self.writer.pos();
        match self.index.remove(&key) {
            Some(old_cmd) => {
                let cmd = Command::remove(key);
                rmp_serde::encode::write(&mut self.writer, &cmd)?;
                self.writer.flush()?;

                let new_pos = self.writer.pos();
                self.uncompacted += new_pos - pos;
                self.uncompacted += old_cmd.len;
                if self.uncompacted > COMPACTION_THRESHOLD {
                    self.compact()?;
                }

                Ok(())
            }
            None => return Err(KvsError::NonExistentKey(key)),
        }
    }

    /// Clears stale entries in the log.
    ///
    /// Compaction is carried out by creating a new log file, copying
    /// all the live commands as found in the index over to the new
    /// log, and replacing the old log with the new one.
    fn compact(&mut self) -> Result<()> {
        log::trace!("Starting compaction...");
        log::trace!("Index size: {}", self.index.len());
        log::trace!("Uncompacted: {}", self.uncompacted);

        let new_path = self.path.join("new.log");
        dbg!(&new_path);
        let new_log = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&new_path)?;

        let mut compaction_writer = BufWriterWithPos::new(new_log)?;
        for cmd_pos in self.index.values_mut() {
            let pos = cmd_pos.pos;
            if self.reader.pos() != pos {
                self.reader.seek(SeekFrom::Start(pos))?;
            }
            let start = compaction_writer.pos();
            let reader = &mut self.reader;
            let mut entry_reader = reader.take(cmd_pos.len);
            let len = std::io::copy(&mut entry_reader, &mut compaction_writer)?;

            *cmd_pos = (start..start + len).into();
        }
        compaction_writer.flush()?;

        let from = new_path;
        let to = self.path.join("kvs.log");
        fs::rename(from, &to)?;
        self.writer = compaction_writer;
        self.reader = BufReaderWithPos::new(File::open(to)?)?;
        log::trace!("Compaction finished");
        Ok(())
    }
}

/// Load the whole log file and store value locations in the index map.
///
/// Returns how many bytes can be saved after a compaction.
fn load(
    mut reader: &mut BufReaderWithPos<File>,
    index: &mut BTreeMap<String, CommandPos>,
) -> Result<u64> {
    let mut uncompacted = 0;
    let end = reader.seek(SeekFrom::End(0))?;
    let mut pos = reader.seek(SeekFrom::Start(0))?;

    loop {
        if pos >= end {
            return Ok(uncompacted);
        }

        let cmd: Command = rmp_serde::from_read(&mut reader)?;
        let new_pos = reader.pos();

        use Command::*;
        match cmd {
            Set { key, .. } => {
                index.insert(key, (pos..new_pos).into());
            }
            Rm { key } => {
                if let Some(old_cmd) = index.remove(&key) {
                    uncompacted += old_cmd.len;
                } else {
                    log::warn!("log out of sync: missing key in index for remove command.");
                }
                uncompacted += new_pos - pos;
            }
        };
        pos = new_pos;
    }
}
