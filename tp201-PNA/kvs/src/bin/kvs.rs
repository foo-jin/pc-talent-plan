use std::process;

use clap::Clap;

#[derive(Clap)]
#[clap(name = env!("CARGO_PKG_NAME"),
	   version = env!("CARGO_PKG_VERSION"),
	   author = env!("CARGO_PKG_AUTHORS"),
       about = env!("CARGO_PKG_DESCRIPTION"))]
enum Cli {
    /// Gets the value corresponding to <key> in the key-value store.
    Get { key: String },
    /// Removes the entry corresponding to <key> from the key-value store.
    Rm { key: String },
    /// Set the value corresponding to <key> in the key-value store to <value>.
    Set { key: String, value: String },
}

fn main() {
    let cli: Cli = Cli::parse();
    let store = kvs::KvStore::new();

    use Cli::*;
    match cli {
        Get { key } => {
            eprintln!("unimplemented");
            process::exit(1);
        }
        Rm { key } => {
            eprintln!("unimplemented");
            process::exit(1);
        }
        Set { key, value } => {
            eprintln!("unimplemented");
            process::exit(1);
        }
    }
}
