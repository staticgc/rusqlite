
use libsqlite3_sys::{self, sqlite3_vfs, sqlite3_file};
use std::path::{PathBuf};
use std::os::raw::{c_char, c_int, c_void};
use std::ffi::{CString, CStr};
use crate::error::Error;
use crate::OpenFlags;

use crate::vfs::{AccessMode, VFS};
use crate::vfs::file_capi::FileWrapper;


fn cpath_to_rust(ptr: *const c_char) -> PathBuf {
    let c_str = unsafe {CStr::from_ptr(ptr)};
    let rs_str = c_str.to_str().unwrap();

    PathBuf::from(rs_str)
}

#[no_mangle]
pub extern "C" fn vfs_open(vfs: *mut sqlite3_vfs, name: *const c_char, file_ptr: *mut sqlite3_file,
    flags: c_int, outflags: *mut c_int) -> c_int {

    let filepath = cpath_to_rust(name);
    //println!("f=vfs_open,filepath={:?}", &filepath);

    let fs = get_user_vfs(vfs);
    let sqlite_flags = unsafe{OpenFlags::from_bits_unchecked(flags)};

    let rc = match fs.open(&filepath, sqlite_flags) {
        Ok(user_file) => {
            FileWrapper::set_user_file(file_ptr, Box::new(user_file));
            if !outflags.is_null() {
                unsafe{*outflags = flags;}
            }
            0
        },
        Err(_e) => {
            //println!("f=vfs_open,err={:?}", e);
            //TODO: raise proper error code
            1
        }
    };
    std::mem::forget(fs);
    rc 
}


#[no_mangle]
pub extern "C" fn vfs_randomness(_vfs: *mut sqlite3_vfs, _n_byte: c_int, _zbyte: *mut c_char) -> c_int {
    //println!("f=vfs_randomness");
    0
}

#[no_mangle]
pub extern "C" fn vfs_fullpath(vfs: *mut sqlite3_vfs, filepath: *const c_char, n_out_size: c_int, path_out: *mut c_char) -> c_int {
    //println!("f=vfs_fullpath");
    let fs = get_user_vfs(vfs);
    let rust_path = cpath_to_rust(filepath);
    let fullpath = fs.fullpath(&rust_path).unwrap();

    let os_str = fullpath.into_os_string();
    let rs_str = os_str.to_str().unwrap();
    let rs_buf = rs_str.as_bytes();

    let buf = unsafe{std::slice::from_raw_parts_mut(path_out as *mut u8, n_out_size as usize)};
    buf[..rs_buf.len()].copy_from_slice(rs_buf);
    buf[rs_str.len()+1] = 0;

    std::mem::forget(fs);
    0
}

#[no_mangle]
pub extern "C" fn vfs_delete(vfs: *mut sqlite3_vfs, filepath: *const c_char, dir_sync: c_int) -> c_int {
    //println!("f=vfs_delete");
    let fs = get_user_vfs(vfs);
    let rust_path = cpath_to_rust(filepath);
    let sync_flag = if dir_sync > 0 {true}else{false};
    let rc = match fs.remove(&rust_path, sync_flag) {
        Ok(()) => {0},
        Err(_e) => {
            //println!("f=vfs_delete,err={:?}", e);
            //TODO: raise proper error code
            1
        }
    };
    std::mem::forget(fs);
    rc 
}

#[no_mangle]
pub extern "C" fn vfs_access(vfs: *mut sqlite3_vfs, filepath: *const c_char, flags: c_int, out: *mut c_int) -> c_int {
    let rust_path = cpath_to_rust(filepath);
    //println!("f=vfs_access,filepath={:?},mode={}", rust_path, flags);
    let fs = get_user_vfs(vfs);

    let mode = match flags {
        0 => AccessMode::Exists,
        1 => AccessMode::ReadWrite,
        2 => AccessMode::Read,
        _ => {
            //println!("f=vfs_access,err=Unknown access mode");
            std::mem::forget(fs);
            return 1;
        }
    };

    let rc = match fs.access(&rust_path, mode) {
        Ok(res) => {
            unsafe {*out = if res {1}else{0};}
            //println!("f=vfs_access,filepath={:?},mode={},result={}", rust_path, flags, res);
            0
        },
        Err(_e) => {
            //println!("f=vfs_access,err={:?}", e);
            //TODO: raise proper error code
            1
        }
    };

    std::mem::forget(fs);
    rc
}


#[no_mangle]
pub extern "C" fn vfs_dlopen(_vfs: *mut sqlite3_vfs, _filepath: *const c_char) -> *mut c_void{
    //println!("f=vfs_dlopen");
    std::ptr::null_mut() as *mut c_void
}

#[no_mangle]
pub extern "C" fn vfs_dlsym(_vfs: *mut sqlite3_vfs, _handle: *mut c_void, _sym: *const c_char) -> Option<unsafe extern "C" fn()>{
    //println!("f=vfs_dlsym");
    None
}

#[no_mangle]
pub extern "C" fn vfs_dlerror(_vfs: *mut sqlite3_vfs, _n: c_int, _sym: *mut c_char) {
    //println!("f=vfs_dlerror");
}

#[no_mangle]
pub extern "C" fn vfs_dlclose(_vfs: *mut sqlite3_vfs, _handle: *mut c_void) {
    //println!("f=vfs_dlclose");
}

#[no_mangle]
pub extern "C" fn vfs_sleep(vfs: *mut sqlite3_vfs, microseconds: c_int) -> c_int {
    //println!("f=vfs_access,interval={}", microseconds);
    let fs = get_user_vfs(vfs);
    let ret = fs.sleep(microseconds);

    std::mem::forget(fs);
    ret
}

#[no_mangle]
pub extern "C" fn vfs_current_time(vfs: *mut sqlite3_vfs, interval: *mut f64) -> c_int {
    //println!("f=vfs_current_time");
    let fs = get_user_vfs(vfs);

    unsafe {*interval = fs.current_time();}

    std::mem::forget(fs);
    0
}

fn get_user_vfs(ptr: *mut sqlite3_vfs) -> Box<Box<dyn VFS>> {
    let fs: Box<Box<dyn VFS>> = unsafe{Box::from_raw((*ptr).pAppData as *mut Box<dyn VFS>)}; 
    fs
}

/// register_vfs registers the given VFS instance with sqlite
/// 
/// The `is_default=true` means the given vfs will be used when none is specified during
/// creating a new connection
///
/// ```rust,no_run
/// struct MyFS {}
/// impl VFS for MyFS {
///    fn name(&self) -> String { "myfs".to_owned() }
///    fn open(&self, filepath: &Path, flags: OpenFlags) -> Result<Box<dyn VFSFile>, Error> {
///         Ok(Box::new(MyFile{filepath: filepath.to_owned()}))
///    }
///    ...
/// }
/// 
/// struct MyFile {
///     filepath: PathBuf,
///     ...
/// }
///
/// impl VFSFile for MyFile {
///     ...
/// }
/// fn some_function() -> Result<(), Error> {
///     let fs = Box::new(MyFS{});
///     rusqlite::vfs::register_vfs(fs)?;
///     Ok(())
/// }
/// ```
///
pub fn register_vfs(fs: Box<dyn VFS>, is_default: bool) -> Result<(), Error> {
    let fs_name = fs.name();
    //println!("f=register_vfs,fs_name={}", &fs_name);

    let tmpfs = Box::new(fs);
    let ptr = Box::into_raw(tmpfs);

    let c_name = CString::new(fs_name.as_str()).unwrap();
    let c_name = c_name.into_raw();

    let vfs = sqlite3_vfs {
        iVersion: 1,
        szOsFile: std::mem::size_of::<FileWrapper>() as c_int,
        mxPathname: 256,
        pNext: std::ptr::null_mut(),
        zName: c_name as *const c_char,
        pAppData: ptr as *mut c_void,
        xOpen: Some(vfs_open),
        xAccess: Some(vfs_access),
        xDelete: Some(vfs_delete),
        xFullPathname: Some(vfs_fullpath),
        xDlOpen: Some(vfs_dlopen),
        xDlError: Some(vfs_dlerror),
        xDlSym: Some(vfs_dlsym),
        xDlClose: Some(vfs_dlclose),
        xRandomness: Some(vfs_randomness),
        xSleep: Some(vfs_sleep),
        xCurrentTime: Some(vfs_current_time),
        xGetLastError: None,
    };

    let vfs_ptr = Box::into_raw(Box::new(vfs));
    let default_flag = if is_default{1}else{0};
    let rc = unsafe {
        libsqlite3_sys::sqlite3_vfs_register(vfs_ptr, default_flag)
    };

    if rc != 0 {
        Err(Error::SqliteFailure(crate::ffi::Error::new(rc), None))
    }else{
        Ok(())
    }
}

