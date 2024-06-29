use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Error, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use std::path::Path;

/// A key/value database with strong durability.  All entries in the database
/// are stored in non-volatile memory as part of each `insert` and `update`
/// operation along with a 32-bit CRC value.  Subsequently, each `get` request
/// does incur some IO cost as the value is stored in the database.

struct KeyValuePair {
    key: String,
    value: String,
}

pub struct ActionKV {
    file: File,
    database: HashMap<String, u64>,
}

impl ActionKV {
    /// Opens the database located at `path`.
    pub fn open(path: &Path) -> Result<ActionKV> {
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

    /// Deletes the value from the database associated with `key`.  Note that
    /// the key remains in the database but the value empty.
    pub fn delete(&mut self, key: String) -> Result<()> {
        if !self.database.contains_key(&key) {
            let error_message = format!("key: {key} not found in database");
            let error = Error::new(ErrorKind::InvalidData, error_message);
            return Err(error);
        };

        let value = String::new();
        let _ = self.insert_in_database(&key, &value);
        Ok(())
    }

    /// Retrieves `key` from the database and returns is associated `value`. If
    /// the key does not exist an error is returned.
    pub fn get(&self, key: String) -> Result<String> {
        let position = match self.database.get(&key) {
            Some(position) => position,
            None => {
                let error_message = format!("key: {key} not found in database");
                let error = Error::new(ErrorKind::InvalidData, error_message);
                return Err(error);
            }
        };

        let akv = self.get_record_at_position(*position)?;
        Ok(akv.value)
    }

    /// Creaes or updates an entry in the database with the `key` and `value`
    /// association.
    pub fn insert(&mut self, key: String, value: String) -> Result<()> {
        let position = self.insert_in_database(&key, &value)?;
        self.database.insert(key, position);
        Ok(())
    }

    /// Creaes or updates an entry in the database with the `key` and `value`
    /// association.
    ///
    /// Note: Calling update is equivalent to calling insert.
    pub fn update(&mut self, key: String, value: String) -> Result<()> {
        self.insert(key, value)?;
        Ok(())
    }

    /// Reads the database file located at `path` into memory.
    fn load(&mut self) -> Result<()> {
        let mut file = BufReader::new(&self.file);

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

    /// Rerieve the record stored in the database at byte offset `position`.
    fn get_record_at_position(&self, position: u64) -> Result<KeyValuePair> {
        let mut file = std::io::BufReader::new(&self.file);
        file.seek(SeekFrom::Start(position))?;
        let akv = ActionKV::process_record(&mut file)?;
        Ok(akv)
    }

    /// Writes a new record in the database for the `key`/`value` pair.
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

    /// Loads an entry `key`/`value` pair from the database.
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
            let error = Error::new(ErrorKind::InvalidData, error_message);
            return Err(error);
        }

        let (key_data, value_data) = data.split_at(key_length as usize);
        let key = String::from_utf8_lossy(key_data).to_string();
        let value = String::from_utf8_lossy(value_data).to_string();
        let kvp = KeyValuePair { key, value };
        Ok(kvp)
    }
}
