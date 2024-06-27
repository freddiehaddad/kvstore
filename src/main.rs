#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

// command line argument parsing
use clap::{Parser, Subcommand};
use std::io::{Result, Write};
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

// database
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::ErrorKind::InvalidData;
use std::io::{BufReader, BufWriter, Error, Read, Seek, SeekFrom};

struct KeyValuePair {
    key: String,
    value: String,
}

struct ActionKV {
    file: File,
    database: HashMap<String, u64>,
}

impl ActionKV {
    fn open(path: &Path) -> Result<ActionKV> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;
        let database = HashMap::new();
        let mut akv = ActionKV { file, database };
        akv.load()?;
        Ok(akv)
    }

    fn delete(&mut self, key: String) -> Result<()> {
        let position = match self.database.get(&key) {
            Some(position) => position,
            None => {
                let error_message = format!("key: {key} not found in database");
                let error = Error::new(InvalidData, error_message);
                return Err(error);
            }
        };

        let value = String::new();
        let _ = self.insert_in_database(&key, &value);
        Ok(())
    }

    fn get(&self, key: String) -> Result<String> {
        let position = match self.database.get(&key) {
            Some(position) => position,
            None => {
                let error_message = format!("key: {key} not found in database");
                let error = Error::new(InvalidData, error_message);
                return Err(error);
            }
        };

        let akv = self.get_record_at_position(*position)?;
        Ok(akv.value)
    }

    fn insert(&mut self, key: String, value: String) -> Result<()> {
        let position = self.insert_in_database(&key, &value)?;
        self.database.insert(key, position);
        Ok(())
    }

    fn update(&mut self, key: String, value: String) -> Result<()> {
        self.insert(key, value)?;
        Ok(())
    }

    fn load(&mut self) -> Result<()> {
        let mut file = std::io::BufReader::new(&self.file);

        loop {
            let current_position = file.stream_position()?;
            let maybe_kv = ActionKV::process_record(&mut file);
            let kv = match maybe_kv {
                Ok(kv) => kv,
                Err(e) => match e.kind() {
                    std::io::ErrorKind::UnexpectedEof => {
                        break;
                    }
                    _ => return Err(e),
                },
            };
            self.database.insert(kv.key, current_position);
        }

        Ok(())
    }

    fn get_record_at_position(&self, position: u64) -> Result<KeyValuePair> {
        let mut file = std::io::BufReader::new(&self.file);
        file.seek(SeekFrom::Start(position))?;
        let akv = ActionKV::process_record(&mut file)?;
        Ok(akv)
    }

    fn update_record_at_position(&self, position: u64, value: String) -> Result<KeyValuePair> {
        let mut file = std::io::BufReader::new(&self.file);
        file.seek(SeekFrom::Start(position))?;
        let akv = ActionKV::process_record(&mut file)?;
        Ok(akv)
    }

    fn insert_in_database(&mut self, key: &str, value: &str) -> Result<u64> {
        let key_length = key.len();
        let value_length = value.len();
        let data_length = key_length + value_length;
        let mut data = Vec::with_capacity(data_length);

        for byte in key.bytes() {
            data.push(byte);
        }

        for byte in value.bytes() {
            data.push(byte);
        }

        let crc = crc::Crc::<u32>::new(&crc::CRC_32_CKSUM);
        let checksum = crc.checksum(&data);

        let mut file = BufWriter::new(&mut self.file);
        let next_byte = SeekFrom::End(0);
        let current_position = file.stream_position()?;
        file.seek(next_byte)?;
        file.write_u32::<BigEndian>(checksum)?;
        file.write_u32::<BigEndian>(key_length as u32)?;
        file.write_u32::<BigEndian>(value_length as u32)?;
        file.write_all(&data)?;

        Ok(current_position)
    }

    fn process_record<R: std::io::Read>(file: &mut R) -> Result<KeyValuePair> {
        let saved_checksum = file.read_u32::<BigEndian>()?;
        let key_length = file.read_u32::<BigEndian>()?;
        let value_length = file.read_u32::<BigEndian>()?;

        let data_length = key_length + value_length;
        let mut data = Vec::with_capacity(data_length as usize);
        file.by_ref()
            .take(data_length as u64)
            .read_to_end(&mut data)?;

        let crc = crc::Crc::<u32>::new(&crc::CRC_32_CKSUM);
        let checksum = crc.checksum(&data);
        if saved_checksum != checksum {
            let error_message = format!(
                "checksum mismatch: expected=0x{:0x} actual=0x{:0x}",
                saved_checksum, checksum
            );
            let error = Error::new(InvalidData, error_message);
            return Err(error);
        }

        let (key_data, value_data) = data.split_at(key_length as usize);
        let key = String::from_utf8_lossy(key_data).to_string();
        let value = String::from_utf8_lossy(value_data).to_string();
        let kvp = KeyValuePair { key, value };
        Ok(kvp)
    }
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
