use crate::errors::KyuubiErr;

use std::path::PathBuf;
use {
    sled::Db,
    std::{
        ffi::OsStr,
        mem::{self, MaybeUninit},
        path::Path,
        ptr::null_mut,
        sync::{
            atomic::{
                AtomicPtr,
                Ordering::{Acquire, Release},
            },
            Arc,
        },
    },
};
pub(crate) async fn get_or_init<T: AsRef<Path> + AsRef<OsStr>>(
    src: Option<T>,
) -> Result<&'static mut Db, KyuubiErr> {
    let src: PathBuf = match src.as_ref() {
        Some(p) => <T as AsRef<Path>>::as_ref(p).to_path_buf(),
        None => PathBuf::from("kyuubi.db"),
    };

    if src.exists() {
        let db: &'static mut Db = match open_disk_db(&src).await {
            Ok(db) => db,
            Err(_) => return Err(KyuubiErr::DbReadError(src.to_string_lossy().to_string())),
        };
        Ok(db)
    } else {
        let db: &'static mut Db = match create_disk_db(&src).await {
            Ok(db) => db,
            Err(_) => return Err(KyuubiErr::WriteError(src.to_string_lossy().to_string())),
        };
        Ok(db)
    }
}

async fn create_disk_db(src: &PathBuf) -> Result<&'static mut Db, KyuubiErr> {
    static PTR: AtomicPtr<Db> = AtomicPtr::new(null_mut());
    let mut db = unsafe { PTR.load(Acquire) };
    if db.is_null() {
        let it = sled::Config::default()
            .path(src)
            .mode(sled::Mode::HighThroughput)
            .create_new(true)
            .open()
            .map_err(KyuubiErr::SledError)?;
        db = Box::into_raw(Box::new(it));
        if let Err(e) = PTR.compare_exchange(null_mut(), db, Release, Acquire) {
            drop(unsafe { Box::from_raw(db) });
            db = e;
        }
    }
    unsafe { Ok(&mut *db) }
}

async fn open_disk_db(src: &PathBuf) -> Result<&'static mut Db, KyuubiErr> {
    static PTR: AtomicPtr<Db> = AtomicPtr::new(null_mut());
    let mut db = unsafe { PTR.load(Acquire) };
    if db.is_null() {
        let it = sled::open(src).map_err(KyuubiErr::SledError)?;
        db = Box::into_raw(Box::new(it));
        if let Err(e) = PTR.compare_exchange(null_mut(), db, Release, Acquire) {
            drop(unsafe { Box::from_raw(db) });
            db = e;
        }
    }
    unsafe { Ok(&mut *db) }
}
