//! Local SQLite case store. Sensitive columns are encrypted at the field level.
use crate::crypto::Crypto;
use crate::output::Output;
use rusqlite::{params, Connection};
use serde::Serialize;

pub struct Store {
    conn: Connection,
    crypto: Crypto,
}

#[derive(Clone, Debug, Serialize)]
pub struct Case {
    pub id: i64,
    pub name: String,
    pub notes: Vec<String>,
    pub targets: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub id: i64,
    pub case_id: i64,
    pub module: String,
    pub target: String,
    pub title: String,
    pub source_url: String,
    pub collected_at: String,
    pub verified: bool,
    pub note: String,
    pub data: Output,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

impl Store {
    pub fn open(path: &std::path::Path, crypto: Crypto) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("cannot open case database: {e}"))?;
        Self::init(conn, crypto)
    }

    #[cfg(test)]
    pub fn memory(crypto: Crypto) -> Self {
        Self::init(Connection::open_in_memory().unwrap(), crypto).unwrap()
    }

    fn init(conn: Connection, crypto: Crypto) -> Result<Self, String> {
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS cases (
               id INTEGER PRIMARY KEY, name TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS targets (
               id INTEGER PRIMARY KEY, case_id INTEGER NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
               value TEXT NOT NULL, added_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS notes (
               id INTEGER PRIMARY KEY, case_id INTEGER NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
               body TEXT NOT NULL, added_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS findings (
               id INTEGER PRIMARY KEY, case_id INTEGER NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
               module TEXT NOT NULL, target TEXT NOT NULL, title TEXT NOT NULL, source_url TEXT NOT NULL,
               collected_at TEXT NOT NULL, verified INTEGER NOT NULL DEFAULT 0, note TEXT NOT NULL, data TEXT NOT NULL);
             CREATE INDEX IF NOT EXISTS idx_findings_case ON findings(case_id);",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self { conn, crypto })
    }

    fn e(&self, s: &str) -> String {
        self.crypto.encrypt(s)
    }
    fn d(&self, s: String) -> String {
        self.crypto.decrypt(&s).unwrap_or_else(|_| "[unreadable]".into())
    }

    pub fn create_case(&self, name: &str) -> Result<i64, String> {
        let t = now();
        self.conn
            .execute("INSERT INTO cases(name,created_at,updated_at) VALUES(?1,?2,?2)", params![self.e(name), t])
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_cases(&self) -> Result<Vec<(i64, String, String, i64)>, String> {
        let mut st = self
            .conn
            .prepare("SELECT c.id,c.name,c.updated_at,(SELECT COUNT(*) FROM findings f WHERE f.case_id=c.id) FROM cases c ORDER BY c.updated_at DESC, c.id DESC")
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?)))
            .map_err(|e| e.to_string())?;
        let mut v = vec![];
        for r in rows {
            let (id, n, u, c) = r.map_err(|e| e.to_string())?;
            v.push((id, self.d(n), u, c));
        }
        Ok(v)
    }

    pub fn get_case(&self, id: i64) -> Result<Case, String> {
        let (name, created, updated): (String, String, String) = self
            .conn
            .query_row("SELECT name,created_at,updated_at FROM cases WHERE id=?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|_| format!("case {id} not found"))?;
        let col = |sql: &str| -> Result<Vec<String>, String> {
            let mut st = self.conn.prepare(sql).map_err(|e| e.to_string())?;
            let it = st.query_map([id], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
            let mut v = vec![];
            for x in it {
                v.push(self.d(x.map_err(|e| e.to_string())?));
            }
            Ok(v)
        };
        Ok(Case {
            id,
            name: self.d(name),
            notes: col("SELECT body FROM notes WHERE case_id=?1 ORDER BY id")?,
            targets: col("SELECT value FROM targets WHERE case_id=?1 ORDER BY id")?,
            created_at: created,
            updated_at: updated,
        })
    }

    fn touch(&self, id: i64) {
        let _ = self.conn.execute("UPDATE cases SET updated_at=?1 WHERE id=?2", params![now(), id]);
    }

    pub fn rename_case(&self, id: i64, name: &str) -> Result<(), String> {
        self.conn.execute("UPDATE cases SET name=?1 WHERE id=?2", params![self.e(name), id]).map_err(|e| e.to_string())?;
        self.touch(id);
        Ok(())
    }

    pub fn delete_case(&self, id: i64) -> Result<(), String> {
        let n = self.conn.execute("DELETE FROM cases WHERE id=?1", [id]).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err(format!("case {id} not found"));
        }
        Ok(())
    }

    pub fn add_target(&self, id: i64, v: &str) -> Result<(), String> {
        self.conn
            .execute("INSERT INTO targets(case_id,value,added_at) VALUES(?1,?2,?3)", params![id, self.e(v), now()])
            .map_err(|e| e.to_string())?;
        self.touch(id);
        Ok(())
    }

    pub fn add_note(&self, id: i64, v: &str) -> Result<(), String> {
        self.conn
            .execute("INSERT INTO notes(case_id,body,added_at) VALUES(?1,?2,?3)", params![id, self.e(v), now()])
            .map_err(|e| e.to_string())?;
        self.touch(id);
        Ok(())
    }

    pub fn add_finding(&self, case_id: i64, out: &Output, note: &str) -> Result<i64, String> {
        let data = serde_json::to_string(out).map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "INSERT INTO findings(case_id,module,target,title,source_url,collected_at,verified,note,data) VALUES(?1,?2,?3,?4,?5,?6,0,?7,?8)",
                params![
                    case_id,
                    out.module,
                    self.e(&out.target),
                    self.e(&out.title),
                    self.e(&out.sources.join("\n")),
                    out.collected_at,
                    self.e(note),
                    self.e(&data)
                ],
            )
            .map_err(|e| e.to_string())?;
        self.touch(case_id);
        Ok(self.conn.last_insert_rowid())
    }

    pub fn set_verified(&self, case_id: i64, fid: i64, v: bool) -> Result<(), String> {
        let n = self
            .conn
            .execute("UPDATE findings SET verified=?1 WHERE id=?2 AND case_id=?3", params![v as i64, fid, case_id])
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err(format!("finding {fid} not found in this case"));
        }
        self.touch(case_id);
        Ok(())
    }

    pub fn findings(&self, case_id: i64) -> Result<Vec<Finding>, String> {
        let mut st = self
            .conn
            .prepare("SELECT id,module,target,title,source_url,collected_at,verified,note,data FROM findings WHERE case_id=?1 ORDER BY id")
            .map_err(|e| e.to_string())?;
        let rows = st
            .query_map([case_id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?, r.get::<_, i64>(6)?, r.get::<_, String>(7)?, r.get::<_, String>(8)?))
            })
            .map_err(|e| e.to_string())?;
        let mut v = vec![];
        for r in rows {
            let (id, module, target, title, src, at, ver, note, data) = r.map_err(|e| e.to_string())?;
            let data: Output = serde_json::from_str(&self.d(data)).map_err(|e| format!("finding {id} unreadable: {e}"))?;
            v.push(Finding {
                id,
                case_id,
                module,
                target: self.d(target),
                title: self.d(title),
                source_url: self.d(src),
                collected_at: at,
                verified: ver != 0,
                note: self.d(note),
                data,
            });
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Status;

    fn out() -> Output {
        let mut o = Output::new("domain", "example.com", "Domain scan: example.com");
        o.kv("A", "93.184.216.34").item(Status::Ok, "TLS", "valid").source("https://rdap.org/domain/example.com");
        o
    }

    #[test]
    fn case_lifecycle() {
        let s = Store::memory(Crypto::from_key([1; 32]));
        let id = s.create_case("Op Alpha").unwrap();
        s.add_target(id, "example.com").unwrap();
        s.add_note(id, "first note").unwrap();
        let fid = s.add_finding(id, &out(), "check later").unwrap();
        s.set_verified(id, fid, true).unwrap();
        let c = s.get_case(id).unwrap();
        assert_eq!(c.name, "Op Alpha");
        assert_eq!(c.targets, vec!["example.com"]);
        assert_eq!(c.notes, vec!["first note"]);
        let f = s.findings(id).unwrap();
        assert_eq!(f.len(), 1);
        assert!(f[0].verified);
        assert_eq!(f[0].note, "check later");
        s.rename_case(id, "Op Beta").unwrap();
        assert_eq!(s.list_cases().unwrap()[0].1, "Op Beta");
        s.delete_case(id).unwrap();
        assert!(s.get_case(id).is_err());
        assert!(s.findings(id).unwrap().is_empty());
        assert!(s.delete_case(id).is_err());
    }

    #[test]
    fn data_is_encrypted_at_rest() {
        let s = Store::memory(Crypto::from_key([2; 32]));
        let id = s.create_case("secret-name").unwrap();
        s.add_note(id, "secret-note").unwrap();
        let raw: String = s.conn.query_row("SELECT name FROM cases", [], |r| r.get(0)).unwrap();
        assert!(!raw.contains("secret"));
        let raw: String = s.conn.query_row("SELECT body FROM notes", [], |r| r.get(0)).unwrap();
        assert!(!raw.contains("secret"));
    }
}
