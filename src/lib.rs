#[cfg(feature = "crypto")]
mod crypto;
/// A simple to use config storage library for Rust.
mod error;

use dirs;
use error::Error;
use json_dotpath::DotPaths;
use serde::Serialize;
use serde_json::{self, Value};
#[cfg(feature = "crypto")]
use std::ops::Index;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    result,
};

/// Convenience type for resulting from a `Result<T>` using [`Result`].
///
/// [`Result`]: `https://doc.rust-lang.org/std/result/enum.Result.html`
pub type Result<T> = result::Result<T, Error>;

/// Represents a store of configuration data in a JSON format.
pub struct Store<'a> {
    /// The base directory for the store.
    path: PathBuf,
    /// The project's name
    pub project_name: &'a str,
    /// The configuration name
    pub config_name: &'a str,
    /// The file extension for configuration files.
    pub file_extension: &'a str,
    /// The project name's suffix
    pub project_suffix: &'a str,
    /// An optional encrpytion key for the store.
    #[cfg(feature = "crypto")]
    encryption_key: Option<[u8; 32]>,
}

impl Store<'_> {
    /// Creates a new instance of the store requiring the project's name.
    /// This name will be used as the folder name to store the configuration data.
    /// The default store location is the application configuration directory.
    ///
    /// See [dirs::config_dir][dirs::config_dir] for more information.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bland::Store;
    /// let store = Store::new("my-app").unwrap();
    /// ```
    pub fn new(project_name: &'static str) -> Result<Self> {
        match dirs::config_dir() {
            Some(base_dirs) => {
                let root_path = base_dirs.to_path_buf();
                return Ok(Self {
                    path: root_path,
                    project_name,
                    config_name: "config",
                    file_extension: "json",
                    project_suffix: "rs",
                    #[cfg(feature = "crypto")]
                    encryption_key: None,
                });
            }
            None => Err(Error::ConfigDir),
        }
    }

    /// Returns the value at the given path from the store.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bland::Store;
    /// let store = Store::new("get-app").unwrap();
    /// store.set("a.b", 42).unwrap();
    /// assert_eq!(store.get("a.b").unwrap().unwrap(), 42);
    /// # store.delete_store().unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Errors if
    /// * The store does not exist.
    /// * It fails to read the store file.
    /// * The store cannot be deserialized.
    /// * `path` is not a valid dot path.
    /// * `path` attempts to access an index out of bounds.
    pub fn get(&self, path: &str) -> Result<Option<Value>> {
        if !self.store_exists() {
            return Err(Error::NotFound);
        }
        let parsed_json = self.get_store_as_parsed_json()?;
        DotPaths::dot_get::<Value>(&parsed_json, path).map_err(|e| Error::from(e))
    }

    /// Sets the given data using a [json dotpath](https://crates.io/crates/json_dotpath).
    ///
    /// **NOTE:** This will create the store directory and file if it doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bland::Store;
    /// let store = Store::new("set-app").unwrap();
    /// store.set("a.b", 42).unwrap();
    /// assert_eq!(store.get("a.b").unwrap().unwrap(), 42);
    /// # store.delete_store().unwrap();
    /// ```
    /// The data will be stored in the following format:
    /// ```json
    /// {
    ///     "a": {
    ///         "b": 42
    ///     }
    /// }
    /// ```
    ///
    /// # Example of a bad set
    ///
    /// ```rust
    /// # use bland::Store;
    /// let store = Store::new("bad-set-app").unwrap();
    /// store.set("a", "hello").unwrap();
    /// match store.set("a.b", "world") {
    ///     Ok(_) => assert!(false),
    ///     Err(e) => {
    ///         assert_eq!(e.to_string(), "Unexpected value reached while traversing path");
    ///    },
    /// };
    /// # store.delete_store().unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Errors if
    /// * The store fails to be created.
    /// * It fails to read the store file.
    /// * The store cannot be deserialized.
    /// * The store file fails to be written to.
    /// * `path` is not a valid dot path.
    pub fn set<T>(&self, path: &str, data: T) -> Result<()>
    where
        T: Serialize,
    {
        let json_data = serde_json::to_value(&data)?;
        if !self.store_exists() {
            self.create_store()?;
        }
        let mut parsed_json = self.get_store_as_parsed_json()?;
        DotPaths::dot_set(&mut parsed_json, path, json_data)?;
        self.write_store(parsed_json.to_string())
    }

    /// Deletes the given path from the store.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bland::Store;
    /// let store = Store::new("delete-app").unwrap();
    /// store.set("a.b", 42).unwrap();
    /// assert_eq!(store.get("a.b").unwrap().unwrap(), 42);
    /// store.delete("a.b").unwrap();
    /// assert!(store.get("a.b").unwrap().is_none());
    /// # store.delete_store().unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Errors if
    /// * The store does not exist.
    /// * It fails to read the store file.
    /// * The store cannot be deserialized.
    /// * The store file fails to be written to.
    /// * `path` is not a valid dot path.
    pub fn delete(&self, path: &str) -> Result<Option<Value>> {
        if !self.store_exists() {
            return Err(Error::NotFound);
        }

        let mut parsed_json = self.get_store_as_parsed_json()?;
        let value = DotPaths::dot_take::<Value>(&mut parsed_json, path)?;
        self.write_store(parsed_json.to_string())?;
        return Ok(value);
    }

    /// Get the path to the directory where the configuration data is stored.
    pub fn get_store_dir_path(&self) -> PathBuf {
        let mut project_name = self.project_name.to_owned();
        project_name.push_str("-");
        project_name.push_str(self.project_suffix);
        let mut store_path = self.path.clone();
        store_path.push(project_name);
        return store_path;
    }

    /// Get the path to the configuration file.
    pub fn get_store_path(&self) -> PathBuf {
        let mut store_dir_path = self.get_store_dir_path();
        let mut file_name = PathBuf::new();
        file_name.push(self.project_name);
        file_name.set_extension(self.file_extension);
        store_dir_path.push(file_name);
        return store_dir_path;
    }

    /// Makes the store directory if it does not exist.
    ///
    /// # Errors
    ///
    /// Errors if the store directory cannot be created.
    fn make_store_path(&self) -> Result<()> {
        fs::create_dir(self.get_store_dir_path()).map_err(|e| Error::from(e))
    }

    /// Makes a new store file. Creates the directory if it doesn't exist.
    ///
    /// # Errors
    ///
    /// Errors if
    /// 
    /// * The store file directory be created.
    /// * The store file cannot be created.
    /// * The store file cannot be initialized.
    pub fn create_store(&self) -> Result<()> {
        if !self.store_dir_exists() {
            if let Err(e) = self.make_store_path() {
                return Err(e);
            };
        }
        File::create(self.get_store_path())?;
        self.init_store()
    }

    /// Initializes the store file.

    /// *NOTE* This will initilize the store as either encrypted or
    /// not depdending on if the encryption key is set.
    ///
    /// # Errors
    ///
    /// Errors if the store file cannot be wrote to.
    pub fn init_store(&self) -> Result<()> {
        self.write_store("{}".to_string())
    }

    /// Returns a boolean indicating whether the store directory exists.
    pub fn store_dir_exists(&self) -> bool {
        Path::new(&self.get_store_dir_path()).exists()
    }

    /// Returns a boolean indicating whether the store file exists.
    pub fn store_exists(&self) -> bool {
        Path::new(&self.get_store_path()).exists()
    }

    /// Deletes the store file and directory.
    ///
    /// # Errors
    ///
    /// Errors if the store file cannot be deleted.
    pub fn delete_store(&self) -> Result<()> {
        fs::remove_dir_all(self.get_store_dir_path()).map_err(|e| Error::from(e))
    }

    /// Writes the store file.
    ///
    /// # Errors
    ///
    /// Errors if the store file cannot be written to.
    fn write_store(&self, data: String) -> Result<()> {
        #[cfg(feature = "crypto")]
        if let Some(key) = self.encryption_key {
            let encrypted_data = crypto::encrypt_data(&data, key)?;
            return fs::write(self.get_store_path(), encrypted_data).map_err(|e| Error::from(e));
        }
        fs::write(self.get_store_path(), data).map_err(|e| Error::from(e))
    }

    /// Returns the parsed JSON of the store file.
    ///
    /// # Errors
    ///
    /// * Errors if the store file does not exist.
    /// * Errors if the store file cannot be read.
    /// * Errors if the store file cannot be deserialized.
    fn get_store_as_parsed_json(&self) -> Result<Value> {
        if !self.store_exists() {
            return Err(Error::NotFound);
        }
        let store_data = fs::read(self.get_store_path())?;
        #[cfg(feature = "crypto")]
        if let Some(key) = self.encryption_key {
            let data = crypto::decrypt_data(store_data, key)?;
            return Store::parse_json(data);
        }

        let data = String::from_utf8(store_data)?;
        Store::parse_json(data)
    }

    fn parse_json(store: String) -> Result<Value> {
        serde_json::from_str(&store).map_err(|e| Error::from(e))
    }

    /// Sets the encryption key. The key must be less than or equal to 32 bytes.
    #[cfg(feature = "crypto")]
    pub fn set_encryption_key(&mut self, key: &str) -> Result<()> {
        let mut final_bytes = [0; 32];
        let key_bytes = key.as_bytes().to_vec();
        if key_bytes.len() > 32 {
            return Err(Error::InvalidKeyLength);
        }
        // probably a better way of doing this...
        for i in 0..key_bytes.len() {
            final_bytes[i] = *key_bytes.index(i);
        }

        self.encryption_key = Some(final_bytes);
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use crate::Store;

    #[cfg(feature = "crypto")]
    use crate::Error;

    fn clean_store(x: &Store) {
        if x.store_exists() {
            x.delete_store().unwrap();
        }
    }

    #[test]
    fn set_get() {
        let x = Store::new("store_get_test").unwrap();
        x.set("a.b", "test1").unwrap();
        x.set("c", [4, 2, 7]).unwrap();
        assert_eq!(x.get("a.b").unwrap().unwrap(), "test1");
        assert_eq!(x.get("c").unwrap().unwrap().as_array().unwrap().len(), 3);
        assert_eq!(x.get("d").unwrap(), None);
        clean_store(&x);
    }

    #[test]
    fn invalid_set() {
        let x = Store::new("store_invalid_set_test").unwrap();
        x.set("x", "test1").unwrap();
        match x.set("x.a", "test2") {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(
                e.to_string(),
                json_dotpath::Error::BadPathElement.to_string()
            ),
        };
        clean_store(&x);
    }

    #[test]
    fn delete() {
        let x = Store::new("store_delete_test").unwrap();
        x.set("a.b", "test1").unwrap();
        assert_eq!(x.get("a.b").unwrap().unwrap(), "test1");
        x.delete("a").unwrap();
        assert_eq!(x.get("a").unwrap(), None);
        clean_store(&x);
    }

    #[test]
    fn init() {
        let x = Store::new("clear_test").unwrap();
        x.set("a.b", "test1").unwrap();
        x.set("c", [4, 2, 7]).unwrap();
        assert_eq!(x.get("a.b").unwrap().unwrap(), "test1");
        assert_eq!(x.get("c").unwrap().unwrap().as_array().unwrap().len(), 3);
        x.init_store().unwrap();
        assert_eq!(x.get("a.b").unwrap(), None);
        assert_eq!(x.get("c").unwrap(), None);
        clean_store(&x);
    }

    #[cfg(feature = "crypto")]
    #[test]
    fn set_encryption_key() {
        let mut x = Store::new("set_encryption_key_test").unwrap();
        x.set_encryption_key("test_key").unwrap();
        assert_eq!(
            x.encryption_key,
            Some([
                116, 101, 115, 116, 95, 107, 101, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0
            ])
        )
    }

    #[cfg(feature = "crypto")]
    #[test]
    fn invalid_encryption_key() {
        let mut x = Store::new("invalid_encryption_key_test").unwrap();
        match x.set_encryption_key("test_key_this_key_will_be_too_long") {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e.to_string(), Error::InvalidKeyLength.to_string()),
        };
    }

    #[cfg(feature = "crypto")]
    #[test]
    fn crypto() {
        let mut x = Store::new("crypto_test").unwrap();
        x.set_encryption_key("test_key").unwrap();
        x.set("a", "test1").unwrap();
        assert_eq!(x.get("a").unwrap().unwrap(), "test1");
        clean_store(&x);
    }
}
