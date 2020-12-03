//! Virtual File System interface
//!
//! Sqlite provides a way to plug your own filesystem as a backend.
//! To implement a custom backend/filesystem two traits are provided
//! See the sqltie vfs [doc](https://www.sqlite.org/c3ref/vfs.html) and 
//! i/o methods needed on each file [here](https://www.sqlite.org/c3ref/io_methods.html)
//!
//! 1. VFS: Represents the filesystem
//! 2. VFSFile: Represents the file. The VFS interface creates the VFSFile instance using trait objects 
//!
//! An instance of `VFS` implementation needs to be registered using `register_vfs()`
//! This has to be done before the sqlite db connection is created. To make sqlite
//! use the custom vfs, pass the name of the vfs in `Connection::open_with_flags_and_vfs()`


mod traits;
mod file_capi;
mod fs_capi;
pub mod passfs;

pub use traits::*;
pub use fs_capi::register_vfs;