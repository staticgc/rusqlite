
use std::path::{Path, PathBuf};
use crate::Error;

use crate::OpenFlags;


/// AccessMode indicates what exactly needs to be checked for access
pub enum AccessMode {
    /// Request to check if file exists or not
    Exists,

    /// Request to check if file has read & write permissions
    ReadWrite,

    /// Request to check if file is read-only or not. As per sqlite, this is not used
    Read,
}

bitflags::bitflags! {
    #[repr(C)]
    /// SyncFlags tell the VFS the way a sync needs to happen
    pub struct SyncFlags: std::os::raw::c_int {
        /// Essentially this is posix fsync()
        const NORMAL = libsqlite3_sys::SQLITE_SYNC_NORMAL;

        /// Mac OSX style fullsync()
        const FULL = libsqlite3_sys::SQLITE_SYNC_FULL;

        /// Only the data of the file and not its inode needs to be synced
        const DATA = libsqlite3_sys::SQLITE_SYNC_DATAONLY;
    }
}


/// This trait represents every file which is opened by sqlite
///
/// All the methods take immutable reference to self. Implementation
/// is expected to take care of interior mutability.
pub trait VFSFile {
    /// Return the filepath associated with the object
    ///
    /// This is not part of the sqlite interface per se. This mainly used by the
    /// logging in vfs wrapper layer.
    fn filepath(&self) -> PathBuf; 

    /// Close the file
    ///
    /// After this call the trait object will be de-allocated
    fn close(&self) -> Result<(), Error>;

    /// Read the buffer at an offset
    ///
    /// Reads into the supplied buffer `buf` at offset `off` returning
    /// the amount of bytes actually read. Offsets should ideally be aligned
    /// to the page size but it is recommended not to assume that.
    fn read(&self, buf: &mut [u8], off: i64) -> Result<usize, Error>;

    /// Write the buffer `buf` at offset `off`
    ///
    /// Writes the supplied buffer and returns the amount of bytes actually written
    /// Offsets should ideally be aligned to the page size but it is recommended 
    /// not to assume that. 
    fn write(&self, buf: &[u8], off: i64) -> Result<usize, Error>;

    /// Sync flushes the data to durable storage
    ///
    /// The sync flags indicate the granular mode of sync
    fn sync(&self, flags: SyncFlags) -> Result<(), Error>;

    /// Truncate sets the file to the given size
    ///
    /// The new size could larger or smaller than existing size
    fn truncate(&self, size: i64) -> Result<(), Error>;

    /// Size returns the number of bytes in the file
    fn size(&self) -> Result<i64, Error>;

    fn lock(&self, _flags: i32) -> Result<(), Error> {
        Ok(())
    }
    fn unlock(&self, _flags: i32) -> Result<(), Error> {
        Ok(())
    }
    fn sector_size(&self) -> i32 {
        512
    }
    fn check_reserved_lock(&self) -> Result<i32, Error> {
        Ok(0)
    }
    fn device_characteristics(&self) -> i32 {
        0
    }
}

/// VFS represents the filesystem
///
/// Using this, sqlite can be made to work on a custom filesystem/backend
pub trait VFS {
    /// Name returns the VFS's name
    ///
    /// This is not a part of the actual sqlite interface, but the vfs wrapper 
    /// calls this during registration of the VFS
    fn name(&self) -> String;

    /// Opens the file
    fn open(&self, filepath: &Path, flags: OpenFlags) -> Result<Box<dyn VFSFile>, Error>;

    /// Deletes the file
    fn remove(&self, filepath: &Path, dir_sync: bool) -> Result<(), Error>;

    /// Check the permissions or existance of the file. See the `AccessMode`
    fn access(&self, filepath: &Path, mode: AccessMode) -> Result<bool, Error>;

    /// Fullpath converts the given relative path to fullpath
    ///
    /// Sqlite will generate new file names/path (-journal, -wal) based on the return value
    fn fullpath(&self, filepath: &Path) -> Result<PathBuf, Error>;

    /// Sleep for specified interval
    ///
    /// The interval is in microseconds
    fn sleep(&self, interval: i32) -> i32 {
        std::thread::sleep(std::time::Duration::from_micros(interval as u64));
        interval
    }

    /// Current time returns the time Julian day number
    fn current_time(&self) -> f64 {
        let d = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap();
        d.as_secs_f64()/86400.0 + 2440587.5
    }
}