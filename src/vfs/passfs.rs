//! Passthrough filesystem which implements the VFS interface
//!
//! Note: This is intented for testing and serve as an example.
//! Do not use this in production.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::io::{Write};

use crate::vfs::{AccessMode, SyncFlags, VFS, VFSFile};
use crate::{Error, OpenFlags};

/// PassFS is a passthrough implementation of VFS
///
/// All I/O requests are passed to the underying FS as is
pub struct PassFS {}


impl VFS for PassFS {
    fn name(&self) -> String {
        "passfs".to_owned()
    }

    fn open(&self, filepath: &Path, _flags: OpenFlags) -> Result<Box<dyn VFSFile>, Error> {
        //trace!("f=open,filepath={:?}", filepath);
        let file = PassFile::new(filepath)?;

        Ok(Box::new(file))
    }

    fn remove(&self, filepath: &Path, _dir_sync: bool) -> Result<(), Error> {
        //trace!("f=remove,filepath={:?},dir_sync={}", filepath, dir_sync);

        match std::fs::remove_file(filepath) {
            Ok(()) => {},
            Err(e) => {
                let kind = e.kind();
                if kind != std::io::ErrorKind::NotFound {
                    Err(e)?
                }
            }
        }

        Ok(())
    }

    fn fullpath(&self, filepath: &Path) -> Result<PathBuf, Error> {
        //trace!("f=fullpath,filepath={:?}", filepath);
        let fullpath = std::env::current_dir()?;
        let fullpath = fullpath.join(filepath);
        //trace!("f=fullpath,fullpath={:?}", fullpath);
        Ok(fullpath)
    }

    fn access(&self, filepath: &Path, mode: AccessMode) -> Result<bool, Error> {
        //trace!("f=access,filepath={:?}", filepath);
        match mode {
            AccessMode::Exists => {
                Ok(filepath.exists())
            },
            AccessMode::ReadWrite => {
                let meta = filepath.metadata()?;
                Ok(!meta.permissions().readonly())
            }
            _ => Ok(false)
        }    
    }
}


struct PassFileInner {
    size: u64,
    file: File,
}

struct PassFile {
    filepath: PathBuf,
    inner: Arc<Mutex<PassFileInner>>,
}

impl PassFile {
    fn new(path: &Path) -> Result<Self, Error> {
        let file = std::fs::OpenOptions::new().write(true).read(true).create(true).open(path)?;

        let stat = file.metadata()?;

        Ok(PassFile {
            filepath: path.to_owned(),
            inner: Arc::new(Mutex::new(PassFileInner{size: stat.len(), file: file})),
        })
    }
}

impl VFSFile for PassFile {

    fn filepath(&self) -> PathBuf {
        self.filepath.clone()
    }

    fn close(&self) -> Result<(), Error> {
        //trace!("f=close,filepath={:?}", self.filepath);

        Ok(())
    }
    fn read(&self, buf: &mut [u8], off: i64) -> Result<usize, Error> {
        //trace!("f=read,filepath={:?},off={},len={}", self.filepath, off, buf.len());

        let inner = self.inner.lock().unwrap();
        let count = inner.file.read_at(buf, off as u64)?;

        Ok(count)
    }

    fn write(&self, buf: &[u8], off: i64) -> Result<usize, Error> {
        //trace!("f=write,filepath={:?},off={},len={}", self.filepath, off, buf.len());

        let mut inner = self.inner.lock().unwrap();
        let count = inner.file.write_at(buf, off as u64)?;

        let new_size = off + count as i64;
        inner.size = inner.size.max(new_size as u64);

        Ok(count)
    }

    fn sync(&self, _flags: SyncFlags) -> Result<(), Error> {
        //trace!("f=sync,filepath={:?},flags={}", self.filepath, flags);
        let mut inner = self.inner.lock().unwrap();

        inner.file.flush()?;
        Ok(())
    }

    fn truncate(&self, size: i64) -> Result<(), Error> {
        //trace!("f=truncate,filepath={:?},size={}", self.filepath, size);
        let inner = self.inner.lock().unwrap();
        inner.file.set_len(size as u64)?;
        Ok(())
    }

    fn size(&self) -> Result<i64, Error> {
        //trace!("f=size,filepath={:?}", self.filepath);

        let inner = self.inner.lock().unwrap();
        Ok(inner.size as i64)
    }
}

