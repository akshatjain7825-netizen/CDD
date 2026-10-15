use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatus {
    pub total_units: u64,
    pub total_functions: u64,
    pub total_sessions: u64,
    pub db_size_bytes: u64,
}

pub struct CompilationCache {
    db_path: PathBuf,
}

impl CompilationCache {
    pub fn new(cache_dir: impl AsRef<Path>) -> Result<Self> {
        let dir = cache_dir.as_ref();
        std::fs::create_dir_all(dir).ok();
        let db_path = dir.join("cache.db");
        let cache = Self { db_path };
        cache.init_schema()?;
        Ok(cache)
    }

    fn open(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            "#,
        )?;
        Ok(conn)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.open()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS build_sessions (
                session_id          TEXT PRIMARY KEY,
                target_triple       TEXT NOT NULL,
                opt_level           TEXT NOT NULL,
                debug_mode          INTEGER NOT NULL,
                output_wasm_blake3  TEXT NOT NULL,
                duration_ms         INTEGER NOT NULL,
                created_at_epoch_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS compilation_units (
                unit_id             INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path           TEXT NOT NULL UNIQUE,
                source_blake3_hash  TEXT NOT NULL,
                compiler_flags_hash TEXT NOT NULL,
                wasm_artifact_blob  BLOB NOT NULL,
                last_compiled_ms    INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_units_lookup
                ON compilation_units(file_path, source_blake3_hash, compiler_flags_hash);

            CREATE TABLE IF NOT EXISTS cached_ssa_functions (
                func_id                 INTEGER PRIMARY KEY AUTOINCREMENT,
                unit_id                 INTEGER NOT NULL REFERENCES compilation_units(unit_id) ON DELETE CASCADE,
                func_name               TEXT NOT NULL,
                cfg_blake3_hash         TEXT NOT NULL,
                compiled_code_sec_bytes BLOB NOT NULL,
                UNIQUE(unit_id, func_name)
            );

            CREATE INDEX IF NOT EXISTS idx_ssa_cfg_hash
                ON cached_ssa_functions(cfg_blake3_hash);

            CREATE TABLE IF NOT EXISTS diagnostic_logs (
                diag_id       INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id    TEXT NOT NULL REFERENCES build_sessions(session_id) ON DELETE CASCADE,
                error_code    TEXT NOT NULL,
                severity      TEXT NOT NULL,
                line_number   INTEGER NOT NULL,
                column_offset INTEGER NOT NULL,
                message       TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    pub fn compute_source_hash(source_utf8: &str) -> String {
        blake3::hash(source_utf8.as_bytes()).to_hex().to_string()
    }

    pub fn compute_flags_hash(target: &str, opt_level: &str, debug: bool, stack_size: u32) -> String {
        let s = format!("{}:{}:{}:{}", target, opt_level, debug, stack_size);
        blake3::hash(s.as_bytes()).to_hex().to_string()
    }

    pub fn lookup_unit(
        &self,
        file_path: &str,
        source_hash: &str,
        flags_hash: &str,
    ) -> Result<Option<Vec<u8>>> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "SELECT wasm_artifact_blob FROM compilation_units
             WHERE file_path = ?1 AND source_blake3_hash = ?2 AND compiler_flags_hash = ?3",
        )?;
        let mut rows = stmt.query(params![file_path, source_hash, flags_hash])?;

        if let Some(row) = rows.next()? {
            let blob: Vec<u8> = row.get(0)?;
            Ok(Some(blob))
        } else {
            Ok(None)
        }
    }

    pub fn store_unit(
        &self,
        file_path: &str,
        source_hash: &str,
        flags_hash: &str,
        wasm_blob: &[u8],
    ) -> Result<()> {
        let conn = self.open()?;
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        conn.execute(
            r#"
            INSERT INTO compilation_units (file_path, source_blake3_hash, compiler_flags_hash, wasm_artifact_blob, last_compiled_ms)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(file_path) DO UPDATE SET
                source_blake3_hash = excluded.source_blake3_hash,
                compiler_flags_hash = excluded.compiler_flags_hash,
                wasm_artifact_blob = excluded.wasm_artifact_blob,
                last_compiled_ms = excluded.last_compiled_ms;
            "#,
            params![file_path, source_hash, flags_hash, wasm_blob, now_ms],
        )?;

        Ok(())
    }

    pub fn record_session(
        &self,
        session_id: &str,
        target: &str,
        opt_level: &str,
        debug_mode: i32,
        output_blake3: &str,
        duration_ms: i64,
    ) -> Result<()> {
        let conn = self.open()?;
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        conn.execute(
            r#"
            INSERT OR REPLACE INTO build_sessions (session_id, target_triple, opt_level, debug_mode, output_wasm_blake3, duration_ms, created_at_epoch_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);
            "#,
            params![session_id, target, opt_level, debug_mode, output_blake3, duration_ms, now_ms],
        )?;

        Ok(())
    }

    pub fn get_status(&self) -> Result<CacheStatus> {
        let conn = self.open()?;

        let total_units: u64 = conn.query_row("SELECT COUNT(*) FROM compilation_units", [], |r| r.get(0))?;
        let total_functions: u64 = conn.query_row("SELECT COUNT(*) FROM cached_ssa_functions", [], |r| r.get(0))?;
        let total_sessions: u64 = conn.query_row("SELECT COUNT(*) FROM build_sessions", [], |r| r.get(0))?;

        let db_size_bytes = std::fs::metadata(&self.db_path).map(|m| m.len()).unwrap_or(0);

        Ok(CacheStatus {
            total_units,
            total_functions,
            total_sessions,
            db_size_bytes,
        })
    }
}
