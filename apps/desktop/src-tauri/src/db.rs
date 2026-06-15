//! 自写 rusqlite + SQLCipher 桥，替代 tauri-plugin-sql。
//! - 单连接 `Mutex<Option<Connection>>`（可在多写时用一把事务，修连接池放弃事务的老债）。
//! - 占位符把 sqlx 风格 `$1..$N` 翻成 SQLite 的 `?1..?N`。
//! - 行按 column_name 映射成 {列名:值} JSON，形状与 tauri-plugin-sql 一致，让 web 层 schema.ts 零改。
//! 阶段 1a：开库不带 key（明文），行为与原 tauri-plugin-sql 一致。加密在阶段 2 由 db_open 带 key 接入。
use rusqlite::types::Value as SqlValue;
use rusqlite::Connection;
use serde_json::{Map, Value as Json};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

/// 单连接状态（由 db_open 填充）。
pub struct Db(pub Mutex<Option<Connection>>);

#[derive(serde::Deserialize)]
pub struct Stmt {
    sql: String,
    #[serde(default)]
    params: Vec<Json>,
}

/// `$1..$N`（sqlx 风格） → `?1..?N`（SQLite/rusqlite）。只对 `$` 紧跟数字时替换，其余原样。
fn translate(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' && chars.peek().is_some_and(|d| d.is_ascii_digit()) {
            out.push('?');
        } else {
            out.push(c);
        }
    }
    out
}

/// JSON 入参 → rusqlite 绑定值。
fn bind(v: &Json) -> SqlValue {
    match v {
        Json::Null => SqlValue::Null,
        Json::Bool(b) => SqlValue::Integer(i64::from(*b)),
        Json::Number(n) => n
            .as_i64()
            .map(SqlValue::Integer)
            .unwrap_or_else(|| SqlValue::Real(n.as_f64().unwrap_or(0.0))),
        Json::String(s) => SqlValue::Text(s.clone()),
        // 数组/对象不应作为参数出现；防御性地序列化为文本。
        other => SqlValue::Text(other.to_string()),
    }
}

/// rusqlite 列值 → JSON（对齐 tauri-plugin-sql：INTEGER/REAL→number、TEXT→string、NULL→null）。
fn json_of(v: SqlValue) -> Json {
    match v {
        SqlValue::Null => Json::Null,
        SqlValue::Integer(i) => Json::from(i),
        SqlValue::Real(f) => serde_json::Number::from_f64(f).map_or(Json::Null, Json::Number),
        SqlValue::Text(s) => Json::String(s),
        SqlValue::Blob(b) => Json::Array(b.into_iter().map(Json::from).collect()),
    }
}

fn run_select(conn: &Connection, sql: &str, params: &[Json]) -> rusqlite::Result<Vec<Json>> {
    let mut stmt = conn.prepare(&translate(sql))?;
    let names: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();
    let n = names.len();
    let bound: Vec<SqlValue> = params.iter().map(bind).collect();
    let rows = stmt.query_map(rusqlite::params_from_iter(bound), |row| {
        let mut obj = Map::with_capacity(n);
        for (i, name) in names.iter().enumerate() {
            obj.insert(name.clone(), json_of(row.get::<_, SqlValue>(i)?));
        }
        Ok(Json::Object(obj))
    })?;
    rows.collect()
}

fn run_exec(conn: &Connection, sql: &str, params: &[Json]) -> rusqlite::Result<()> {
    let tsql = translate(sql);
    if params.is_empty() {
        // 迁移 DDL / PRAGMA：execute_batch 容忍多语句并丢弃结果集（如 journal_mode 的返回行）。
        conn.execute_batch(&tsql)
    } else {
        let bound: Vec<SqlValue> = params.iter().map(bind).collect();
        conn.execute(&tsql, rusqlite::params_from_iter(bound)).map(|_| ())
    }
}

// ---- commands ----

/// 打开（或创建）本地库，PRAGMA key 必须为开库第一条；随后 WAL/外键/busy_timeout。
/// path 形如 'sqlite:heng.db'，相对应用配置目录（与原 tauri-plugin-sql 同一位置，保留既有数据）。
#[tauri::command]
pub fn db_open(app: AppHandle, db: State<Db>, path: String, key: Option<String>) -> Result<(), String> {
    let file = path.strip_prefix("sqlite:").unwrap_or(&path);
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let full = dir.join(file);
    let conn = Connection::open(&full).map_err(|e| e.to_string())?;
    if let Some(k) = key {
        conn.pragma_update(None, "key", &k).map_err(|e| e.to_string())?; // 必须第一条
    }
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
        .map_err(|e| e.to_string())?;
    *db.0.lock().unwrap() = Some(conn);
    Ok(())
}

#[tauri::command]
pub fn db_select(db: State<Db>, sql: String, params: Vec<Json>) -> Result<Vec<Json>, String> {
    let guard = db.0.lock().unwrap();
    let conn = guard.as_ref().ok_or("db not open")?;
    run_select(conn, &sql, &params).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn db_execute(db: State<Db>, sql: String, params: Vec<Json>) -> Result<(), String> {
    let guard = db.0.lock().unwrap();
    let conn = guard.as_ref().ok_or("db not open")?;
    run_exec(conn, &sql, &params).map_err(|e| e.to_string())
}

fn run_batch(conn: &mut Connection, stmts: &[Stmt]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for s in stmts {
        let bound: Vec<SqlValue> = s.params.iter().map(bind).collect();
        tx.execute(&translate(&s.sql), rusqlite::params_from_iter(bound))?;
    }
    tx.commit()
}

/// 多条写在一把事务里（要么全成、要么全不写）。给多写方法用，修「半截交易」老债。
#[tauri::command]
pub fn db_batch(db: State<Db>, stmts: Vec<Stmt>) -> Result<(), String> {
    let mut guard = db.0.lock().unwrap();
    let conn = guard.as_mut().ok_or("db not open")?;
    run_batch(conn, &stmts).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn db_close(db: State<Db>) -> Result<(), String> {
    *db.0.lock().unwrap() = None; // drop 即关闭连接
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_placeholders() {
        assert_eq!(translate("WHERE id=$1 AND book_id=$2"), "WHERE id=?1 AND book_id=?2");
        assert_eq!(translate("IN ($1, $2, $10)"), "IN (?1, ?2, ?10)");
        assert_eq!(translate("no params"), "no params");
        assert_eq!(translate("PRAGMA user_version = 5"), "PRAGMA user_version = 5");
    }

    #[test]
    fn binds_and_reads_back() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t(a TEXT, b INTEGER, c REAL, d TEXT)").unwrap();
        run_exec(
            &conn,
            "INSERT INTO t(a,b,c,d) VALUES ($1,$2,$3,$4)",
            &[Json::String("x".into()), Json::from(7i64), Json::from(1.5f64), Json::Null],
        )
        .unwrap();
        let rows = run_select(&conn, "SELECT * FROM t WHERE b=$1", &[Json::from(7i64)]).unwrap();
        assert_eq!(rows.len(), 1);
        let o = rows[0].as_object().unwrap();
        assert_eq!(o["a"], Json::String("x".into()));
        assert_eq!(o["b"], Json::from(7i64));
        assert_eq!(o["c"], Json::from(1.5f64));
        assert_eq!(o["d"], Json::Null);
    }

    #[test]
    fn batch_is_atomic() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t(id TEXT PRIMARY KEY)").unwrap();
        // 成功：两条都写入
        run_batch(
            &mut conn,
            &[
                Stmt { sql: "INSERT INTO t(id) VALUES ($1)".into(), params: vec![Json::from("a")] },
                Stmt { sql: "INSERT INTO t(id) VALUES ($1)".into(), params: vec![Json::from("b")] },
            ],
        )
        .unwrap();
        let n: i64 = conn.query_row("SELECT count(*) FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 2);
        // 失败回滚：'c' 本会插入，但随后 'a' 重复主键报错 → 整批回滚、'c' 不留
        let res = run_batch(
            &mut conn,
            &[
                Stmt { sql: "INSERT INTO t(id) VALUES ($1)".into(), params: vec![Json::from("c")] },
                Stmt { sql: "INSERT INTO t(id) VALUES ($1)".into(), params: vec![Json::from("a")] },
            ],
        );
        assert!(res.is_err());
        let n2: i64 = conn.query_row("SELECT count(*) FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(n2, 2, "rollback: 'c' 不应持久化");
    }
}
