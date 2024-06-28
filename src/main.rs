use clap::{Parser, Subcommand};
use kvstore::ActionKV;
use std::io::Result;
use std::path::Path;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    action: Actions,
    /// Database file name
    database: String,
}

#[derive(Subcommand)]
enum Actions {
    /// Delete a value from the database
    Delete {
        /// The key for the key/value pair.
        key: String,
    },
    /// Get a value from the database
    Get {
        /// The key for the key/value pair.
        key: String,
    },
    /// Insert a value into the database
    Insert {
        /// The key for the key/value pair.
        key: String,
        /// The value for the key/value pair.
        value: String,
    },
    /// Update a value in the database
    Update {
        /// The key for the key/value pair.
        key: String,
        /// The value for the key/value pair.
        value: String,
    },
}

// entry point
fn main() -> Result<()> {
    let args = Cli::parse();

    let path = Path::new(&args.database);
    let mut akv = ActionKV::open(path)?;

    match args.action {
        Actions::Delete { key } => akv.delete(key)?,
        Actions::Get { key } => {
            let value = akv.get(key)?;
            println!("{value}");
        }
        Actions::Insert { key, value } => akv.insert(key, value)?,
        Actions::Update { key, value } => akv.update(key, value)?,
    }

    Ok(())
}
