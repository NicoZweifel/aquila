## Aquila OpenDAL
[![Crates.io](https://img.shields.io/crates/v/aquila_opendal.svg)](https://crates.io/crates/aquila_opendal)
[![Downloads](https://img.shields.io/crates/d/aquila_opendal.svg)](https://crates.io/crates/aquila_opendal)
[![Docs](https://docs.rs/aquila_opendal/badge.svg)](https://docs.rs/aquila_opendal/)

A storage backend powered by [Apache OpenDAL](https://opendal.apache.org/).

Allows the server to be backed by any storage service supported by OpenDAL, including
the file system, AWS S3, GCS, Azure Blob Storage and more.

### Usage

```rust
let mut builder = Gcs::default();
builder.bucket("my-gcs-bucket");

let op = Operator::new(builder).unwrap().finish();
let storage = OpendalStorage::new(op);
```

License: MIT OR Apache-2.0
