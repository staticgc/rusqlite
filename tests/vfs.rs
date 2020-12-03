use std::path::{Path, PathBuf};
use rusqlite::{self, OpenFlags, Connection, Error, params};

struct TestParams {
    dbfile: PathBuf,
}

#[cfg(feature = "vfs")]
#[test]
fn test_passfs_vfs_same_conn() -> Result<(), Error> {
    let p = TestParams {
        dbfile: PathBuf::from("vfstest.db"),
    };

    setup(&p)?;

    let conn = create_conn(&p, false)?;
    write_data(&conn)?;

    let count = get_row_count(&conn)?;
    assert_eq!(4096, count, "row count");

    read_data(&conn)?;

    teardown(&p)?;
    Ok(())
}

#[cfg(feature = "vfs")]
#[test]
fn test_passfs_vfs_diff_conn() -> Result<(), Error> {
    let p = TestParams {
        dbfile: PathBuf::from("vfstest.db"),
    };

    setup(&p)?;

    {
        let conn = create_conn(&p, false)?;
        write_data(&conn)?;
    }

    {
        let conn = create_conn(&p, true)?;
        let count = get_row_count(&conn)?;
        assert_eq!(4096, count, "row count");

        read_data(&conn)?;
    }

    teardown(&p)?;
    Ok(())
}

fn setup(p: &TestParams) -> Result<(), Error> {
    delete_file(&p.dbfile)
}

fn teardown(p: &TestParams) -> Result<(), Error> {
    delete_file(&p.dbfile)
}

fn create_conn(p: &TestParams, readonly: bool) -> Result<Connection, Error> {
    let fs = Box::new(rusqlite::vfs::passfs::PassFS{});
    rusqlite::vfs::register_vfs(fs, false)?;

    let flags = if readonly {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    }else{
        OpenFlags::default()
    };

    let conn = Connection::open_with_flags_and_vfs(&p.dbfile, flags, "passfs")?;
    conn.execute("pragma journal_mode = off;", params![])?;

    Ok(conn)
}

fn write_data(conn: &Connection) -> Result<(), Error> {
    conn.execute("
        create table if not exists nums (
            n integer,
            v text
        );
    ", params![])?;

    let mut stmt = conn.prepare_cached("insert into nums values (?, ?)")?;

    conn.execute("BEGIN", params![])?;
    for n in 0..4096 {
        stmt.execute(params![n, "abc"])?;
    }
    conn.execute("COMMIT", params![])?;
    Ok(())
} 

fn get_row_count(conn: &Connection) -> Result<u32, Error> {
    let mut stmt = conn.prepare("select count(*) from nums")?;
    stmt.query_row(params![], |row| row.get(0))
}

fn read_data(conn: &Connection) -> Result<(), Error> {
    let mut stmt = conn.prepare("select n,v from nums")?;

    let mut rows = stmt.query(params![])?;
    while let Some(r) = rows.next()? {
        let _n: i32 = r.get(0)?;
        let s: String = r.get(1)?;
        assert_eq!("abc", &s);
    }

    Ok(())
} 

fn delete_file(dbfile: &Path) -> Result<(), Error> {
    match std::fs::remove_file(dbfile) {
        Ok(()) => {Ok(())},
        Err(e) => {
            let kind = e.kind();
            if kind != std::io::ErrorKind::NotFound {
                Err(Error::IOError(e))
            }else{
                Ok(())
            }
        }
    }
}
