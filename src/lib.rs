/// A simple to use config storage library for Rust.
mod error;

use dirs;
use error::Error;
use json_dotpath::DotPaths;
use serde::Serialize;
use serde_json::{self, Value};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    result,
};

const STORE_NAME: &str = "store.json";

pub type Result<T> = result::Result<T, Error>;

/// Represents a store of configuration data in a JSON format.
pub struct Store {
    /// The base directory for the store.
    pub path: PathBuf,
    /// The application's name
    pub application_name: &'static str,
}

impl Store {
    /// Creates a new instance of the store requiring an application name.
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
    pub fn new(application_name: &'static str) -> Result<Self> {
        match dirs::config_dir() {
            Some(base_dirs) => {
                let root_path = base_dirs.to_path_buf();
                return Ok(Self {
                    path: root_path,
                    application_name: application_name,
                });
            }
            None => Err(Error::ConfigDir),
        }
    }

    /// Inserts the given data using a [json dotpath](https://crates.io/crates/json_dotpath).
    ///
    /// **NOTE:** This will create the store directory and file if it doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bland::Store;
    /// let store = Store::new("my-app").unwrap();
    /// store.insert("a.b", 42).unwrap();
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
    /// # Panics
    ///
    /// Panics if
    /// * the store fails to be created.
    /// * it fails to read the store file.
    /// * the store cannot be deserialized.
    /// * the store file fails to be written to.
    /// * `path` is not a valid dot path.
    pub fn insert<T>(&self, path: &str, data: T) -> Result<()>
    where
        T: Serialize,
    {
        match serde_json::to_value(&data) {
            Ok(json_data) => {
                if !self.store_exists() {
                    if let Err(e) = self.create_store() {
                        return Err(Error::from(e));
                    };
                }
                match self.get_store_as_parsed_json() {
                    Ok(mut parsed_json) => {
                        match DotPaths::dot_set(&mut parsed_json, path, json_data) {
                            Ok(_) => match self.write_store(parsed_json.to_string()) {
                                Ok(_) => return Ok(()),
                                Err(e) => Err(Error::from(e)),
                            },
                            Err(e) => Err(Error::from(e)),
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(Error::from(e)),
        }
    }

    /// Deletes the given path from the store.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bland::Store;
    /// let store = Store::new("my-app").unwrap();
    /// store.insert("a.b", 42).unwrap();
    /// assert_eq!(store.get("a.b").unwrap().unwrap(), 42);
    /// store.delete("a.b").unwrap();
    /// assert!(store.get("a.b").unwrap().is_none());
    /// # store.delete_store().unwrap();
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if
    /// * the store does not exist.
    /// * it fails to read the store file.
    /// * the store cannot be deserialized.
    /// * the store file fails to be written to.
    /// * `path` is not a valid dot path.
    pub fn delete(&self, path: &str) -> Result<Option<Value>> {
        if !self.store_exists() {
            return Err(Error::NotFound);
        }
        match self.get_store_as_parsed_json() {
            Ok(mut parsed_json) => match DotPaths::dot_take::<Value>(&mut parsed_json, path) {
                Ok(value) => match self.write_store(parsed_json.to_string()) {
                    Ok(_) => return Ok(value),
                    Err(e) => Err(Error::from(e)),
                },
                Err(e) => Err(Error::from(e)),
            },
            Err(e) => Err(e),
        }
    }

    /// Returns the value at the given path from the store.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bland::Store;
    /// let store = Store::new("my-app").unwrap();
    /// store.insert("a.b", 42).unwrap();
    /// assert_eq!(store.get("a.b").unwrap().unwrap(), 42);
    /// # store.delete_store().unwrap();
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if
    /// * the store does not exist.
    /// * it fails to read the store file.
    /// * the store cannot be deserialized.
    /// * `path` is not a valid dot path.
    /// * `path` attempts to access an index out of bounds.
    pub fn get(&self, path: &str) -> Result<Option<Value>> {
        if !self.store_exists() {
            return Err(Error::NotFound);
        }
        match self.get_store_as_parsed_json() {
            Ok(parsed_json) => match DotPaths::dot_get::<Value>(&parsed_json, path) {
                Ok(res) => match res {
                    Some(value) => Ok(Some(value)),
                    None => Ok(None),
                },
                Err(e) => Err(Error::from(e)),
            },
            Err(err) => Err(err),
        }
    }

    /// Get the path to the directory where the configuration data is stored.
    pub fn get_store_dir_path(&self) -> PathBuf {
        let mut store_path = self.path.clone();
        store_path.push(self.application_name);
        return store_path;
    }

    /// Get the path to the configuration file.
    pub fn get_store_path(&self) -> PathBuf {
        let mut store_dir_path = self.get_store_dir_path();
        store_dir_path.push(STORE_NAME);
        return store_dir_path;
    }

    /// Set the app name.
    pub fn set_application_name(&mut self, new_name: &'static str) {
        self.application_name = new_name;
    }

    /// Makes the store directory if it does not exist.
    ///
    /// # Panics
    ///
    /// Panics if the store directory cannot be created.
    fn make_store_path(&self) -> Result<()> {
        fs::create_dir(self.get_store_dir_path()).map_err(|e| Error::from(e))
    }

    /// Makes a new store file. Creates the directory if it doesn't exist.
    ///
    /// # Panics
    ///
    /// Panics if the store file directory be created.
    /// Panics if the store file cannot be created.
    /// Panics is the store file cannot be initialized.
    pub fn create_store(&self) -> Result<()> {
        if !self.store_dir_exists() {
            if let Err(e) = self.make_store_path() {
                return Err(e);
            };
        }
        match File::create(self.get_store_path()) {
            Ok(_) => match self.init_store() {
                Ok(_) => return Ok(()),
                Err(e) => Err(e),
            },
            Err(e) => Err(Error::from(e)),
        }
    }

    /// Initializes the store file.
    ///
    /// # Panics
    ///
    /// Panics if the store file cannot be wrote to.
    fn init_store(&self) -> Result<()> {
        self.write_store("{}".to_string())
        // match file.write_all("{}".as_bytes()) {
        //     Ok(_) => Ok(()),
        //     Err(e) => Err(Error::from(e)),
        // }
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
    /// # Panics
    ///
    /// Panics if the store file cannot be deleted.
    pub fn delete_store(&self) -> Result<()> {
        fs::remove_dir_all(self.get_store_dir_path()).map_err(|e| Error::from(e))
    }

    /// Writes the store file.
    ///
    /// # Panics
    ///
    /// Panics if the store file cannot be written to.
    fn write_store(&self, data: String) -> Result<()> {
        fs::write(self.get_store_path(), data).map_err(|e| Error::from(e))
    }

    /// Returns the parsed JSON of the store file.
    ///
    /// # Panics
    ///
    /// Panics if the store file does not exist.
    /// Panics if the store file cannot be read.
    /// Panics if the store file cannot be deserialized.
    fn get_store_as_parsed_json(&self) -> Result<Value> {
        if !self.store_exists() {
            return Err(Error::NotFound);
        }
        match fs::read_to_string(self.get_store_path()) {
            Ok(store) => match serde_json::from_str(&store) {
                Ok(parsed_json) => return Ok(parsed_json),
                Err(e) => Err(Error::from(e)),
            },
            Err(e) => Err(Error::from(e)),
        }
    }
}

// Only run tests using the following command:
// `cargo test -- --test-threads=1`
#[cfg(test)]
mod tests {
    use crate::Store;

    fn clean_store(x: &Store) {
        if x.store_exists() {
            x.delete_store().unwrap();
        }
    }

    #[test]
    fn store_get() {
        let x = Store::new("store_get_test").unwrap();
        x.insert("a.b", "test1").unwrap();
        x.insert("c", [4, 2, 7]).unwrap();
        assert_eq!(x.get("a.b").unwrap().unwrap(), "test1");
        assert_eq!(x.get("c").unwrap().unwrap().as_array().unwrap().len(), 3);
        assert_eq!(x.get("d").unwrap(), None);
        clean_store(&x);
    }

    #[test]
    fn store_delete() {
        let x = Store::new("store_delete_test").unwrap();
        x.insert("a.b", "test1").unwrap();
        assert_eq!(x.get("a.b").unwrap().unwrap(), "test1");
        x.delete("a").unwrap();
        assert_eq!(x.get("a").unwrap(), None);
        clean_store(&x);
    }

    #[test]
    fn set_application_name_test() {
        let mut x = Store::new("test").unwrap();
        assert_eq!(x.application_name, "test");
        x.set_application_name("test2");
        assert_eq!(x.application_name, "test2");
    }
}
