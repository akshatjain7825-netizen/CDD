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
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            "#,
        )?;
        Ok(conn)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn cache_dir(&self) -> &Path {
        self.db_path.parent().unwrap_or_else(|| Path::new("."))
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

    pub const COMPILER_VERSION: &str = env!("CARGO_PKG_VERSION");

    pub fn compute_source_hash(source_utf8: &str) -> String {
        blake3::hash(source_utf8.as_bytes()).to_hex().to_string()
    }

    pub fn compute_flags_hash(target: &str, opt_level: &str, debug: bool, stack_size: u32) -> String {
        Self::compute_flags_hash_extended(target, opt_level, debug, stack_size, true, true)
    }

    pub fn compute_flags_hash_extended(
        target: &str,
        opt_level: &str,
        debug: bool,
        stack_size: u32,
        emit_stack_check: bool,
        bundle_allocator: bool,
    ) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"wasmc_v:");
        hasher.update(Self::COMPILER_VERSION.as_bytes());
        hasher.update(b"|target:");
        hasher.update(target.as_bytes());
        hasher.update(b"|opt:");
        hasher.update(opt_level.as_bytes());
        hasher.update(b"|debug:");
        hasher.update(if debug { b"1" } else { b"0" });
        hasher.update(b"|stack:");
        hasher.update(&stack_size.to_le_bytes());
        hasher.update(b"|stack_chk:");
        hasher.update(if emit_stack_check { b"1" } else { b"0" });
        hasher.update(b"|bundle_alloc:");
        hasher.update(if bundle_allocator { b"1" } else { b"0" });
        hasher.finalize().to_hex().to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flags_hash_sensitivity() {
        let h1 = CompilationCache::compute_flags_hash_extended("wasm32-unknown-web", "O2", false, 1048576, true, true);
        let h2 = CompilationCache::compute_flags_hash_extended("wasm32-unknown-web", "O0", false, 1048576, true, true);
        let h3 = CompilationCache::compute_flags_hash_extended("wasm32-wasi", "O2", false, 1048576, true, true);
        let h4 = CompilationCache::compute_flags_hash_extended("wasm32-unknown-web", "O2", true, 1048576, true, true);
        let h5 = CompilationCache::compute_flags_hash_extended("wasm32-unknown-web", "O2", false, 65536, true, true);
        let h6 = CompilationCache::compute_flags_hash_extended("wasm32-unknown-web", "O2", false, 1048576, false, true);
        let h7 = CompilationCache::compute_flags_hash_extended("wasm32-unknown-web", "O2", false, 1048576, true, false);

        assert_ne!(h1, h2, "opt_level change must invalidate hash");
        assert_ne!(h1, h3, "target change must invalidate hash");
        assert_ne!(h1, h4, "debug mode change must invalidate hash");
        assert_ne!(h1, h5, "stack size change must invalidate hash");
        assert_ne!(h1, h6, "emit_stack_check change must invalidate hash");
        assert_ne!(h1, h7, "bundle_allocator change must invalidate hash");
    }

    #[test]
    fn test_sqlite_cache_lifecycle_and_wal_timeout() {
        let temp_dir = std::env::temp_dir().join(format!("wasmc_cache_unit_{}", std::process::id()));
        let cache = CompilationCache::new(&temp_dir).expect("init cache");

        let file = "main.src";
        let src_hash = CompilationCache::compute_source_hash("fn main() {}");
        let flags_hash = CompilationCache::compute_flags_hash("wasm32-unknown-web", "O2", false, 1048576);
        let wasm_blob = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

        // Miss before store
        let miss = cache.lookup_unit(file, &src_hash, &flags_hash).unwrap();
        assert!(miss.is_none());

        // Store
        cache.store_unit(file, &src_hash, &flags_hash, &wasm_blob).unwrap();

        // Hit
        let hit = cache.lookup_unit(file, &src_hash, &flags_hash).unwrap();
        assert_eq!(hit, Some(wasm_blob.clone()));

        // Invalidation when flags hash changes (e.g., version or opt level)
        let other_flags = CompilationCache::compute_flags_hash("wasm32-unknown-web", "O0", false, 1048576);
        let invalid = cache.lookup_unit(file, &src_hash, &other_flags).unwrap();
        assert!(invalid.is_none());

        // Status
        let status = cache.get_status().unwrap();
        assert_eq!(status.total_units, 1);
        assert!(status.db_size_bytes > 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

