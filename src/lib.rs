/// A simple to use config storage library for Rust.
use dirs;
use serde::Serialize;
use serde_json::{self, Value};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use json_dotpath::DotPaths;

const STORE_NAME: &str = "store.json";

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
    /// let store = Store::new("my-app");
    /// ```
    pub fn new(application_name: &'static str) -> Self {
        if let Some(base_dirs) = dirs::config_dir() {
            let root_path = base_dirs.to_path_buf();
            return Self {
                path: root_path,
                application_name: application_name,
            };
        } else {
            panic!("Config dir not found!");
        }
    }

    /// Stores the given data using a [json dotpath](https://crates.io/crates/json_dotpath).
    ///
    /// **NOTE:** This will create the store directory and file if it doesn't exist.
    ///
    /// # Prohibited store actions
    /// You cannot over-write existing data in the store if it has children.
    /// If you want to over-write this data, you must delete the existing data first.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bland::Store;
    /// let store = Store::new("my-app");
    /// store.store("a.b", 42).unwrap();
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
    pub fn store<T>(&self, path: &str, data: T) -> Result<(), ()>
    where
        T: Serialize,
    {
        if let Ok(json_data) = serde_json::to_value(&data) {
            if !self.store_exists() {
                if let Err(_) = self.create_store() {
                    return Err(());
                };
            }
            if let Ok(mut parsed_json) = self.get_store_as_parsed_json() {
                if let Ok(_) = DotPaths::dot_set(&mut parsed_json, path, json_data) {
                    if let Ok(_) = self.write_store(parsed_json.to_string()) {
                        return Ok(());
                    }
                }
            }
        }
        Err(())
    }

    /// Deletes the given path from the store.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bland::Store;
    /// let store = Store::new("my-app");
    /// store.store("a.b", 42).unwrap();
    /// assert_eq!(store.get("a.b").unwrap().unwrap(), 42);
    /// store.delete("a.b").unwrap();
    /// assert!(store.get("a.b").unwrap().is_none());
    /// # store.delete_store().unwrap();
    /// ```
    pub fn delete(&self, path: &str) -> Result<Option<Value>, ()> {
        if !self.store_exists() {
            return Err(());
        }
        if let Ok(mut parsed_json) = self.get_store_as_parsed_json() {
            if let Ok(value) = DotPaths::dot_take::<Value>(&mut parsed_json, path) {
                if let Ok(_) = self.write_store(parsed_json.to_string()) {
                    return Ok(value);
                }
            }
        }
        Err(())
    }

    /// Returns the value at the given path from the store.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bland::Store;
    /// let store = Store::new("my-app");
    /// store.store("a.b", 42).unwrap();
    /// assert_eq!(store.get("a.b").unwrap().unwrap(), 42);
    /// # store.delete_store().unwrap();
    /// ```
    pub fn get(&self, path: &str) -> Result<Option<Value>, ()> {
        if !self.store_exists() {
            return Err(());
        }
        if let Ok(parsed_json) = self.get_store_as_parsed_json() {
            if let Ok(res) = DotPaths::dot_get::<Value>(&parsed_json, path) {
                match res {
                    Some(value) => return Ok(Some(value)),
                    None => return Ok(None),
                }
            }
        }
        Err(())
    }

    /// Get the directory where the configuration data is stored.
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

    fn make_store_path(&self) -> Result<(), std::io::Error> {
        fs::create_dir(self.get_store_dir_path())
    }

    /// Makes a new store file. Creates the directory if it doesn't exist.
    pub fn create_store(&self) -> Result<(), ()> {
        if !self.store_dir_exists() {
            if let Err(_) = self.make_store_path() {
                return Err(());
            };
        }
        if let Ok(file) = File::create(self.get_store_path()) {
            if let Ok(_) = Store::init_store(file) {
                return Ok(());
            }
        }
        Err(())
    }

    fn init_store(mut file: File) -> Result<(), ()> {
        match file.write_all("{}".as_bytes()) {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
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
    pub fn delete_store(&self) -> Result<(), std::io::Error> {
        fs::remove_dir_all(self.get_store_dir_path())
    }

    fn write_store(&self, data: String) -> Result<(), std::io::Error> {
        fs::write(self.get_store_path(), data)
    }

    fn get_store_as_parsed_json(&self) -> Result<Value, ()> {
        if !self.store_exists() {
            return Err(());
        }
        if let Ok(store) = fs::read_to_string(self.get_store_path()) {
            if let Ok(parsed_json) = serde_json::from_str::<Value>(&store) {
                return Ok(parsed_json);
            }
        }
        Err(())
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
        let x = Store::new("test");
        clean_store(&x);
        x.store("a.b", "test1").unwrap();
        x.store("c", [4, 2, 7]).unwrap();
        assert_eq!(x.get("a.b").unwrap().unwrap(), "test1");
        assert_eq!(x.get("c").unwrap().unwrap().as_array().unwrap().len(), 3);
        assert_eq!(x.get("d").unwrap(), None);
    }

    #[test]
    fn store_delete() {
        let x = Store::new("test");
        clean_store(&x);
        x.store("a.b", "test1").unwrap();
        assert_eq!(x.get("a.b").unwrap().unwrap(), "test1");
        x.delete("a").unwrap();
        assert_eq!(x.get("a").unwrap(), None);
    }

    #[test]
    fn set_application_name_test() {
        let mut x = Store::new("test");
        assert_eq!(x.application_name, "test");
        x.set_application_name("test2");
        assert_eq!(x.application_name, "test2");
    }
}
