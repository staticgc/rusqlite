
use std::os::raw::{c_int, c_void};
use libsqlite3_sys::{self, sqlite3_file, sqlite3_io_methods, sqlite3_int64};

use crate::vfs::{VFSFile, SyncFlags};


#[repr(C)]
pub(crate) struct FileWrapper {
    f: sqlite3_file,
    user_file: *mut c_void,
}

impl FileWrapper {
    pub(crate) fn get_user_file(ptr: *mut sqlite3_file) -> Box<Box<dyn VFSFile>> {
        let wrapper_ptr = ptr as *mut FileWrapper;

        let file_box: Box<Box<dyn VFSFile>> = unsafe {
            Box::from_raw((*wrapper_ptr).user_file as *mut Box<dyn VFSFile>)
        };
        file_box
    }

    pub(crate) fn set_user_file(ptr: *mut sqlite3_file, user_file: Box<Box<dyn VFSFile>>) {
        let wrapper_ptr = unsafe{std::mem::transmute::<*mut sqlite3_file, *mut FileWrapper>(ptr)};

        //wrap into one more box
        let rs_file_ptr = Box::into_raw(user_file) as *mut c_void;

        unsafe {
            (*wrapper_ptr).user_file = rs_file_ptr;

            let methods = Box::into_raw(Box::new(sqlite3_io_methods{
                iVersion: 1,
                xClose: Some(vfs_close),
                xRead: Some(vfs_read),
                xWrite: Some(vfs_write),
                xTruncate: Some(vfs_truncate),
                xSync: Some(vfs_sync),
                xFileSize: Some(vfs_file_size),
                xLock: Some(vfs_lock),
                xUnlock: Some(vfs_lock),
                xCheckReservedLock: Some(vfs_check_reserved_lock),
                xFileControl: Some(vfs_file_control),
                xSectorSize: Some(vfs_sector_size),
                xDeviceCharacteristics: Some(vfs_device_characteristics),                
            }));
            
            (*wrapper_ptr).f.pMethods = methods;
        }
    }
}

#[no_mangle]
pub(crate) extern "C" fn vfs_close(file_ptr: *mut sqlite3_file) -> c_int {
    let f = FileWrapper::get_user_file(file_ptr);
    //println!("f=vfs_close,filepath={:?}", f.filepath());
    let rc = match f.close() {
        Ok(_) => {0},
        Err(_e) => {
            //println!("f=vfs_close,err={}", e);
            //TODO: raise proper error code
            libsqlite3_sys::SQLITE_IOERR_READ
        }
    };

    rc
}

#[no_mangle]
pub(crate) extern "C" fn vfs_read(file_ptr: *mut sqlite3_file, data: *mut c_void, i_amt: c_int, off: sqlite3_int64) -> c_int {
    let buf = unsafe{std::slice::from_raw_parts_mut(data as *mut u8, i_amt as usize)};
    //println!("f=vfs_read,off={},len={}", off, i_amt);

    let f = FileWrapper::get_user_file(file_ptr);

    let rc = match f.read(buf, off as i64) {
        Ok(r) => {
            //println!("f=vfs_read,r={},buflen={}", r, buf.len());
            if r < buf.len() {
                libsqlite3_sys::SQLITE_IOERR_SHORT_READ
            }else{
                0
            }
        },
        Err(_e) => {
            //println!("f=vfs_read,err={:?}", e);
            //TODO: raise proper error code
            libsqlite3_sys::SQLITE_IOERR_READ
        }
    };

    std::mem::forget(f);
    rc
}

#[no_mangle]
pub(crate) extern "C" fn vfs_write(file_ptr: *mut sqlite3_file, data: *const c_void, i_amt: c_int, off: sqlite3_int64) -> c_int {
    //println!("f=vfs_write,off={},buflen={}", off, i_amt);
    let buf = unsafe{std::slice::from_raw_parts(data as *const u8, i_amt as usize)};

    let f = FileWrapper::get_user_file(file_ptr);
    let rc = match f.write(buf, off as i64) {
        Ok(_) => {0},
        Err(_e) => {
            //println!("f=vfs_write,err={}", e);
            //TODO: raise proper error code
            libsqlite3_sys::SQLITE_IOERR_READ
        }
    };

    std::mem::forget(f);
    rc
}

#[no_mangle]
pub(crate) extern "C" fn vfs_truncate(file_ptr: *mut sqlite3_file, size: sqlite3_int64) -> c_int {
    //println!("f=vfs_truncate,size={}", size);

    let f = FileWrapper::get_user_file(file_ptr);
    let rc = match f.truncate(size) {
        Ok(()) => 0,
        Err(_e) => {
            //println!("f=vfs_truncate,err={}", e);
            //TODO: raise proper error code
            libsqlite3_sys::SQLITE_IOERR_READ
        }
    };

    std::mem::forget(f);
    rc
}

#[no_mangle]
pub(crate) extern "C" fn vfs_sync(file_ptr: *mut sqlite3_file, flags: c_int) -> c_int {
    //println!("f=vfs_sync");

    let f = FileWrapper::get_user_file(file_ptr);
    let sync_flags = unsafe{
        SyncFlags::from_bits_unchecked(flags)
    };

    let rc = match f.sync(sync_flags) {
        Ok(()) => {0},
        Err(_e) => {
            //println!("f=vfs_sync,err={}", e);
            //TODO: raise proper error code
            1
        }
    };

    std::mem::forget(f);
    rc
}

#[no_mangle]
pub(crate) extern "C" fn vfs_file_size(file_ptr: *mut sqlite3_file, p_size: *mut sqlite3_int64) -> c_int {
    //println!("f=vfs_file_size");
    let f = FileWrapper::get_user_file(file_ptr);

    let rc = match f.size() {
        Ok(sz) => {
            unsafe{*p_size = sz;}
            0
        },
        Err(_e) => {
            //println!("f=vfs_size,err={}", e);
            //TODO: raise proper error code
            1
        }
    };

    std::mem::forget(f);
    rc
}

#[no_mangle]
pub(crate) extern "C" fn vfs_lock(_file_ptr: *mut sqlite3_file, _arg2: c_int) -> c_int {
    0
}

#[no_mangle]
pub(crate) extern "C" fn vfs_unlock(_file_ptr: *mut sqlite3_file, _arg2: c_int) -> c_int {
    0
}

#[no_mangle]
pub(crate) extern "C" fn vfs_check_reserved_lock(_file_ptr: *mut sqlite3_file, _p_resout: *mut c_int) -> c_int {
    0
}

#[no_mangle]
pub(crate) extern "C" fn vfs_file_control(_file_ptr: *mut sqlite3_file, _op: c_int, _p_arg: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub(crate) extern "C" fn vfs_sector_size(file_ptr: *mut sqlite3_file) -> c_int {
    //println!("f=vfs_sector_size");
    let f = FileWrapper::get_user_file(file_ptr);

    let rc = f.sector_size();
    std::mem::forget(f);
    rc
}

#[no_mangle]
pub(crate) extern "C" fn vfs_device_characteristics(file_ptr: *mut sqlite3_file) -> c_int {
    //println!("f=vfs_device_characteristics");
    let f = FileWrapper::get_user_file(file_ptr);

    let rc = f.device_characteristics();
    std::mem::forget(f);
    rc
}


