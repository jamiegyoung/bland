# bland
A simple config store for rust programs.

##  Optional Features

###  `crypto`
Provides encryption and decryption functionality for `bland`'s config store.
For example usage, see the `crypto` test in `lib.rs`.

### `compression`
Provides compression and decompression functionality for `bland`'s config store.
For example usage, see the `compression` test in `lib.rs`.
*Note*: If both `compression` and `crypto` are enabled, `crypto` will take priority.

##  Documentation
Run `cargo doc --open` to open the documentation in your browser.

Run `cargo doc --open --feature <feature>` to open the documentation for a specific feature.
