## Aquila FS 
[![Crates.io](https://img.shields.io/crates/v/aquila_fs.svg)](https://crates.io/crates/aquila_fs)
[![Downloads](https://img.shields.io/crates/d/aquila_fs.svg)](https://crates.io/crates/aquila_fs)
[![Docs](https://docs.rs/aquila_fs/badge.svg)](https://docs.rs/aquila_fs/)

A storage backend powered by the local filesystem.

Uses atomic writes to ensure assets are not read partially or lost during upload.

### Usage

```rust
use aquila_fs::FileSystemStorage;

let storage = FileSystemStorage::new("./aquila_data");
```

License: MIT OR Apache-2.0
