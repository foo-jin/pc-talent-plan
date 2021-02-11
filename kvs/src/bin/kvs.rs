use std::process;

use clap::Clap;

const KEY_NOT_FOUND: &str = "Key not found";

#[derive(Clap)]
#[clap(name = env!("CARGO_PKG_NAME"),
	   version = env!("CARGO_PKG_VERSION"),
	   author = env!("CARGO_PKG_AUTHORS"),
       about = env!("CARGO_PKG_DESCRIPTION"))]
struct Cli {
    /// The path where the key-value store should store its data.
    #[clap(parse(from_os_str), default_value = ".")]
    path: std::path::PathBuf,
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Clap)]
enum Command {
    /// Gets the value corresponding to <key> in the key-value store.
    Get { key: String },
    /// Removes the entry corresponding to <key> from the key-value store.
    Rm { key: String },
    /// Set the value corresponding to <key> in the key-value store to <value>.
    Set { key: String, value: String },
}

fn main() -> kvs::Result<()> {
    let cli: Cli = Cli::parse();
    let mut store = kvs::KvStore::open(cli.path)?;

    use Command::*;
    match cli.cmd {
        Get { key } => {
            let msg = store.get(key)?.unwrap_or_else(|| KEY_NOT_FOUND.to_owned());
            println!("{}", msg);
        }
        Rm { key } => {
            let result = store.remove(key);
            match result {
                Ok(()) => (),
                Err(kvs::KvsError::NonExistentKey(_)) => {
                    println!("{}", KEY_NOT_FOUND);
                    process::exit(1);
                }
                Err(e) => return Err(e),
            }
        }
        Set { key, value } => {
            store.set(key, value)?;
        }
    };
    Ok(())
}
