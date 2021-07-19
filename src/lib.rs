#[cfg(feature = "crypto")]
mod crypto;
/// A simple to use config storage library for Rust.
mod error;
use error::Error;
#[cfg(feature = "compression")]
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use json_dotpath::DotPaths;
use serde::Serialize;
use serde_json::{self, Value};
#[cfg(feature = "compression")]
use std::io::{Read, Write};

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
    #[cfg(feature = "compression")]
    compressed: bool,
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
                let root_path = base_dirs;
                Ok(Self {
                    path: root_path,
                    project_name,
                    config_name: "config",
                    file_extension: "json",
                    project_suffix: "rs",
                    #[cfg(feature = "crypto")]
                    encryption_key: None,
                    #[cfg(feature = "compression")]
                    compressed: false,
                })
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
        DotPaths::dot_get::<Value>(&parsed_json, path).map_err(Error::from)
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
            self.init_store()?;
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
        Ok(value)
    }

    /// Get the path to the directory where the configuration data is stored.
    pub fn get_store_dir_path(&self) -> PathBuf {
        let mut project_name = self.project_name.to_owned();
        project_name.push('-');
        project_name.push_str(self.project_suffix);
        let mut store_path = self.path.clone();
        store_path.push(project_name);
        store_path
    }

    /// Get the path to the configuration file.
    pub fn get_store_path(&self) -> PathBuf {
        let mut store_dir_path = self.get_store_dir_path();
        let mut file_name = PathBuf::new();
        file_name.push(self.project_name);
        file_name.set_extension(self.file_extension);
        store_dir_path.push(file_name);
        store_dir_path
    }

    /// Makes the store directory if it does not exist.
    ///
    /// # Errors
    ///
    /// Errors if the store directory cannot be created.
    fn make_store_path(&self) -> Result<()> {
        fs::create_dir(self.get_store_dir_path()).map_err(Error::from)
    }

    /// Initializes the store file.

    /// *NOTE* This will initilize the store as either encrypted or
    /// not depdending on if the encryption key is set.
    ///
    /// # Errors
    ///
    /// * The store file directory be created.
    /// * The store file cannot be created.
    /// * The store file cannot be initialized.
    /// * The store file cannot be wrote to.
    fn init_store(&self) -> Result<()> {
        if !self.store_dir_exists() {
            if let Err(e) = self.make_store_path() {
                return Err(e);
            };
        }
        if !self.store_exists() {
            File::create(self.get_store_path())?;
        }
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
        fs::remove_dir_all(self.get_store_dir_path()).map_err(Error::from)
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
            return fs::write(self.get_store_path(), encrypted_data).map_err(Error::from);
        }

        #[cfg(feature = "compression")]
        if self.get_compressed() {
            let mut e = GzEncoder::new(Vec::new(), Compression::default());
            e.write_all(data.as_bytes())?;
            // returns io error so can be unwrapped
            let compressed_data = e.finish()?;
            return fs::write(self.get_store_path(), compressed_data).map_err(Error::from);
        }

        fs::write(self.get_store_path(), data).map_err(Error::from)
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

        #[cfg(feature = "compression")]
        if self.get_compressed() {
            let mut gz = GzDecoder::new(&store_data[..]);
            let mut s = String::new();
            gz.read_to_string(&mut s)?;
            return Self::parse_json(s);
        }

        let data = String::from_utf8(store_data)?;
        Store::parse_json(data)
    }

    fn parse_json(store: String) -> Result<Value> {
        serde_json::from_str(&store).map_err(Error::from)
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
        for (i, byte) in key_bytes.iter().enumerate() {
            final_bytes[i] = *byte;
        }

        self.encryption_key = Some(final_bytes);
        Ok(())
    }

    #[cfg(feature = "compression")]
    pub fn set_compressed(&mut self, compressed: bool) {
        self.compressed = compressed;
    }

    #[cfg(feature = "compression")]
    pub fn get_compressed(&self) -> bool {
        self.compressed
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
            Ok(_) => panic!(),
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
            Ok(_) => panic!(),
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

    #[cfg(feature = "compression")]
    #[test]
    fn compression() {
        let mut x = Store::new("compression_test").unwrap();
        x.compressed = true;
        let data = "
            Lorem ipsum dolor sit amet, consectetur adipiscing elit. Proin eu sem euismod, luctus diam non, dignissim enim. Proin iaculis condimentum mattis. Donec sagittis gravida urna eget faucibus. Vestibulum vel iaculis neque. Cras varius nisi convallis diam semper mattis. Pellentesque nec lectus risus. Proin egestas ultricies ligula, eu convallis arcu condimentum sed. Maecenas ligula urna, faucibus sit amet porta sit amet, laoreet at diam. Curabitur hendrerit, ipsum eget luctus porttitor, lectus lectus aliquam quam, in ullamcorper risus elit in purus. Nunc elementum nisi in felis commodo, vel rhoncus purus blandit. Mauris non lectus at lorem sodales dapibus. Nulla viverra libero vitae malesuada laoreet. Integer nulla est, tristique eget nibh id, luctus commodo velit. Nullam viverra ante eget risus sollicitudin laoreet. Nullam placerat, nisl vel euismod venenatis, quam ante faucibus tellus, sed condimentum urna arcu consequat ipsum. Curabitur interdum, odio a ultrices mattis, purus magna semper dui, non tristique libero justo ut arcu.
            Sed risus risus, fringilla nec lobortis sed, pretium eget ligula. Mauris placerat tincidunt massa eu condimentum. Sed dapibus diam nec mattis pretium. Praesent gravida erat facilisis diam sodales, eget malesuada ligula aliquam. Duis sed rutrum dui. Pellentesque ultricies augue velit. Praesent eu consectetur sapien.
            Phasellus hendrerit eros quis dui efficitur dignissim. Pellentesque augue lacus, sagittis eget aliquam vel, sollicitudin ac eros. In tempor enim velit, in sodales risus tristique nec. Phasellus vulputate non massa quis malesuada. Aenean pulvinar, tortor sit amet laoreet finibus, lorem lectus congue sapien, at vestibulum augue ipsum at quam. Pellentesque porta convallis convallis. Integer maximus convallis elit, at iaculis arcu porttitor sit amet. Quisque gravida tempus elit non efficitur. Sed euismod, orci sit amet finibus malesuada, magna erat dapibus tortor, congue volutpat mauris magna id lectus. Nunc pellentesque eleifend velit, nec eleifend diam pulvinar vitae. Duis tempus eros lectus, eget bibendum ipsum lacinia sed. Pellentesque at interdum purus. Vivamus placerat eu justo ut egestas. Etiam semper volutpat massa quis facilisis. Sed eu luctus arcu.
            Duis eget eros et lectus iaculis scelerisque in ac odio. Etiam vehicula, justo vel pulvinar dignissim, felis orci cursus justo, sit amet elementum sapien nulla vitae augue. Proin eu purus dui. Nam sagittis dictum orci, a scelerisque arcu pulvinar eget. Ut sit amet pellentesque sem. Sed eu erat ac ipsum condimentum faucibus nec id metus. Quisque a velit porta, pulvinar dui eu, finibus magna. Vivamus facilisis mi mi, ultrices congue erat interdum commodo. Nullam eget pharetra turpis. Nunc in sem in nibh consectetur finibus. Nunc purus eros, faucibus et elementum ac, faucibus eget metus.
            Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed commodo nulla eget metus placerat, vel dapibus mauris interdum. Etiam eget commodo nulla. Sed vehicula dui lacus, in eleifend dui tincidunt non. Donec id consequat ipsum, at iaculis nunc. In posuere odio ut metus cursus, non facilisis enim feugiat. Morbi tortor sem, hendrerit nec suscipit vel, accumsan sed est. Aenean ac venenatis dolor, eget sagittis lorem. Ut in facilisis erat. Mauris velit lectus, bibendum at nisl ac, porttitor lobortis mauris. Phasellus ac porttitor ipsum. Donec tristique laoreet tortor, vitae tincidunt lectus efficitur a. Sed ut semper lorem. ";
        x.init_store().unwrap();
        x.set("a", data).unwrap();
        assert_eq!(x.get("a").unwrap().unwrap(), data);
        clean_store(&x);
    }

    // This test should prioritize the encryption over the compression
    #[cfg(feature = "compression")]
    #[cfg(feature = "crypto")]
    #[test]
    fn compression_and_encryption() {
        let mut x = Store::new("compression_encryption_test").unwrap();
        x.compressed = true;
        x.set_encryption_key("the encryption key").unwrap();
        let data = "
            Lorem ipsum dolor sit amet, consectetur adipiscing elit. Proin eu sem euismod, luctus diam non, dignissim enim. Proin iaculis condimentum mattis. Donec sagittis gravida urna eget faucibus. Vestibulum vel iaculis neque. Cras varius nisi convallis diam semper mattis. Pellentesque nec lectus risus. Proin egestas ultricies ligula, eu convallis arcu condimentum sed. Maecenas ligula urna, faucibus sit amet porta sit amet, laoreet at diam. Curabitur hendrerit, ipsum eget luctus porttitor, lectus lectus aliquam quam, in ullamcorper risus elit in purus. Nunc elementum nisi in felis commodo, vel rhoncus purus blandit. Mauris non lectus at lorem sodales dapibus. Nulla viverra libero vitae malesuada laoreet. Integer nulla est, tristique eget nibh id, luctus commodo velit. Nullam viverra ante eget risus sollicitudin laoreet. Nullam placerat, nisl vel euismod venenatis, quam ante faucibus tellus, sed condimentum urna arcu consequat ipsum. Curabitur interdum, odio a ultrices mattis, purus magna semper dui, non tristique libero justo ut arcu.
            Sed risus risus, fringilla nec lobortis sed, pretium eget ligula. Mauris placerat tincidunt massa eu condimentum. Sed dapibus diam nec mattis pretium. Praesent gravida erat facilisis diam sodales, eget malesuada ligula aliquam. Duis sed rutrum dui. Pellentesque ultricies augue velit. Praesent eu consectetur sapien.
            Phasellus hendrerit eros quis dui efficitur dignissim. Pellentesque augue lacus, sagittis eget aliquam vel, sollicitudin ac eros. In tempor enim velit, in sodales risus tristique nec. Phasellus vulputate non massa quis malesuada. Aenean pulvinar, tortor sit amet laoreet finibus, lorem lectus congue sapien, at vestibulum augue ipsum at quam. Pellentesque porta convallis convallis. Integer maximus convallis elit, at iaculis arcu porttitor sit amet. Quisque gravida tempus elit non efficitur. Sed euismod, orci sit amet finibus malesuada, magna erat dapibus tortor, congue volutpat mauris magna id lectus. Nunc pellentesque eleifend velit, nec eleifend diam pulvinar vitae. Duis tempus eros lectus, eget bibendum ipsum lacinia sed. Pellentesque at interdum purus. Vivamus placerat eu justo ut egestas. Etiam semper volutpat massa quis facilisis. Sed eu luctus arcu.
            Duis eget eros et lectus iaculis scelerisque in ac odio. Etiam vehicula, justo vel pulvinar dignissim, felis orci cursus justo, sit amet elementum sapien nulla vitae augue. Proin eu purus dui. Nam sagittis dictum orci, a scelerisque arcu pulvinar eget. Ut sit amet pellentesque sem. Sed eu erat ac ipsum condimentum faucibus nec id metus. Quisque a velit porta, pulvinar dui eu, finibus magna. Vivamus facilisis mi mi, ultrices congue erat interdum commodo. Nullam eget pharetra turpis. Nunc in sem in nibh consectetur finibus. Nunc purus eros, faucibus et elementum ac, faucibus eget metus.
            Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed commodo nulla eget metus placerat, vel dapibus mauris interdum. Etiam eget commodo nulla. Sed vehicula dui lacus, in eleifend dui tincidunt non. Donec id consequat ipsum, at iaculis nunc. In posuere odio ut metus cursus, non facilisis enim feugiat. Morbi tortor sem, hendrerit nec suscipit vel, accumsan sed est. Aenean ac venenatis dolor, eget sagittis lorem. Ut in facilisis erat. Mauris velit lectus, bibendum at nisl ac, porttitor lobortis mauris. Phasellus ac porttitor ipsum. Donec tristique laoreet tortor, vitae tincidunt lectus efficitur a. Sed ut semper lorem. ";

        x.init_store().unwrap();
        x.set("a", data).unwrap();
        x.compressed = false;
        assert_eq!(x.get("a").unwrap().unwrap(), data);
        clean_store(&x);
    }
}
