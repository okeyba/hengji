//! 阶段 2 · 本地加密的密钥层（NCrypt 平台加密提供程序 / PCP）。
//!
//! 架构（Spike 定案＝软路，见 docs/design/spike-results.md）：
//! - 随机 256-bit **DEK** 真正加密 SQLCipher 库（见 db.rs 的 raw-key 开库）。
//! - DEK 被 TPM 里一把**非导出** RSA-2048 封装钥匙（OAEP-SHA256）封住；口令作该钥匙的
//!   `PCP_USAGEAUTH`（使用授权）。解封必须 ① 在本机芯片 ② 输对口令（Spike #2 实测：错口令
//!   被芯片 `NTE_PERM` 拒绝并计入全局 DA 限速）。
//! - 封装产物（密文 + 版本/算法/创建时间/slot）存库同目录的 `heng.dek.tpm`（JSON 信封）。
//! - 改密＝**重封 under 新钥匙**（Spike 定 `PCP_CHANGEPASSWORD` 不可用）：用另一个 slot 建新钥匙、
//!   封同一把 DEK、原子顶替信封、再删旧钥匙（两阶段原子 + 启动自愈，见 reconcile）。
//!
//! 阶段 3（本阶段）新增：set_password/remove_password 含 **明↔密库原子迁移**（§9，sqlcipher_export +
//! 同卷 rename + 迁移标记启动自愈）；db_open 从 Crypto state 取已解锁 DEK 开库（DEK **绝不**回传 JS）；
//! 新增 lock 命令（自动锁/手动锁用）。security_status 先 reconcile 再判定（启动门自愈中断的迁移/改密）。
//! 本阶段边界：
//! - **不**做销毁（错 N 次）/ 备份导出 / kill-9 注入测试硬化（Phase 4）。
//! - **不**做无芯片软件弱版（本期后置）。
//!
//! 安全纪律（来自 Spike #2）：除了**唯一一次**蓄意的错口令强制性测试（1/32 DA strike），
//! 绝不用错口令反复 unwrap。自愈用的是删钥匙（不计 DA），不靠猜口令。
use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::fs::File;
use std::io::Write as _;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};
use zeroize::Zeroizing;

use crate::db::{config_dir, open_db, Db, DB_FILE};

/// DEK → 64 位十六进制（Zeroizing：用完清零）。供 db_open / 命令开 SQLCipher 库时传 raw-key。
/// DEK 本体仍只存 Rust 侧 Crypto state，hex 仅在 Rust 内部短暂存在、绝不回传 JS。
pub(crate) fn dek_hex(dek: &[u8; 32]) -> Zeroizing<String> {
    Zeroizing::new(dek.iter().map(|b| format!("{b:02x}")).collect())
}

/// 已解锁会话状态（仅 Rust 侧，绝不跨 IPC 回传 JS）。
#[derive(Default)]
pub struct CryptoState {
    /// 已解锁的 DEK（Zeroizing：替换/drop 时清零）。
    pub dek: Option<Zeroizing<[u8; 32]>>,
    /// 本次解锁会话内是否成功导出过备份（强闸门：开启销毁前要求本会话内已备份；解锁/锁定时归零）。
    pub backed_up_session: bool,
}

/// 这把锁**串行化所有 crypto 命令**：Tauri 同步命令跑在线程池上，可并发；改密/解锁/移除/销毁都在
/// 固定 slot a/b + 单一 heng.dek.tpm 上做多步非原子操作，若并发会互踩（如改密 commit 与另一路
/// reconcile 的删 slot 撞车 → 封装文件指向的 slot 钥匙被删 = 库永久不可解）。命令全程持此锁互斥。
pub struct Crypto(pub Mutex<CryptoState>);

/// 解锁/封装失败的分流（§5）。UI 据此分屏：
/// - WrongPassword：口令错（芯片 NTE_PERM 拒）→ 计入销毁计数（Phase 4）。
/// - Locked：口令连错过多触发芯片 DA 防爆破锁定（TPM_20_E_LOCKOUT 等）→ 等冷却 / 正确口令复位；
///   **不计销毁**（芯片此时拒绝校验、并未判定口令对错；reboot 不一定解，DA 计数靠时间递减）。
/// - Corrupt：信封/密文损坏 → “数据可能损坏”。
/// - ChipUnavailable：芯片占用/句柄异常/钥匙缺失 → “芯片暂不可用，请重启”，**不计销毁**。
/// - Internal：目录解析/IO/序列化等基建错（非解锁三态）。
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailClass {
    WrongPassword,
    Locked,
    Corrupt,
    ChipUnavailable,
    Internal,
    /// 本次错口令使失败计数达阈值、且已开启销毁 → 数据已销毁。UI 跳终态屏（不再是普通错口令）。
    Destroyed,
}

/// 跨 IPC 回传给 JS 的错误：粗分类 + 原始 HRESULT（供 UI 细化，如锁定 vs 单纯错口令）+ 文案。
#[derive(Serialize, Debug, Clone)]
pub struct CryptoError {
    pub class: FailClass,
    pub code: u32,
    pub message: String,
}

/// 给状态行的判定输入（三态由 scheme + 本期仅 tpm-pcp 推出）+ 备份新鲜度（阶段 4a）。
#[derive(Serialize, Debug, Clone)]
pub struct SecurityStatus {
    /// heng.dek.tpm 信封是否存在。
    pub encrypted: bool,
    /// 封装方案，本期只有 "tpm-pcp"（弱软件版后置）；信封损坏时为 None。
    pub scheme: Option<String>,
    /// 能否打开 PCP 提供程序（§5 解锁前的芯片健康 ping）。
    pub tpm_available: bool,
    /// 上次明文备份的时间（unix 秒）/ 路径（heng.security 记录，锁定态可读）。None＝从未备份。
    pub last_backup_unix: Option<i64>,
    pub last_backup_path: Option<String>,
    /// 数据是否已被销毁（heng.destroyed sentinel 在）。门优先识别 → 终态屏。
    pub destroyed: bool,
    /// 当前失败计数 / 是否开启销毁 / 销毁阈值（解锁屏显"再错 N 次销毁、已错 M 次"；设置卡显开关）。
    pub fail_count: u32,
    pub destroy_enabled: bool,
    pub destroy_threshold: u32,
}

/// export_backup 成功后回传 JS（路径 + 时间 + 行数，供 UI 显示新鲜度）。
#[derive(Serialize, Debug, Clone)]
pub struct BackupInfo {
    pub path: String,
    pub unix: i64,
    pub rows: i64,
}

mod engine {
    //! dir-based 纯引擎：所有 NCrypt FFI + 信封 IO + 明↔密库迁移都在此，便于单测（不依赖 Tauri runtime）。
    use super::*;
    use rusqlite::Connection;
    use sha2::{Digest, Sha256};
    use std::io::Read as _;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Security::Cryptography::*;
    use zeroize::Zeroize;

    use crate::db::DB_FILE;

    const PROVIDER: PCWSTR = w!("Microsoft Platform Crypto Provider");
    // 两个固定 slot：改密时 ping-pong（新钥匙建在另一个 slot，验证+提交后才删旧）。
    const KEY_A: PCWSTR = w!("heng-dek-wrap-a");
    const KEY_B: PCWSTR = w!("heng-dek-wrap-b");
    const PROP_LENGTH: PCWSTR = w!("Length");
    const PROP_EXPORT: PCWSTR = w!("Export Policy");
    const PROP_USAGEAUTH: PCWSTR = w!("PCP_USAGEAUTH");
    const ALG_RSA: PCWSTR = w!("RSA");
    const ALG_SHA256: PCWSTR = w!("SHA256");

    /// 信封文件名（库同目录）。
    pub(super) const ENVELOPE: &str = "heng.dek.tpm";
    /// 改密 staging（写好待提交的新信封；reconcile 据其存在判定“改密未提交”→回滚）。
    const ENVELOPE_NEW: &str = "heng.dek.tpm.new";
    /// 明↔密迁移标记（含方向 + 信封）。存在＝迁移进行中；reconcile 据库文件头判定提交点哪侧后前滚/回滚。
    const MIGRATE_MARKER: &str = "heng.migrate";
    /// 预解锁安全状态（销毁计数/开关 + 上次备份）。锁定态可读，故是 config_dir 明文文件（与信封同列）。
    const SECURITY_FILE: &str = "heng.security";
    /// 错 N 次销毁的阈值（用户拍板 N=5，不加额外节流）。
    pub(super) const DESTROY_THRESHOLD: u32 = 5;
    /// 销毁进行中标记（含隔离目录路径）。reconcile 见之＝前滚补完销毁（单向、不回滚）。
    const DESTROYING_MARKER: &str = "heng.destroying";
    /// 销毁完成 sentinel（终态）。门优先识别 → 终态屏，与"损坏""芯片不可用"彻底分开。
    const DESTROYED_SENTINEL: &str = "heng.destroyed";
    /// 隔离区目录名前缀（密文库销毁前移到这里，密钥删后不可解、仅作墓碑）。
    const QUARANTINE_PREFIX: &str = "heng.destroyed-";
    /// 迁移临时库（同目录＝同卷，便于原子 rename 顶替 heng.db）。
    const ENC_TMP: &str = "heng.db.enc.tmp";
    const PLAIN_TMP: &str = "heng.db.plain.tmp";
    /// 明文 SQLite 文件头（前 16 字节）。SQLCipher 密文库此处是随机密文，据此区分明/密。
    const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

    // 关注的 NCrypt HRESULT（数值比较，避免 import 不确定性）。
    const NTE_BAD_DATA: u32 = 0x8009_0005; // OAEP 解包失败（口令对但密文坏）→ Corrupt
    const NTE_PERM: u32 = 0x8009_0010; // 使用授权失败（错口令）→ WrongPassword
    const NTE_NOT_FOUND: u32 = 0x8009_0011; // 钥匙不存在 → 幂等删除 / ChipUnavailable
    const NTE_BAD_KEYSET: u32 = 0x8009_0016; // keyset 缺失 → ChipUnavailable
    // TPM DA 防爆破锁定（口令连错过多，芯片临时拒绝使用钥匙）→ Locked（不计销毁；等冷却/正确口令复位）。
    const TPM_20_E_LOCKOUT: u32 = 0x8028_0921; // TPM 2.0 RC_LOCKOUT
    const TPM_E_DEFEND_LOCK_RUNNING: u32 = 0x8028_0210; // TPM 1.2 防御锁运行中

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Slot {
        A,
        B,
    }
    fn key_const(s: Slot) -> PCWSTR {
        match s {
            Slot::A => KEY_A,
            Slot::B => KEY_B,
        }
    }
    fn slot_str(s: Slot) -> &'static str {
        match s {
            Slot::A => "a",
            Slot::B => "b",
        }
    }
    fn slot_from(s: &str) -> Option<Slot> {
        match s {
            "a" => Some(Slot::A),
            "b" => Some(Slot::B),
            _ => None,
        }
    }
    fn other_slot(s: Slot) -> Slot {
        match s {
            Slot::A => Slot::B,
            Slot::B => Slot::A,
        }
    }

    // ---- 错误构造 ----
    fn classify(code: u32) -> FailClass {
        match code {
            NTE_PERM => FailClass::WrongPassword,
            TPM_20_E_LOCKOUT | TPM_E_DEFEND_LOCK_RUNNING => FailClass::Locked, // 连错触发芯片防爆破锁定
            NTE_BAD_DATA => FailClass::Corrupt,
            // 钥匙缺失：可能是芯片暂时态，也可能是 Clear TPM/换主板/封装文件拷到他机 = 钥匙永久没了（库不可解）。
            // 本期粗归 ChipUnavailable；Phase 3/4 UI 用保留的原始 code 区分「暂不可用·重启」与「永久销毁·终态屏」
            // （§5「数据已按安全设置销毁」终态须与「芯片暂不可用」彻底分开，否则用户对着乱码徒劳重启）。
            NTE_BAD_KEYSET | NTE_NOT_FOUND => FailClass::ChipUnavailable,
            _ => FailClass::ChipUnavailable,
        }
    }
    fn from_win(e: windows::core::Error) -> CryptoError {
        let code = e.code().0 as u32;
        CryptoError {
            class: classify(code),
            code,
            message: e.message().to_string(),
        }
    }
    fn corrupt(m: &str) -> CryptoError {
        CryptoError {
            class: FailClass::Corrupt,
            code: 0,
            message: m.into(),
        }
    }
    fn internal_io(e: std::io::Error) -> CryptoError {
        CryptoError {
            class: FailClass::Internal,
            code: 0,
            message: e.to_string(),
        }
    }
    fn internal(m: String) -> CryptoError {
        CryptoError {
            class: FailClass::Internal,
            code: 0,
            message: m,
        }
    }

    // ---- 信封 ----
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct WrapEnvelope {
        version: u32,
        scheme: String,
        alg: String,
        slot: String,
        created_unix: i64,
        wrapped_dek_hex: String,
    }
    impl WrapEnvelope {
        fn new(slot: Slot, ct: &[u8]) -> Self {
            WrapEnvelope {
                version: 1,
                scheme: "tpm-pcp".into(),
                alg: "rsa2048-oaep-sha256".into(),
                slot: slot_str(slot).into(),
                created_unix: now_unix(),
                wrapped_dek_hex: encode_hex(ct),
            }
        }
    }

    fn now_unix() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
    fn encode_hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }
    fn decode_hex(s: &str) -> Option<Vec<u8>> {
        if s.len() % 2 != 0 {
            return None;
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
            .collect()
    }

    fn read_envelope(path: &Path) -> Result<WrapEnvelope, CryptoError> {
        let data = std::fs::read(path).map_err(|e| corrupt(&format!("read envelope: {e}")))?;
        serde_json::from_slice(&data).map_err(|e| corrupt(&format!("parse envelope: {e}")))
    }
    /// 写信封：create + write + sync_all（FlushFileBuffers），不 rename（由调用方原子顶替）。
    fn write_sync(path: &Path, env: &WrapEnvelope) -> Result<(), CryptoError> {
        let data = serde_json::to_vec_pretty(env).map_err(|e| internal(format!("serialize: {e}")))?;
        let mut f = File::create(path).map_err(internal_io)?;
        f.write_all(&data).map_err(internal_io)?;
        f.sync_all().map_err(internal_io)?;
        Ok(())
    }

    /// 原子顶替信封：写 `.tmp` + fsync → rename 到 ENVELOPE（同卷原子，Spike #4 已验）。
    /// rename 用重试版（与迁移/改密一致，吃掉 Windows 下 AV/索引器对刚写文件的短暂持锁；
    /// 也缩小 set_password「迁移已提交但信封 commit 失败」的窗口，review enc-4）。
    fn write_envelope_atomic(dir: &Path, env: &WrapEnvelope) -> Result<(), CryptoError> {
        let tmp = dir.join(format!("{ENVELOPE}.tmp"));
        write_sync(&tmp, env)?;
        rename_with_retry(&tmp, &dir.join(ENVELOPE))
    }

    // ---- 明↔密库迁移标记（§9 启动自愈的依据） ----
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct MigrateMarker {
        /// "encrypt"（明→密）或 "decrypt"（密→明）。
        direction: String,
        /// 该次迁移对应的信封（前滚提交 / 回滚恢复都用它，自愈无需口令）。
        envelope: WrapEnvelope,
    }
    fn write_marker(dir: &Path, m: &MigrateMarker) -> Result<(), CryptoError> {
        let data = serde_json::to_vec_pretty(m).map_err(|e| internal(format!("serialize marker: {e}")))?;
        let mut f = File::create(dir.join(MIGRATE_MARKER)).map_err(internal_io)?;
        f.write_all(&data).map_err(internal_io)?;
        f.sync_all().map_err(internal_io)?;
        Ok(())
    }
    fn read_marker(path: &Path) -> Result<MigrateMarker, CryptoError> {
        let data = std::fs::read(path).map_err(internal_io)?;
        serde_json::from_slice(&data).map_err(|e| internal(format!("parse marker: {e}")))
    }

    // ---- 预解锁安全状态 heng.security（销毁计数/开关 4b 用；上次备份 4a 用）----
    #[derive(Serialize, Deserialize, Debug, Clone, Default)]
    struct SecurityFile {
        #[serde(default)]
        version: u32,
        /// 绑当前信封哈希：不符＝信封换过（改密/重设）或被篡改 → 销毁相关字段视为新、归零。
        #[serde(default)]
        envelope_sha256: Option<String>,
        #[serde(default)]
        fail_count: u32,
        #[serde(default)]
        destroy_enabled: bool,
        #[serde(default)]
        last_backup_unix: Option<i64>,
        #[serde(default)]
        last_backup_path: Option<String>,
        #[serde(default)]
        last_backup_sha256: Option<String>,
        #[serde(default)]
        last_backup_rows: Option<i64>,
    }

    fn sha256_hex(data: &[u8]) -> String {
        Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
    }
    fn envelope_sha256(dir: &Path) -> Option<String> {
        std::fs::read(dir.join(ENVELOPE)).ok().map(|b| sha256_hex(&b))
    }
    fn sha256_file(path: &Path) -> Result<String, CryptoError> {
        Ok(sha256_hex(&std::fs::read(path).map_err(internal_io)?))
    }

    /// 读 heng.security，带安全默认：缺失/解析失败/信封哈希不符 → 销毁相关字段归零（删/篡改只能饶过、绝不导致销毁）。
    /// 备份字段（last_backup_*）与信封无关，保留。
    fn read_security(dir: &Path) -> SecurityFile {
        let f: SecurityFile = std::fs::read(dir.join(SECURITY_FILE))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let cur = envelope_sha256(dir);
        if cur.is_none() || f.envelope_sha256 != cur {
            SecurityFile { fail_count: 0, destroy_enabled: false, envelope_sha256: cur, ..f }
        } else {
            f
        }
    }

    /// 原子写 heng.security（写时盖上当前信封哈希，保持一致）。
    fn write_security(dir: &Path, f: &SecurityFile) -> Result<(), CryptoError> {
        let mut out = f.clone();
        out.version = 1;
        out.envelope_sha256 = envelope_sha256(dir);
        let data = serde_json::to_vec_pretty(&out).map_err(|e| internal(format!("serialize security: {e}")))?;
        let tmp = dir.join(format!("{SECURITY_FILE}.tmp"));
        let mut fh = File::create(&tmp).map_err(internal_io)?;
        fh.write_all(&data).map_err(internal_io)?;
        fh.sync_all().map_err(internal_io)?;
        rename_with_retry(&tmp, &dir.join(SECURITY_FILE))
    }

    // ---- 备份导出（明文，§7）----

    /// 拒绝把备份导到应用数据目录里的 `heng.*`（活动库/信封/控制文件），防覆盖自毁。
    /// dest 可能尚不存在 → canonicalize 其父目录后比对。
    fn validate_backup_dest(dir: &Path, dest: &Path) -> Result<(), CryptoError> {
        let parent = dest.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(dir);
        let canon_parent = parent
            .canonicalize()
            .map_err(|e| internal(format!("无效的保存路径: {e}")))?;
        // config_dir 的 canonicalize 失败 → 硬报错（绝不静默放行，否则可绕过 heng.* 防撞写穿活动库）。
        // config_dir 在更早已 create_dir_all，正常必成功。
        let canon_dir = dir
            .canonicalize()
            .map_err(|e| internal(format!("应用数据目录不可访问: {e}")))?;
        if canon_parent == canon_dir {
            if let Some(name) = dest.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("heng.") {
                    return Err(internal("不能把备份导出到应用数据目录的 heng.* 文件（会覆盖活动数据）".into()));
                }
            }
        }
        Ok(())
    }

    /// 统计某库（schema=main/bak）所有用户表的总行数（备份新鲜度/4b 销毁前完整性校验用）。
    fn count_user_rows(conn: &Connection, schema: &str) -> Result<i64, CryptoError> {
        let tables: Vec<String> = {
            let q = format!("SELECT name FROM {schema}.sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'");
            let mut st = conn.prepare(&q).map_err(|e| corrupt(&format!("list tables: {e}")))?;
            let rows = st.query_map([], |r| r.get::<_, String>(0)).map_err(|e| corrupt(&format!("list: {e}")))?;
            rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|e| corrupt(&format!("list: {e}")))?
        };
        let mut total = 0i64;
        for t in &tables {
            let n: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {schema}.\"{t}\""), [], |r| r.get(0))
                .map_err(|e| corrupt(&format!("count {t}: {e}")))?;
            total += n;
        }
        Ok(total)
    }

    /// 导出一份**明文**备份到 dest（关闭加密的等价物）。加密库须传已解锁 DEK；明文库 dek=None。
    /// 统一走 sqlcipher_export（不分明/密两套）：写 `.tmp`→export+校验→DETACH+close→无 WAL 边车→原子 rename。
    /// 成功后把 last_backup_*（含 sha256+行数，供 4b 销毁前重验）写回 heng.security。
    pub(super) fn export_backup(dir: &Path, dek: Option<&[u8; 32]>, dest_str: &str) -> Result<BackupInfo, CryptoError> {
        reconcile(dir);
        let db = dir.join(DB_FILE);
        let encrypted = dir.join(ENVELOPE).exists();
        if encrypted && dek.is_none() {
            return Err(internal("数据库已加密但尚未解锁，无法导出备份".into()));
        }
        let dest = Path::new(dest_str);
        validate_backup_dest(dir, dest)?;
        // 临时文件放 dest 同目录（同卷 → 原子 rename）。
        let tmp = {
            let fname = format!(
                "{}.heng-backup.tmp",
                dest.file_name().and_then(|n| n.to_str()).unwrap_or("backup")
            );
            match dest.parent().filter(|p| !p.as_os_str().is_empty()) {
                Some(d) => d.join(fname),
                None => std::path::PathBuf::from(fname),
            }
        };
        let _ = std::fs::remove_file(&tmp);
        clean_sidecars(&tmp);

        let rows = {
            let src = Connection::open(&db).map_err(|e| corrupt(&format!("open db: {e}")))?;
            if let Some(d) = dek {
                let key = Zeroizing::new(format!("PRAGMA key = \"x'{}'\";", &*super::dek_hex(d)));
                src.execute_batch(key.as_str()).map_err(|e| corrupt(&format!("pragma key: {e}")))?;
            }
            let _ = src.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
            src.execute("ATTACH DATABASE ?1 AS bak KEY ''", [tmp.to_string_lossy().as_ref()])
                .map_err(|e| corrupt(&format!("attach bak: {e}")))?;
            export_into(&src, "bak")?;
            verify_attached(&src, "bak")?;
            let rows = count_user_rows(&src, "main")?;
            src.execute_batch("DETACH DATABASE bak;").map_err(|e| corrupt(&format!("detach: {e}")))?;
            src.close().map_err(|(_, e)| corrupt(&format!("close: {e}")))?;
            rows
        };
        clean_sidecars(&tmp); // 备份是单文件，清掉任何残留边车
        rename_with_retry(&tmp, dest)?;

        let unix = now_unix();
        let mut sec = read_security(dir);
        sec.last_backup_unix = Some(unix);
        sec.last_backup_path = Some(dest.to_string_lossy().into_owned());
        sec.last_backup_sha256 = Some(sha256_file(dest)?);
        sec.last_backup_rows = Some(rows);
        write_security(dir, &sec)?;
        Ok(BackupInfo { path: dest.to_string_lossy().into_owned(), unix, rows })
    }

    // ---- 销毁（错 N 次，§5）----

    /// 销毁进行中标记（携隔离目录路径；heal 复用、不重算 now()，避免两次销毁/重入碰撞）。
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct DestroyMarker {
        quarantine_dir: String,
    }

    /// 校验记录在案的备份**当前仍在且完整**（文件存在 + sha256 与记录一致）。强闸门：开启销毁前、
    /// 真正销毁前都要过这关；过不了则**中止销毁**、绝不在无有效备份时毁数据。
    fn verify_backup(sec: &SecurityFile) -> Result<(), CryptoError> {
        let (Some(path), Some(sha)) = (&sec.last_backup_path, &sec.last_backup_sha256) else {
            return Err(internal("没有可用备份记录".into()));
        };
        let p = Path::new(path);
        if !p.exists() {
            return Err(internal(format!("备份文件已不在: {path}")));
        }
        if &sha256_file(p)? != sha {
            return Err(internal(format!("备份文件已损坏或被改动: {path}")));
        }
        Ok(())
    }

    /// 解锁成功 → 失败计数归零（绑当前信封哈希一并写）。
    pub(super) fn on_unlock_success(dir: &Path) {
        let mut sec = read_security(dir);
        if sec.fail_count != 0 {
            sec.fail_count = 0;
            let _ = write_security(dir, &sec);
        }
    }

    /// 错口令（**仅** WrongPassword 调用）→ 计数 +1（原子 fsync）。达阈值且开了销毁且备份验得过 → 销毁、返回 true。
    /// 备份验不过 → **不销毁**（数据保留加密态、留待恢复备份），返回 false。其它情况返回 false。
    pub(super) fn on_wrong_password(dir: &Path) -> Result<bool, CryptoError> {
        let mut sec = read_security(dir);
        sec.fail_count = sec.fail_count.saturating_add(1);
        write_security(dir, &sec)?;
        if sec.destroy_enabled && sec.fail_count >= DESTROY_THRESHOLD {
            // destroy() 内含迁移闸 + 备份重验（紧贴销毁、闭合 TOCTOU）：任一不过则提交点前中止、不毁数据。
            let _ = destroy(dir);
            // 一旦销毁已提交（标记/sentinel 在）即视为 destroyed（前滚到底、门会自愈补完）；
            // 提交前中止（迁移在途/备份验不过 → 标记没写）则 false、数据原样保留。
            return Ok(is_destroyed(dir));
        }
        Ok(false)
    }

    /// 是否处于销毁态。**sentinel 或 destroying 标记任一在即算 destroyed**（review must-fix #1/#3）：
    /// 标记一旦写下＝销毁已提交（前滚单向）；即便后续写 sentinel 因崩溃/磁盘满失败，门也据标记走终态屏、
    /// 绝不落到 plaintext 分支静默造空库。标记只在销毁完整收尾(slot 删净)后删除、那时 sentinel 必已在。
    pub(super) fn is_destroyed(dir: &Path) -> bool {
        dir.join(DESTROYED_SENTINEL).exists() || dir.join(DESTROYING_MARKER).exists()
    }

    /// 开/关销毁。开启要求：本次解锁会话内成功备份过（session_backed_up）+ 记录的备份仍在且完整。
    pub(super) fn set_destroy_enabled(dir: &Path, enabled: bool, session_backed_up: bool) -> Result<(), CryptoError> {
        let mut sec = read_security(dir);
        if enabled {
            if !dir.join(ENVELOPE).exists() {
                return Err(internal("未加密，无法开启销毁".into()));
            }
            if !session_backed_up {
                return Err(internal("开启销毁前，请先在本次解锁会话内导出一份备份".into()));
            }
            verify_backup(&sec)?; // 备份必须仍在且完整
        }
        sec.destroy_enabled = enabled;
        write_security(dir, &sec)?;
        Ok(())
    }

    /// 销毁（错 N 次触发）。**前滚单向**（一旦触发即完成，绝不回滚——数据本就该没）。
    /// 顺序（崩溃安全，见 reconcile 自愈）：写标记(含隔离路径) → 隔离密文库+清边车+扫残留 tmp →
    /// 删信封 → **写 sentinel（先于删钥匙！）** → 删两 slot（线程化 bool：没删净留标记 heal）→ 删标记。
    /// 调用前提：调用方已 reconcile（无迁移/改密标记在途）。
    fn destroy(dir: &Path) -> Result<(), CryptoError> {
        // 双保险（must-fix #1）：销毁绝不与迁移/改密中途并发。正常路径 engine::unlock 已先 reconcile 清掉它们；
        // 若仍在途则**中止销毁**（数据保留，下次重试），绝不在半迁移态毁数据。
        if dir.join(MIGRATE_MARKER).exists() || dir.join(ENVELOPE_NEW).exists() {
            return Err(internal("迁移/改密进行中，暂不销毁".into()));
        }
        // **最后一道备份闸**（review BACKUP-VERIFY-GATE / #2）：紧贴销毁提交点重验，闭合「on_wrong_password 验过
        // 到这里」之间的 TOCTOU——备份不在/损坏则**中止销毁、绝不无恢复路径毁数据**。这是销毁的提交闸：
        // 一旦下面 run_destroy 写标记即视为已提交、前滚到底（heal 不再重验，避免半态死局）。
        verify_backup(&read_security(dir))?;
        let qdir = dir.join(format!("{QUARANTINE_PREFIX}{}", now_unix()));
        run_destroy(dir, &qdir, true)
    }

    /// 销毁的可重入实现（首发 fresh=true 写标记；reconcile heal 时 fresh=false 复用既有标记里的隔离路径）。
    fn run_destroy(dir: &Path, qdir: &Path, fresh: bool) -> Result<(), CryptoError> {
        if fresh {
            write_destroy_marker(dir, qdir)?;
        }
        let _ = std::fs::create_dir_all(qdir);
        // **sentinel 先于一切数据破坏写**（review must-fix #1）：移库/删信封前先把终态真相落盘，
        // 杜绝"库已隔离/信封已删但无 sentinel→门走 plaintext 造空库"的窗口。（标记已覆盖此窗口，sentinel-first 再加一层。）
        write_destroyed_sentinel(dir)?;
        // 隔离密文库（移而不删——非静默 unlink；密钥删后仍不可解，仅作墓碑）。库可能已被前次 heal 移走 → 容缺失。
        let db = dir.join(DB_FILE);
        if db.exists() {
            let _ = rename_with_retry(&db, &qdir.join(DB_FILE));
        }
        clean_sidecars(&db);
        // 扫掉 config_dir 里任何残留的明文/迁移 tmp（避免销毁后留明文窗口）。
        for tmp in ["heng.db.enc.tmp", "heng.db.plain.tmp"] {
            let _ = std::fs::remove_file(dir.join(tmp));
        }
        let _ = std::fs::remove_file(dir.join(ENVELOPE)); // 删信封
        write_quarantine_readme(qdir);
        let slots_gone = delete_both_slots(None); // 删两 slot（不可逆）
        // 计数清零（数据已没，计数无意义；信封已删 → read_security 本就会归零，这里显式写一次）。
        let mut sec = read_security(dir);
        sec.fail_count = 0;
        sec.destroy_enabled = false;
        let _ = write_security(dir, &sec);
        if slots_gone {
            let _ = std::fs::remove_file(dir.join(DESTROYING_MARKER));
        }
        // slot 没删净 → 留标记作 heal 锚点，下次 reconcile 重试删钥匙。
        Ok(())
    }

    fn write_destroy_marker(dir: &Path, qdir: &Path) -> Result<(), CryptoError> {
        let m = DestroyMarker { quarantine_dir: qdir.to_string_lossy().into_owned() };
        let data = serde_json::to_vec_pretty(&m).map_err(|e| internal(format!("serialize destroy marker: {e}")))?;
        let tmp = dir.join(format!("{DESTROYING_MARKER}.tmp"));
        let mut f = File::create(&tmp).map_err(internal_io)?;
        f.write_all(&data).map_err(internal_io)?;
        f.sync_all().map_err(internal_io)?;
        rename_with_retry(&tmp, &dir.join(DESTROYING_MARKER))
    }
    fn write_destroyed_sentinel(dir: &Path) -> Result<(), CryptoError> {
        let data = serde_json::to_vec_pretty(&serde_json::json!({ "destroyed_unix": now_unix() }))
            .map_err(|e| internal(format!("serialize sentinel: {e}")))?;
        let tmp = dir.join(format!("{DESTROYED_SENTINEL}.tmp"));
        let mut f = File::create(&tmp).map_err(internal_io)?;
        f.write_all(&data).map_err(internal_io)?;
        f.sync_all().map_err(internal_io)?;
        rename_with_retry(&tmp, &dir.join(DESTROYED_SENTINEL))
    }
    fn write_quarantine_readme(qdir: &Path) {
        let note = "此目录是衡记「错密码达上限自动销毁」留下的密文墓碑。\r\n\
            数据已按你的安全设置永久销毁：封装密钥已从安全芯片删除，这份密文**无法再解开**。\r\n\
            可恢复的数据只在你此前导出的未加密备份里。确认无用后可整目录删除。\r\n";
        let _ = std::fs::write(qdir.join("README.txt"), note);
    }

    /// 销毁自愈（在 reconcile **最前面**调用）：标记在 → 前滚补完销毁、返回 true（调用方据此短路、忽略迁移/改密标记）。
    fn heal_destroy(dir: &Path) -> bool {
        let mp = dir.join(DESTROYING_MARKER);
        if !mp.exists() {
            return false;
        }
        let qdir = read_marker_json::<DestroyMarker>(&mp)
            .map(|m| std::path::PathBuf::from(m.quarantine_dir))
            // 标记损坏 → 退一个新隔离目录名（极罕见；至少把销毁补完不留半态）。
            .unwrap_or_else(|| dir.join(format!("{QUARANTINE_PREFIX}heal-{}", now_unix())));
        let _ = run_destroy(dir, &qdir, false);
        true
    }

    fn read_marker_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
        std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok())
    }

    /// 销毁后「从空白重新开始」：清 sentinel + 标记 + 隔离区 + heng.security，下次启动即建全新空明文库。
    pub(super) fn restart_after_destroy(dir: &Path) -> Result<(), CryptoError> {
        // **sentinel + 标记先删且必须确认删掉**（review DESTROY-005）：删不掉就别动其它（隔离区保留可重试），
        // 报错让 UI 重试——否则它俩任一残留下次启动仍判 destroyed、用户卡死终态屏（is_destroyed 看二者）。
        let _ = std::fs::remove_file(dir.join(DESTROYED_SENTINEL));
        let _ = std::fs::remove_file(dir.join(DESTROYING_MARKER));
        if is_destroyed(dir) {
            return Err(internal("清除销毁标记失败（文件被占用？），请稍后重试".into()));
        }
        // 兜底清干净（即便 run_destroy 某步删失败，这里也确保下次是全新空库）：信封 + 各标记 + 状态文件。
        for f in [ENVELOPE, ENVELOPE_NEW, MIGRATE_MARKER, SECURITY_FILE] {
            let _ = std::fs::remove_file(dir.join(f));
        }
        // 清掉所有隔离墓碑目录。
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().starts_with(QUARANTINE_PREFIX) {
                    let _ = std::fs::remove_dir_all(e.path());
                }
            }
        }
        // 残留库一并清（销毁已把它隔离走；若有残留也清掉，确保下次是全新空库）。
        let db = dir.join(DB_FILE);
        let _ = std::fs::remove_file(&db);
        clean_sidecars(&db);
        Ok(())
    }

    /// 库文件头是否为明文 SQLite。文件缺失／太短 → 视为「非明文」（保守：不会把损坏库当明文去开）。
    pub(super) fn db_is_plaintext(path: &Path) -> bool {
        match File::open(path) {
            Ok(mut f) => {
                let mut buf = [0u8; 16];
                f.read_exact(&mut buf).is_ok() && &buf == SQLITE_HEADER
            }
            Err(_) => false,
        }
    }

    /// 同卷原子 rename 顶替。Windows 下刚关闭连接的库文件可能被 AV/索引器/close-pending 短暂持锁，
    /// rename 报 ACCESS_DENIED(os error 5)——Spike #2 probe2 定为**可重试**：小退避重试几次再放弃。
    fn rename_with_retry(from: &Path, to: &Path) -> Result<(), CryptoError> {
        let mut last: Option<std::io::Error> = None;
        for i in 0..12u32 {
            match std::fs::rename(from, to) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last = Some(e);
                    std::thread::sleep(std::time::Duration::from_millis(40 * u64::from(i + 1)));
                }
            }
        }
        Err(internal_io(last.expect("loop runs ≥1 time")))
    }

    /// 删库的 WAL/SHM 边车（迁移后残留旧边车会损坏新库，Spike #2 probe2）。
    fn clean_sidecars(db: &Path) {
        for ext in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ext}", db.display()));
        }
    }

    /// 校验迁移目标库：能用预期 key 打开 + integrity_check=ok + 每张用户表行数与源库一致。
    /// 在「源连接仍 ATTACH 着目标(别名 alias)」时调用，趁手对比两库行数（§9「逐行一致」的低成本等价：
    /// integrity_check 抓页级损坏、行数比对抓 export 漏表/截断；全行 hash 比对留 Phase 4 硬化）。
    fn verify_attached(src: &Connection, alias: &str) -> Result<(), CryptoError> {
        let ok: String = src
            .query_row(&format!("PRAGMA {alias}.integrity_check"), [], |r| r.get(0))
            .map_err(|e| corrupt(&format!("integrity_check: {e}")))?;
        if ok != "ok" {
            return Err(corrupt(&format!("目标库完整性校验失败: {ok}")));
        }
        let tables: Vec<String> = {
            let mut st = src
                .prepare("SELECT name FROM main.sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
                .map_err(|e| corrupt(&format!("list tables: {e}")))?;
            let rows = st
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| corrupt(&format!("list tables: {e}")))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| corrupt(&format!("list tables: {e}")))?
        };
        for t in &tables {
            // 表名取自本库 sqlite_master（自有 schema，固定安全标识符）；双引号防御。
            let q = format!("SELECT (SELECT count(*) FROM main.\"{t}\"), (SELECT count(*) FROM {alias}.\"{t}\")");
            let (a, b): (i64, i64) = src
                .query_row(&q, [], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(|e| corrupt(&format!("count {t}: {e}")))?;
            if a != b {
                return Err(corrupt(&format!("表 {t} 行数不一致: 源 {a} ≠ 目标 {b}")));
            }
        }
        Ok(())
    }

    /// 用 sqlcipher_export 把 `src`（已开好的连接）整库导出到 ATTACH 的 `alias`，并补传 user_version
    /// （sqlcipher_export **不**复制 PRAGMA user_version——丢了会让下次启动重跑全部迁移）。
    fn export_into(src: &Connection, alias: &str) -> Result<(), CryptoError> {
        let uv: i64 = src
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| corrupt(&format!("read user_version: {e}")))?;
        // sqlcipher_export 是带副作用的 SQL 函数；用 query 并排空结果集（不假设返回行数）。
        {
            let mut st = src
                .prepare(&format!("SELECT sqlcipher_export('{alias}')"))
                .map_err(|e| corrupt(&format!("sqlcipher_export prepare: {e}")))?;
            let mut rows = st.query([]).map_err(|e| corrupt(&format!("sqlcipher_export: {e}")))?;
            while rows.next().map_err(|e| corrupt(&format!("sqlcipher_export: {e}")))?.is_some() {}
        }
        src.execute_batch(&format!("PRAGMA {alias}.user_version = {uv};"))
            .map_err(|e| corrupt(&format!("set user_version: {e}")))?;
        Ok(())
    }

    /// 明文 heng.db → 密文（用 dek）。调用方须已关闭 State 连接（文件不被占用）。
    /// 协议：清残留 tmp → 开明文源 → checkpoint 折叠 WAL → ATTACH 密文 tmp(raw key) → export+校验 →
    /// DETACH+关闭 → fsync tmp → **原子 rename tmp→heng.db（提交点）** → 清明文边车。失败前回滚（删 tmp）。
    fn migrate_encrypt(dir: &Path, dek: &[u8; 32]) -> Result<(), CryptoError> {
        let db = dir.join(DB_FILE);
        let tmp = dir.join(ENC_TMP);
        let _ = std::fs::remove_file(&tmp);
        clean_sidecars(&tmp);
        // DEK hex 与内嵌它的 SQL 全程 Zeroizing（用后清零，不在堆上留明文密钥残渣）。
        // raw-key 字符串字面量；dek_hex 恒为 64 位十六进制（自产），无注入。tmp 路径用绑定参数。
        // 注：错误只回传内层 rusqlite error（不含语句文本，SQLite 设计如此），DEK 不入 CryptoError.message。
        let dek_hex = super::dek_hex(dek);
        let attach = Zeroizing::new(format!("ATTACH DATABASE ?1 AS enc KEY \"x'{}'\"", &*dek_hex));
        let run = || -> Result<(), CryptoError> {
            let src = Connection::open(&db).map_err(|e| corrupt(&format!("open plaintext db: {e}")))?;
            src.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|e| corrupt(&format!("checkpoint: {e}")))?;
            src.execute(attach.as_str(), [tmp.to_string_lossy().as_ref()])
                .map_err(|e| corrupt(&format!("attach enc: {e}")))?;
            export_into(&src, "enc")?;
            verify_attached(&src, "enc")?;
            src.execute_batch("DETACH DATABASE enc;")
                .map_err(|e| corrupt(&format!("detach: {e}")))?;
            // 显式 close（同步、抛 BUSY）而非 drop——drop 在 Windows 上可能延迟释放文件句柄，
            // 导致随后 rename 顶替 heng.db 报 ACCESS_DENIED。
            // tmp 内容已由 SQLite 在 sqlcipher_export 提交时 fsync（synchronous=FULL，rollback-journal）；
            // 不再 reopen-fsync（Windows 下刚写完的 .db 常被 Defender/索引器短暂独占，reopen 会 ACCESS_DENIED）。
            src.close().map_err(|(_, e)| corrupt(&format!("close src: {e}")))?;
            rename_with_retry(&tmp, &db)?; // 提交点（retry 吃掉 AV/索引器对 tmp/db 的短暂持锁）
            clean_sidecars(&db);
            Ok(())
        };
        let r = run();
        if r.is_err() {
            let _ = std::fs::remove_file(&tmp);
            clean_sidecars(&tmp);
        }
        r
    }

    /// 密文 heng.db（用 dek）→ 明文。调用方须已关闭 State 连接。协议同上、方向反转（ATTACH 目标 KEY ''＝明文）。
    fn migrate_decrypt(dir: &Path, dek: &[u8; 32]) -> Result<(), CryptoError> {
        let db = dir.join(DB_FILE);
        let tmp = dir.join(PLAIN_TMP);
        let _ = std::fs::remove_file(&tmp);
        clean_sidecars(&tmp);
        let dek_hex = super::dek_hex(dek); // Zeroizing
        let key_pragma = Zeroizing::new(format!("PRAGMA key = \"x'{}'\";", &*dek_hex));
        let run = || -> Result<(), CryptoError> {
            let src = Connection::open(&db).map_err(|e| corrupt(&format!("open cipher db: {e}")))?;
            // raw-key 必为开库首条 PRAGMA。
            src.execute_batch(key_pragma.as_str())
                .map_err(|e| corrupt(&format!("pragma key: {e}")))?;
            src.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|e| corrupt(&format!("checkpoint: {e}")))?;
            src.execute("ATTACH DATABASE ?1 AS plain KEY ''", [tmp.to_string_lossy().as_ref()])
                .map_err(|e| corrupt(&format!("attach plain: {e}")))?;
            export_into(&src, "plain")?;
            verify_attached(&src, "plain")?;
            src.execute_batch("DETACH DATABASE plain;")
                .map_err(|e| corrupt(&format!("detach: {e}")))?;
            src.close().map_err(|(_, e)| corrupt(&format!("close src: {e}")))?;
            rename_with_retry(&tmp, &db)?; // 提交点（tmp 已由 SQLite 提交时 fsync；retry 吃掉短暂持锁）
            clean_sidecars(&db);
            Ok(())
        };
        let r = run();
        if r.is_err() {
            let _ = std::fs::remove_file(&tmp);
            clean_sidecars(&tmp);
        }
        r
    }

    // ---- 口令 → PCP 使用授权摘要（UTF-16LE → SHA-256，与 Spike #2 一致） ----
    fn usage_auth_digest(pw: &str) -> Zeroizing<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::with_capacity(pw.len() * 2);
        for u in pw.encode_utf16() {
            buf.extend_from_slice(&u.to_le_bytes());
        }
        let out = Zeroizing::new(Sha256::digest(&buf).to_vec());
        buf.zeroize();
        out
    }

    // ---- NCrypt 句柄 RAII（杜绝错误路径泄漏句柄） ----
    struct Provider(NCRYPT_PROV_HANDLE);
    impl Drop for Provider {
        fn drop(&mut self) {
            unsafe {
                let _ = NCryptFreeObject(NCRYPT_HANDLE(self.0 .0));
            }
        }
    }
    struct KeyHandle(NCRYPT_KEY_HANDLE);
    impl Drop for KeyHandle {
        fn drop(&mut self) {
            unsafe {
                let _ = NCryptFreeObject(NCRYPT_HANDLE(self.0 .0));
            }
        }
    }

    fn open_provider() -> Result<Provider, CryptoError> {
        let mut p = NCRYPT_PROV_HANDLE::default();
        unsafe { NCryptOpenStorageProvider(&mut p, PROVIDER, 0) }.map_err(from_win)?;
        Ok(Provider(p))
    }

    /// 建 per-user（无 machine-key flag ⇒ 非提权）非导出 RSA-2048 钥匙，口令作 PCP_USAGEAUTH。
    /// OVERWRITE：清掉该 slot 任何残留孤儿钥匙。
    fn create_slot_key(prov: &Provider, slot: Slot, pw: &str) -> Result<KeyHandle, CryptoError> {
        unsafe {
            let mut raw = NCRYPT_KEY_HANDLE::default();
            NCryptCreatePersistedKey(
                prov.0,
                &mut raw,
                ALG_RSA,
                key_const(slot),
                CERT_KEY_SPEC(0),
                NCRYPT_OVERWRITE_KEY_FLAG,
            )
            .map_err(from_win)?;
            let key = KeyHandle(raw);
            let len: u32 = 2048;
            NCryptSetProperty(NCRYPT_HANDLE(key.0 .0), PROP_LENGTH, &len.to_le_bytes(), NCRYPT_FLAGS(0))
                .map_err(from_win)?;
            let exp: u32 = 0; // 非导出（TPM 本就非导出，双保险）
            NCryptSetProperty(NCRYPT_HANDLE(key.0 .0), PROP_EXPORT, &exp.to_le_bytes(), NCRYPT_FLAGS(0))
                .map_err(from_win)?;
            let digest = usage_auth_digest(pw);
            NCryptSetProperty(NCRYPT_HANDLE(key.0 .0), PROP_USAGEAUTH, digest.as_slice(), NCRYPT_FLAGS(0))
                .map_err(from_win)?;
            NCryptFinalizeKey(key.0, NCRYPT_FLAGS(0)).map_err(from_win)?;
            Ok(key)
        }
    }

    /// 打开已有 slot 钥匙并在 reopened handle 上设 USAGEAUTH（SILENT，不弹 UI）。
    /// 注意：设 USAGEAUTH 即便口令错也会**成功**；错口令在 unwrap 的 NCryptDecrypt 处才被芯片拒绝。
    fn open_slot_key(prov: &Provider, slot: Slot, pw: &str) -> Result<KeyHandle, CryptoError> {
        unsafe {
            let mut raw = NCRYPT_KEY_HANDLE::default();
            NCryptOpenKey(prov.0, &mut raw, key_const(slot), CERT_KEY_SPEC(0), NCRYPT_SILENT_FLAG)
                .map_err(from_win)?;
            let key = KeyHandle(raw);
            let digest = usage_auth_digest(pw);
            NCryptSetProperty(NCRYPT_HANDLE(key.0 .0), PROP_USAGEAUTH, digest.as_slice(), NCRYPT_SILENT_FLAG)
                .map_err(from_win)?;
            Ok(key)
        }
    }

    fn wrap_dek(key: &KeyHandle, dek: &[u8; 32]) -> Result<Vec<u8>, CryptoError> {
        unsafe {
            let oaep = BCRYPT_OAEP_PADDING_INFO {
                pszAlgId: ALG_SHA256,
                pbLabel: std::ptr::null_mut(),
                cbLabel: 0,
            };
            let pad = Some(&oaep as *const _ as *const c_void);
            let mut need = 0u32;
            NCryptEncrypt(key.0, Some(dek), pad, None, &mut need, NCRYPT_PAD_OAEP_FLAG).map_err(from_win)?;
            let mut ct = vec![0u8; need as usize];
            NCryptEncrypt(key.0, Some(dek), pad, Some(&mut ct), &mut need, NCRYPT_PAD_OAEP_FLAG)
                .map_err(from_win)?;
            ct.truncate(need as usize);
            Ok(ct)
        }
    }

    /// 解封 DEK。错口令 → NCryptDecrypt 返回 NTE_PERM（→ WrongPassword）；密文坏 → NTE_BAD_DATA（→ Corrupt）。
    fn unwrap_dek(key: &KeyHandle, ct: &[u8]) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
        unsafe {
            let oaep = BCRYPT_OAEP_PADDING_INFO {
                pszAlgId: ALG_SHA256,
                pbLabel: std::ptr::null_mut(),
                cbLabel: 0,
            };
            let pad = Some(&oaep as *const _ as *const c_void);
            let mut need = 0u32;
            NCryptDecrypt(key.0, Some(ct), pad, None, &mut need, NCRYPT_PAD_OAEP_FLAG).map_err(from_win)?;
            let mut pt = Zeroizing::new(vec![0u8; need as usize]);
            NCryptDecrypt(key.0, Some(ct), pad, Some(pt.as_mut_slice()), &mut need, NCRYPT_PAD_OAEP_FLAG)
                .map_err(from_win)?;
            let n = need as usize;
            if n != 32 {
                return Err(corrupt(&format!("unwrapped DEK length {n} != 32")));
            }
            let mut dek = Zeroizing::new([0u8; 32]);
            dek.copy_from_slice(&pt[..32]);
            Ok(dek)
        }
    }

    /// 删 slot 钥匙（幂等：不存在视为已删）。NCryptDeleteKey 会一并释放 handle ⇒ 不走 KeyHandle RAII（防双重释放）。
    fn delete_slot_key(prov: &Provider, slot: Slot) -> Result<(), CryptoError> {
        unsafe {
            let mut raw = NCRYPT_KEY_HANDLE::default();
            match NCryptOpenKey(prov.0, &mut raw, key_const(slot), CERT_KEY_SPEC(0), NCRYPT_SILENT_FLAG) {
                Ok(()) => {
                    // NCryptDeleteKey 成功会一并释放 handle；失败则不会 → 失败分支手动释放，避免泄漏。
                    match NCryptDeleteKey(raw, 0) {
                        Ok(()) => Ok(()),
                        Err(e) => {
                            let _ = NCryptFreeObject(NCRYPT_HANDLE(raw.0));
                            Err(from_win(e))
                        }
                    }
                }
                Err(e) => {
                    let code = e.code().0 as u32;
                    if code == NTE_NOT_FOUND || code == NTE_BAD_KEYSET {
                        Ok(())
                    } else {
                        Err(from_win(e))
                    }
                }
            }
        }
    }

    fn gen_dek() -> Result<Zeroizing<[u8; 32]>, CryptoError> {
        let mut dek = Zeroizing::new([0u8; 32]);
        unsafe { BCryptGenRandom(None, &mut dek[..], BCRYPT_USE_SYSTEM_PREFERRED_RNG) }
            .ok()
            .map_err(from_win)?;
        Ok(dek)
    }

    /// 删除移除密码涉及的 slot 钥匙。`slot=Some(s)`：删 s + other(s)；`slot=None`（标记损坏、方向未知）：删 A、B 两个。
    /// 返回 true＝全部成功（含「本就不存在」幂等成功）。芯片不可用或删失败 → false，调用方据此**保留迁移标记**
    /// 作锚点，下次 reconcile 重试 —— 杜绝「信封已删但 slot 残留、又无锚点可清」的孤儿 slot（review enc-2/3/5）。
    fn delete_both_slots(slot: Option<Slot>) -> bool {
        let Ok(prov) = open_provider() else { return false };
        let (s1, s2) = match slot {
            Some(s) => (s, other_slot(s)),
            None => (Slot::A, Slot::B),
        };
        // 两个都尝试（非短路 &），即便第一个失败也删第二个。
        delete_slot_key(&prov, s1).is_ok() & delete_slot_key(&prov, s2).is_ok()
    }

    /// 启动/操作前自愈（DA-free，绝不靠猜口令）。三段，按依赖序：
    /// 1. **迁移标记 `heng.migrate`**（明↔密迁移中断）：据库文件头判定在原子 rename(提交点)哪一侧——
    ///    - encrypt：库仍明文 ⇒ rename 未发生 → **回滚**（删未提交信封/tmp/staging slot，回到明文）；
    ///      库已密文 ⇒ rename 已成 → **前滚**（提交信封，进入加密态）。
    ///    - decrypt：库仍密文 ⇒ **回滚**（恢复信封、删 plain tmp，留在加密态）；
    ///      库已明文 ⇒ **前滚**（删信封+两 slot，完成解密）。
    /// 2. **`heng.dek.tpm.new`**（改密在 commit 前中断）：删 staging slot + .new（用旧口令仍可解）。
    /// 3. 顺带清掉 live 信封的非 live slot 孤儿钥匙。
    pub(super) fn reconcile(dir: &Path) {
        // —— 0. 销毁自愈（**最前面 + 短路**）：销毁是终态、单向前滚，优先于一切迁移/改密标记 ——
        // （否则中断的销毁 + 残留迁移标记会互踩成无 sentinel 的死局，review must-fix #1）。
        if heal_destroy(dir) {
            return;
        }
        // —— 1. 迁移标记 ——
        let markerp = dir.join(MIGRATE_MARKER);
        if markerp.exists() {
            // keep_marker：清理未竟（slot 没删干净）时保留标记作下次重试锚点（标记内含 slot 信息，无需信封）。
            let mut keep_marker = false;
            match read_marker(&markerp) {
                Ok(m) => {
                    let plaintext = db_is_plaintext(&dir.join(DB_FILE));
                    let slot = slot_from(&m.envelope.slot);
                    match m.direction.as_str() {
                        "encrypt" => {
                            if plaintext {
                                // 回滚到明文：删未提交信封（及其 .tmp）+ enc tmp + staging slot。
                                let _ = std::fs::remove_file(dir.join(ENVELOPE));
                                let _ = std::fs::remove_file(dir.join(format!("{ENVELOPE}.tmp")));
                                let _ = std::fs::remove_file(dir.join(ENC_TMP));
                                if let (Some(s), Ok(prov)) = (slot, open_provider()) {
                                    let _ = delete_slot_key(&prov, s);
                                }
                            } else {
                                // 前滚：提交信封（幂等）。
                                let _ = write_envelope_atomic(dir, &m.envelope);
                            }
                        }
                        "decrypt" => {
                            if plaintext {
                                // 前滚：完成移除（删两 slot + 信封）。slot 没删净则保留标记下次重试。
                                keep_marker = !delete_both_slots(slot);
                                let _ = std::fs::remove_file(dir.join(ENVELOPE));
                            } else {
                                // 回滚到加密态：恢复信封（幂等）+ 删 plain tmp。
                                let _ = write_envelope_atomic(dir, &m.envelope);
                                let _ = std::fs::remove_file(dir.join(PLAIN_TMP));
                            }
                        }
                        _ => {}
                    }
                }
                // 标记损坏（极罕见——write 时 sync_all 后才返回；多为外部损坏，方向已不可知）：
                // 据库文件头做**方向无关**自愈——明文库 ⇒ 任何信封/slot 都已过时，删之（slot 没删净则保留标记重试）；
                // 密文库 ⇒ 一致的加密态，不动信封。enc-1：不再无脑删坏标记，避免「明文库却 status=已加密」的悬挂态。
                Err(_) => {
                    if db_is_plaintext(&dir.join(DB_FILE)) {
                        let _ = std::fs::remove_file(dir.join(ENVELOPE));
                        keep_marker = !delete_both_slots(None);
                    }
                }
            }
            if !keep_marker {
                let _ = std::fs::remove_file(&markerp);
            }
        }
        // —— 2. 改密 staging ——
        let newp = dir.join(ENVELOPE_NEW);
        if newp.exists() {
            if let Ok(env) = read_envelope(&newp) {
                if let (Some(slot), Ok(prov)) = (slot_from(&env.slot), open_provider()) {
                    let _ = delete_slot_key(&prov, slot);
                }
            }
            let _ = std::fs::remove_file(&newp);
        }
        // —— 3. 孤儿 slot ——
        if let Ok(env) = read_envelope(&dir.join(ENVELOPE)) {
            if let (Some(live), Ok(prov)) = (slot_from(&env.slot), open_provider()) {
                let _ = delete_slot_key(&prov, other_slot(live));
            }
        }
    }

    // ---- 引擎操作（命令薄包装它们；测试直接调） ----

    /// 读当前安全状态。**调用前须先 reconcile**（security_status 命令已保证）：否则中断的销毁/迁移未自愈，
    /// destroyed/encrypted 判定可能基于半态。仅经 security_status 命令调用，勿在别处直接调 status() 而不 reconcile。
    pub(super) fn status(dir: &Path) -> SecurityStatus {
        let envelope = dir.join(ENVELOPE);
        let (encrypted, scheme) = match read_envelope(&envelope) {
            Ok(env) => (true, Some(env.scheme)),
            Err(_) if envelope.exists() => (true, None), // 文件在但坏 → 仍算已加密（损坏态）
            Err(_) => (false, None),
        };
        let sec = read_security(dir);
        SecurityStatus {
            encrypted,
            scheme,
            tpm_available: open_provider().is_ok(),
            last_backup_unix: sec.last_backup_unix,
            last_backup_path: sec.last_backup_path,
            destroyed: is_destroyed(dir),
            fail_count: sec.fail_count,
            destroy_enabled: sec.destroy_enabled,
            destroy_threshold: DESTROY_THRESHOLD,
        }
    }

    /// 首次设密码（含明文→密文迁移，§9）：随机 DEK → slot A 建钥匙(口令) → 封 DEK → 写迁移标记 →
    /// **迁移 heng.db 明文→密文（原子 rename = 提交点）** → 提交信封 → 删标记。返回 DEK（命令存 Rust 侧）。
    /// 调用方须已关闭 State 连接，使 heng.db 文件不被占用。
    pub(super) fn set_password(dir: &Path, pw: &str) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
        reconcile(dir); // 先healing 任何上次中断残留，从一致态开始
        if dir.join(ENVELOPE).exists() {
            return Err(internal("already encrypted".into()));
        }
        let prov = open_provider()?;
        let dek = gen_dek()?;
        let key = create_slot_key(&prov, Slot::A, pw)?;
        let ct = wrap_dek(&key, &dek)?;
        let env = WrapEnvelope::new(Slot::A, &ct);
        write_marker(dir, &MigrateMarker { direction: "encrypt".into(), envelope: env.clone() })?;
        if let Err(e) = migrate_encrypt(dir, &dek) {
            // 迁移失败 ⇒ heng.db 仍明文（migrate_encrypt 已删 tmp）→ 完整回滚：删 staging slot + 标记。
            let _ = delete_slot_key(&prov, Slot::A);
            let _ = std::fs::remove_file(dir.join(MIGRATE_MARKER));
            return Err(e);
        }
        // heng.db 已是密文 ⇒ 提交信封（幂等原子）。若此处失败：标记仍在 + 库密文 → 下次启动 reconcile 前滚。
        write_envelope_atomic(dir, &env)?;
        let _ = std::fs::remove_file(dir.join(MIGRATE_MARKER));
        Ok(dek)
    }

    /// 解锁：自愈 → 读信封 → 开 slot → 解封 DEK。错口令 → WrongPassword（1 DA strike）。
    pub(super) fn unlock(dir: &Path, pw: &str) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
        reconcile(dir);
        let env = read_envelope(&dir.join(ENVELOPE))?;
        let slot = slot_from(&env.slot).ok_or_else(|| corrupt("unknown slot"))?;
        let ct = decode_hex(&env.wrapped_dek_hex).ok_or_else(|| corrupt("bad ciphertext hex"))?;
        let prov = open_provider()?;
        let key = open_slot_key(&prov, slot, pw)?;
        unwrap_dek(&key, &ct)
    }

    /// 改密＝重封 under 新钥匙（两阶段原子）：
    /// 1. 旧口令解出 DEK（验证旧口令）。2. 另一 slot 用新口令封同一 DEK。
    /// 3. 写 staging `.new`(+sync)。4. 从 staging 验证可解且 ==DEK。
    /// 5. 原子 rename `.new`→信封（commit 点）。6. 删旧 slot 钥匙。
    pub(super) fn change_password(dir: &Path, old: &str, new: &str) -> Result<(), CryptoError> {
        reconcile(dir);
        let env = read_envelope(&dir.join(ENVELOPE))?;
        let live = slot_from(&env.slot).ok_or_else(|| corrupt("unknown slot"))?;
        let ct = decode_hex(&env.wrapped_dek_hex).ok_or_else(|| corrupt("bad ciphertext hex"))?;
        let prov = open_provider()?;
        let dek = {
            let k = open_slot_key(&prov, live, old)?;
            unwrap_dek(&k, &ct)?
        };
        let target = other_slot(live);
        let newkey = create_slot_key(&prov, target, new)?;
        let newct = wrap_dek(&newkey, &dek)?;
        let newenv = WrapEnvelope::new(target, &newct);
        let newp = dir.join(ENVELOPE_NEW);
        write_sync(&newp, &newenv)?;
        // 验证（用新的、正确的口令 ⇒ 0 DA）
        {
            let vkey = open_slot_key(&prov, target, new)?;
            let vdek = unwrap_dek(&vkey, &newct)?;
            if vdek[..] != dek[..] {
                let _ = std::fs::remove_file(&newp);
                let _ = delete_slot_key(&prov, target);
                return Err(corrupt("re-wrap verification mismatch"));
            }
        }
        rename_with_retry(&newp, &dir.join(ENVELOPE))?; // commit（与迁移一致用重试 rename，吃掉 AV/索引器短暂持锁）
        let _ = delete_slot_key(&prov, live); // 失败不致命，reconcile 兜底
        Ok(())
    }

    /// 移除密码（含密文→明文反向迁移，§9）：验证口令（解出 DEK）→ 写迁移标记 →
    /// **迁移 heng.db 密文→明文（原子 rename = 提交点）** → 删信封 + 两 slot → 删标记。
    /// 调用方须已关闭 State 连接。提交点前失败 ⇒ 留在加密态（信封/slot 都在）。
    pub(super) fn remove_password(dir: &Path, pw: &str) -> Result<(), CryptoError> {
        reconcile(dir);
        let env = read_envelope(&dir.join(ENVELOPE))?;
        let slot = slot_from(&env.slot).ok_or_else(|| corrupt("unknown slot"))?;
        let ct = decode_hex(&env.wrapped_dek_hex).ok_or_else(|| corrupt("bad ciphertext hex"))?;
        let prov = open_provider()?;
        let dek = {
            let k = open_slot_key(&prov, slot, pw)?;
            unwrap_dek(&k, &ct)? // 验证口令；错则 WrongPassword（迁移尚未开始，不动库）
        };
        write_marker(dir, &MigrateMarker { direction: "decrypt".into(), envelope: env.clone() })?;
        if let Err(e) = migrate_decrypt(dir, &dek) {
            // 迁移失败 ⇒ heng.db 仍密文（已删 tmp）+ 信封/slot 都在 → 一致的加密态；删标记后原样返回。
            let _ = std::fs::remove_file(dir.join(MIGRATE_MARKER));
            return Err(e);
        }
        // heng.db 已明文 ⇒ 拆封装：先删两 slot + 删信封；仅当 slot 全删净才删标记，否则保留标记
        // （含 slot 信息）作锚点、下次 reconcile 重试，杜绝「信封已删 + slot 残留 + 无锚点」的孤儿 slot（review enc-2/5）。
        let slots_gone = delete_both_slots(Some(slot));
        let _ = std::fs::remove_file(dir.join(ENVELOPE));
        if slots_gone {
            let _ = std::fs::remove_file(dir.join(MIGRATE_MARKER));
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::db::open_db; // 仅测试用（迁移/TPM roundtrip 开库读回）

        // ---- 纯逻辑（无 TPM、可自动跑、parallel-safe） ----

        #[test]
        fn usage_auth_digest_is_utf16le() {
            // "A" → UTF-16LE = 0x41 0x00；摘要应等于 SHA256 of those 2 bytes（钉住编码，防 UTF-8/大端跑偏，
            // 并隐含「无尾随 NUL」——多塞一个 NUL 会变 3 字节、对不上）。
            assert_eq!(usage_auth_digest("A").to_vec(), Sha256::digest([0x41u8, 0x00]).to_vec());
            // 多码元顺序："AB" → 0x41 00 42 00（钉住小端 + 字节不串位）。
            assert_eq!(usage_auth_digest("AB").to_vec(), Sha256::digest([0x41u8, 0x00, 0x42, 0x00]).to_vec());
            // 空口令 → SHA256 of 空串（别误塞 NUL/盐）。
            assert_eq!(usage_auth_digest("").to_vec(), Sha256::digest([]).to_vec());
            assert_eq!(usage_auth_digest("x").len(), 32);
            assert_ne!(usage_auth_digest("a").to_vec(), usage_auth_digest("b").to_vec());
        }

        #[test]
        fn hex_roundtrip() {
            let b = [0x00u8, 0x9f, 0xff, 0x10];
            assert_eq!(decode_hex(&encode_hex(&b)), Some(b.to_vec()));
            assert_eq!(decode_hex("abc"), None); // 奇数长度
            assert_eq!(decode_hex("zz"), None); // 非十六进制
        }

        #[test]
        fn envelope_serde_roundtrip() {
            let env = WrapEnvelope::new(Slot::B, &[1u8, 2, 3, 4]);
            let bytes = serde_json::to_vec(&env).unwrap();
            let back: WrapEnvelope = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(back.slot, "b");
            assert_eq!(back.version, 1);
            assert_eq!(back.scheme, "tpm-pcp");
            assert_eq!(back.wrapped_dek_hex, "01020304");
        }

        #[test]
        fn classify_maps_known_codes() {
            assert_eq!(classify(NTE_PERM), FailClass::WrongPassword);
            assert_eq!(classify(TPM_20_E_LOCKOUT), FailClass::Locked);
            assert_eq!(classify(TPM_E_DEFEND_LOCK_RUNNING), FailClass::Locked);
            assert_eq!(classify(NTE_BAD_DATA), FailClass::Corrupt);
            assert_eq!(classify(NTE_NOT_FOUND), FailClass::ChipUnavailable);
            assert_eq!(classify(NTE_BAD_KEYSET), FailClass::ChipUnavailable);
            assert_eq!(classify(0xdead_beef), FailClass::ChipUnavailable);
        }

        #[test]
        fn slot_helpers() {
            assert_eq!(other_slot(Slot::A), Slot::B);
            assert_eq!(slot_from("a"), Some(Slot::A));
            assert_eq!(slot_from("x"), None);
        }

        #[test]
        fn marker_serde_roundtrip() {
            let m = MigrateMarker { direction: "encrypt".into(), envelope: WrapEnvelope::new(Slot::A, &[9u8, 8, 7]) };
            let back: MigrateMarker = serde_json::from_slice(&serde_json::to_vec(&m).unwrap()).unwrap();
            assert_eq!(back.direction, "encrypt");
            assert_eq!(back.envelope.slot, "a");
            assert_eq!(back.envelope.wrapped_dek_hex, "090807");
        }

        #[test]
        fn db_is_plaintext_detects_header() {
            let dir = std::env::temp_dir().join(format!("heng-hdr-test-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            // 真明文 SQLite 库 → true
            let plain = dir.join("plain.db");
            Connection::open(&plain).unwrap().execute_batch("CREATE TABLE t(x)").unwrap();
            assert!(db_is_plaintext(&plain));
            // 随机字节（模拟密文头）→ false；缺失 → false
            let cipherish = dir.join("cipher.db");
            std::fs::write(&cipherish, [0xABu8; 64]).unwrap();
            assert!(!db_is_plaintext(&cipherish));
            assert!(!db_is_plaintext(&dir.join("missing.db")));
            let _ = std::fs::remove_dir_all(&dir);
        }

        /// 明→密→明 迁移：数据 + user_version 全程保留，库头随之翻转。**纯 DEK（无 TPM）、0 DA、自动跑**——
        /// 覆盖本阶段最高风险的新代码（sqlcipher_export + user_version 补传 + 原子 rename）。
        #[test]
        fn migration_roundtrip_preserves_data_and_user_version() {
            let dir = std::env::temp_dir().join(format!("heng-migrate-test-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let db = dir.join(DB_FILE);
            {
                let conn = Connection::open(&db).unwrap();
                conn.execute_batch("PRAGMA user_version=7; CREATE TABLE t(x TEXT); INSERT INTO t VALUES('a'),('b'),('c');")
                    .unwrap();
            }
            let dek = [0x11u8; 32];
            // 明 → 密
            migrate_encrypt(&dir, &dek).unwrap();
            assert!(!db_is_plaintext(&db), "迁移后库头不应是明文");
            {
                let conn = open_db(&db, Some(&encode_hex(&dek))).unwrap();
                let n: i64 = conn.query_row("SELECT count(*) FROM t", [], |r| r.get(0)).unwrap();
                assert_eq!(n, 3);
                let uv: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
                assert_eq!(uv, 7, "user_version 必须随迁移保留（否则下次启动重跑全部迁移）");
            }
            // 密 → 明
            migrate_decrypt(&dir, &dek).unwrap();
            assert!(db_is_plaintext(&db), "解密后库头应是明文");
            {
                let conn = open_db(&db, None).unwrap();
                let n: i64 = conn.query_row("SELECT count(*) FROM t", [], |r| r.get(0)).unwrap();
                assert_eq!(n, 3);
                let uv: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
                assert_eq!(uv, 7);
            }
            let _ = std::fs::remove_dir_all(&dir);
        }

        /// 备份导出（明文库 + 加密库两条路）：备份恒明文、数据+user_version 保留、记 heng.security。**0 DA、无 TPM、自动跑**
        /// （加密路用假信封 + 已知 DEK：export_backup 只凭信封存在判加密 + 用传入 DEK 解，不碰真 TPM）。
        #[test]
        fn backup_roundtrip_plaintext_and_encrypted() {
            let dir = std::env::temp_dir().join(format!("heng-backup-test-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let db = dir.join(DB_FILE);
            // —— 明文库导出 ——
            {
                let conn = open_db(&db, None).unwrap();
                conn.execute_batch("PRAGMA user_version=9; CREATE TABLE t(x TEXT); INSERT INTO t VALUES('a'),('b');").unwrap();
            }
            let dest1 = dir.join("backup1.db");
            let info1 = export_backup(&dir, None, dest1.to_str().unwrap()).unwrap();
            assert_eq!(info1.rows, 2);
            assert!(db_is_plaintext(&dest1), "备份必须是明文");
            {
                let conn = open_db(&dest1, None).unwrap();
                assert_eq!(conn.query_row("SELECT count(*) FROM t", [], |r| r.get::<_, i64>(0)).unwrap(), 2);
                assert_eq!(conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0)).unwrap(), 9);
            }
            let sec = read_security(&dir);
            assert_eq!(sec.last_backup_path.as_deref(), Some(dest1.to_string_lossy().as_ref()));
            assert!(sec.last_backup_unix.is_some() && sec.last_backup_sha256.is_some());

            // —— 加密库导出（假信封 + 已知 DEK）——
            std::fs::remove_file(&db).unwrap();
            let dek = [0x22u8; 32];
            {
                let conn = open_db(&db, Some(&encode_hex(&dek))).unwrap();
                conn.execute_batch("PRAGMA user_version=9; CREATE TABLE t(x TEXT); INSERT INTO t VALUES('a'),('b'),('c');").unwrap();
            }
            std::fs::write(dir.join(ENVELOPE), b"dummy-envelope").unwrap();
            let dest2 = dir.join("backup2.db");
            let info2 = export_backup(&dir, Some(&dek), dest2.to_str().unwrap()).unwrap();
            assert_eq!(info2.rows, 3);
            assert!(db_is_plaintext(&dest2), "加密库的备份也必须是明文（关闭加密的等价物）");
            {
                let conn = open_db(&dest2, None).unwrap();
                assert_eq!(conn.query_row("SELECT count(*) FROM t", [], |r| r.get::<_, i64>(0)).unwrap(), 3);
                assert_eq!(conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0)).unwrap(), 9);
            }
            // 加密库未传 DEK → 拒
            assert!(export_backup(&dir, None, dir.join("x.db").to_str().unwrap()).is_err());
            let _ = std::fs::remove_dir_all(&dir);
        }

        /// 备份目标路径防撞：拒绝导到应用数据目录里的 heng.* 控制文件，允许同目录非 heng.* 名。
        #[test]
        fn backup_rejects_control_paths() {
            let dir = std::env::temp_dir().join(format!("heng-backup-reject-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            {
                let conn = open_db(&dir.join(DB_FILE), None).unwrap();
                conn.execute_batch("CREATE TABLE t(x)").unwrap();
            }
            for name in ["heng.db", "heng.dek.tpm", "heng.security", "heng.migrate"] {
                assert!(export_backup(&dir, None, dir.join(name).to_str().unwrap()).is_err(), "{name} 应被拒");
            }
            assert!(export_backup(&dir, None, dir.join("mybackup.db").to_str().unwrap()).is_ok());
            let _ = std::fs::remove_dir_all(&dir);
        }

        // ---- 销毁（4b）。文件态断言与 TPM 无关；delete_both_slots 在本机 fTPM 上删不存在的 slot＝0-DA no-op ----

        fn setup_destroyable(dir: &Path, fail_count: u32, destroy_enabled: bool) -> std::path::PathBuf {
            let _ = std::fs::remove_dir_all(dir);
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join(ENVELOPE), b"dummy-envelope").unwrap();
            std::fs::write(dir.join(DB_FILE), b"cipher-db-bytes-not-plaintext").unwrap();
            let bak = dir.join("backup.db");
            std::fs::write(&bak, b"plaintext-backup-data").unwrap();
            let sec = SecurityFile {
                fail_count,
                destroy_enabled,
                last_backup_path: Some(bak.to_string_lossy().into_owned()),
                last_backup_sha256: Some(sha256_file(&bak).unwrap()),
                ..Default::default()
            };
            write_security(dir, &sec).unwrap(); // 盖当前信封哈希 → read 时校验通过
            bak
        }

        #[test]
        fn verify_backup_checks_presence_and_hash() {
            let dir = std::env::temp_dir().join(format!("heng-vbak-{}", std::process::id()));
            let bak = setup_destroyable(&dir, 0, false);
            let sec = read_security(&dir);
            assert!(verify_backup(&sec).is_ok(), "完好备份应过");
            std::fs::write(&bak, b"TAMPERED").unwrap(); // 改动 → sha 变
            assert!(verify_backup(&sec).is_err(), "改动的备份应拒");
            std::fs::remove_file(&bak).unwrap();
            assert!(verify_backup(&sec).is_err(), "缺失的备份应拒");
            let _ = std::fs::remove_dir_all(&dir);
        }

        #[test]
        fn set_destroy_enabled_requires_session_backup_and_valid_file() {
            let dir = std::env::temp_dir().join(format!("heng-sde-{}", std::process::id()));
            let bak = setup_destroyable(&dir, 0, false);
            assert!(set_destroy_enabled(&dir, true, false).is_err(), "无本会话备份 → 拒开");
            assert!(set_destroy_enabled(&dir, true, true).is_ok(), "有会话备份+备份完好 → 可开");
            assert!(read_security(&dir).destroy_enabled);
            // 备份没了 → 不能再开
            std::fs::remove_file(&bak).unwrap();
            assert!(set_destroy_enabled(&dir, true, true).is_err(), "备份缺失 → 拒开");
            // 关闭恒可
            assert!(set_destroy_enabled(&dir, false, false).is_ok());
            let _ = std::fs::remove_dir_all(&dir);
        }

        #[test]
        fn wrong_password_destroys_at_threshold_only_with_valid_backup() {
            // 第 5 次错口令 + 开了销毁 + 备份完好 → 销毁
            let dir = std::env::temp_dir().join(format!("heng-wpd-{}", std::process::id()));
            setup_destroyable(&dir, 4, true);
            let destroyed = on_wrong_password(&dir).unwrap();
            assert!(destroyed, "第5次错口令应触发销毁");
            assert!(is_destroyed(&dir), "sentinel 应在");
            assert!(!dir.join(ENVELOPE).exists(), "信封应删");
            assert!(!dir.join(DB_FILE).exists(), "密文库应移走");
            let q = std::fs::read_dir(&dir).unwrap().flatten()
                .find(|e| e.file_name().to_string_lossy().starts_with(QUARANTINE_PREFIX)).expect("应有隔离目录");
            assert!(q.path().join(DB_FILE).exists(), "密文库应在隔离区（墓碑、移而不删）");
            assert!(q.path().join("README.txt").exists());
            assert!(status(&dir).destroyed);
            // 从空白重新开始 → 清理干净
            restart_after_destroy(&dir).unwrap();
            assert!(!is_destroyed(&dir));
            assert!(!dir.join(DB_FILE).exists());
            assert!(std::fs::read_dir(&dir).unwrap().flatten()
                .all(|e| !e.file_name().to_string_lossy().starts_with(QUARANTINE_PREFIX)), "隔离墓碑应清掉");
            let _ = std::fs::remove_dir_all(&dir);

            // 备份损坏 → 第 5 次错口令**中止销毁**、数据保留
            let dir2 = std::env::temp_dir().join(format!("heng-wpd2-{}", std::process::id()));
            let bak2 = setup_destroyable(&dir2, 4, true);
            std::fs::write(&bak2, b"corrupted-now").unwrap(); // sha 不再匹配
            let destroyed2 = on_wrong_password(&dir2).unwrap();
            assert!(!destroyed2, "备份损坏 → 中止销毁");
            assert!(dir2.join(ENVELOPE).exists() && dir2.join(DB_FILE).exists(), "未销毁，数据保留");
            assert!(!is_destroyed(&dir2));
            let _ = std::fs::remove_dir_all(&dir2);
        }

        #[test]
        fn destroy_heal_forward_rolls_interrupted_destroy() {
            // 模拟「写了 destroying 标记 + 库还在 + 无 sentinel」的中断态 → reconcile 前滚补完。
            let dir = std::env::temp_dir().join(format!("heng-dheal-{}", std::process::id()));
            setup_destroyable(&dir, 5, true);
            let qdir = dir.join(format!("{QUARANTINE_PREFIX}interrupted"));
            write_destroy_marker(&dir, &qdir).unwrap();
            // 标记一写即视为 destroyed（提交点）；sentinel 尚未落盘。
            assert!(dir.join(DESTROYING_MARKER).exists() && is_destroyed(&dir));
            assert!(!dir.join(DESTROYED_SENTINEL).exists(), "sentinel 尚未写");
            // reconcile 最前面处理 destroy：前滚补完。
            reconcile(&dir);
            assert!(dir.join(DESTROYED_SENTINEL).exists(), "heal 应补写 sentinel");
            assert!(is_destroyed(&dir));
            assert!(!dir.join(ENVELOPE).exists(), "heal 应删信封");
            assert!(qdir.join(DB_FILE).exists(), "heal 应把库移进标记记录的隔离目录");
            let _ = std::fs::remove_dir_all(&dir);
        }

        // ---- 真 TPM 集成测试（消耗用户机芯片 + DA；默认 #[ignore]） ----
        // 这些用固定 slot a/b ⇒ **必须串行**：手动跑
        //   build-tauri.bat 同环境下 `cargo test -- --ignored --test-threads=1`
        // hex 小工具，仅测试用。
        fn hx(d: &[u8; 32]) -> String {
            encode_hex(d)
        }

        /// 清场（DA-free）：删临时目录 + best-effort 删两 slot 钥匙，让 #[ignore] 测试可重复跑——
        /// 不被上次 panic 残留的 envelope（会触发 set_password 的「already encrypted」）或孤儿钥匙绊倒。
        fn reset_tpm_test(dir: &Path) {
            let _ = std::fs::remove_dir_all(dir);
            std::fs::create_dir_all(dir).unwrap();
            if let Ok(prov) = open_provider() {
                let _ = delete_slot_key(&prov, Slot::A);
                let _ = delete_slot_key(&prov, Slot::B);
            }
        }

        /// 设密码→解锁→改密→解锁→移除，**全程正确口令 ⇒ 0 DA strike**。
        #[test]
        #[ignore = "touches real TPM; run manually, serial"]
        fn tpm_set_unlock_change_remove_no_strikes() {
            let dir = std::env::temp_dir().join(format!("heng-crypto-it-{}", std::process::id()));
            reset_tpm_test(&dir);
            let dek1 = set_password(&dir, "correct-pw-1").unwrap();
            let dek2 = unlock(&dir, "correct-pw-1").unwrap();
            assert_eq!(dek1[..], dek2[..], "解出的 DEK 应与封入的一致");
            change_password(&dir, "correct-pw-1", "correct-pw-2").unwrap();
            let dek3 = unlock(&dir, "correct-pw-2").unwrap();
            assert_eq!(dek1[..], dek3[..], "改密不改 DEK");
            remove_password(&dir, "correct-pw-2").unwrap();
            assert!(!status(&dir).encrypted, "移除后应未加密");
            let _ = std::fs::remove_dir_all(&dir);
        }

        /// reconcile 自愈：模拟改密在 commit(rename) 前崩溃（已写 .new + 建好新 slot，未顶替）→
        /// 下次解锁应**回滚**（删 staging slot 钥匙 + 删 .new），旧口令仍解出原 DEK。全程正确口令 ⇒ 0 DA。
        #[test]
        #[ignore = "touches real TPM; run manually, serial"]
        fn tpm_change_password_crash_before_commit_rolls_back() {
            let dir = std::env::temp_dir().join(format!("heng-crypto-rollback-{}", std::process::id()));
            reset_tpm_test(&dir);
            let dek = set_password(&dir, "old-pw").unwrap(); // envelope{A} + slot A
            // 手工构造「staging 已写、未 commit」的中断态：slot B 用新口令封同一 DEK，写 .new（不 rename）。
            {
                let prov = open_provider().unwrap();
                let bkey = create_slot_key(&prov, Slot::B, "new-pw").unwrap();
                let bct = wrap_dek(&bkey, &dek).unwrap();
                write_sync(&dir.join(ENVELOPE_NEW), &WrapEnvelope::new(Slot::B, &bct)).unwrap();
            }
            assert!(dir.join(ENVELOPE_NEW).exists());
            // 解锁触发 reconcile：回滚 → .new 删除、旧口令解出同一 DEK。
            let dek2 = unlock(&dir, "old-pw").unwrap();
            assert_eq!(dek[..], dek2[..], "回滚后旧口令应解出原 DEK");
            assert!(!dir.join(ENVELOPE_NEW).exists(), "reconcile 应删未提交的 .new");
            // slot B 应已被回滚删除：重新走一遍完整改密应成功（再建 slot B）。
            change_password(&dir, "old-pw", "new2-pw").unwrap();
            let dek3 = unlock(&dir, "new2-pw").unwrap();
            assert_eq!(dek[..], dek3[..], "改密不改 DEK");
            remove_password(&dir, "new2-pw").unwrap();
            let _ = std::fs::remove_dir_all(&dir);
        }

        /// 加密一个测试 SQLCipher 库 → 锁 → 解 → 读回；**全程正确口令 ⇒ 0 DA**。
        #[test]
        #[ignore = "touches real TPM; run manually, serial"]
        fn tpm_db_encrypt_lock_unlock() {
            let dir = std::env::temp_dir().join(format!("heng-crypto-db-{}", std::process::id()));
            reset_tpm_test(&dir);
            let dbf = dir.join("t.db");
            let dek = set_password(&dir, "pw").unwrap();
            {
                let conn = open_db(&dbf, Some(&hx(&dek))).unwrap();
                conn.execute_batch("CREATE TABLE t(x TEXT); INSERT INTO t VALUES('marker');").unwrap();
            }
            let dek2 = unlock(&dir, "pw").unwrap(); // 锁→解
            {
                let conn = open_db(&dbf, Some(&hx(&dek2))).unwrap();
                let v: String = conn.query_row("SELECT x FROM t", [], |r| r.get(0)).unwrap();
                assert_eq!(v, "marker");
            }
            remove_password(&dir, "pw").unwrap();
            let _ = std::fs::remove_dir_all(&dir);
        }

        /// 唯一一次蓄意错口令：芯片必须拒绝（WrongPassword）。**恰好消耗 1 DA strike。**
        #[test]
        #[ignore = "DELIBERATELY consumes exactly 1 DA strike; run manually, once, serial"]
        fn tpm_wrong_password_one_strike() {
            let dir = std::env::temp_dir().join(format!("heng-crypto-wrong-{}", std::process::id()));
            reset_tpm_test(&dir);
            set_password(&dir, "the-correct-one").unwrap();
            let e = unlock(&dir, "WRONG").unwrap_err(); // ← 唯一一次错口令
            assert_eq!(e.class, FailClass::WrongPassword, "错口令应被芯片拒绝, got {e:?}");
            remove_password(&dir, "the-correct-one").unwrap();
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

// ---- Tauri 命令（薄包装；DEK 只存 Rust 侧 Crypto state，绝不回传 JS） ----

/// 基建错（目录解析 / 开库 IO 等，非解锁三态）→ FailClass::Internal。
fn internal_err(m: String) -> CryptoError {
    CryptoError { class: FailClass::Internal, code: 0, message: m }
}

#[tauri::command]
pub fn security_status(app: AppHandle, crypto: State<Crypto>) -> Result<SecurityStatus, String> {
    let dir = config_dir(&app)?;
    let _op = crypto.0.lock().unwrap(); // 串行化（不与改密/迁移的 rename 撞读）
    engine::reconcile(&dir); // 启动门先自愈：把任何中断的迁移/改密healing 到一致态，再据此判定加密与否
    Ok(engine::status(&dir))
}

/// 设密码（含明文→密文迁移）。全程持 Crypto + Db 两把锁（序 Crypto→Db，与 db_open/lock 一致）：
/// 关闭 State 连接 → 迁移 → 重开。迁移期间 db_select 等会阻塞在 Db 锁上、待重开后继续。
#[tauri::command]
pub fn set_password(
    app: AppHandle,
    crypto: State<Crypto>,
    db: State<Db>,
    password: String,
) -> Result<(), CryptoError> {
    let dir = config_dir(&app).map_err(internal_err)?;
    let db_file = dir.join(DB_FILE);
    let mut dek_slot = crypto.0.lock().unwrap();
    let mut conn_slot = db.0.lock().unwrap();
    *conn_slot = None; // 释放 heng.db 文件供原子替换
    match engine::set_password(&dir, &password) {
        Ok(dek) => {
            // 迁移已提交＝库已密文。DEK 先入 state：即便随后 open_db 因瞬时文件锁失败，
            // 本会话后续 db_open(encrypted) 或重启解锁仍能用它开库，不丢密钥（review lock-1/sec-1）。
            let opened = open_db(&db_file, Some(&dek_hex(&dek)));
            dek_slot.dek = Some(dek);
            *conn_slot = Some(opened.map_err(internal_err)?);
            Ok(())
        }
        Err(e) => {
            // 失败：engine 已尽力回滚；reconcile 兜底healing。据库头重开连接（明文常态；若已前滚成密文则本会话无 DEK，留待重启解锁）。
            engine::reconcile(&dir);
            if engine::db_is_plaintext(&db_file) {
                if let Ok(conn) = open_db(&db_file, None) {
                    *conn_slot = Some(conn);
                }
            }
            Err(e)
        }
    }
}

/// 解锁。全程持 Crypto 锁，**串行化失败计数的读改写**（否则成功归零会被并发的错误尝试覆写回去 → 正确口令后仍被销毁）。
/// 计数纪律（§5）：**仅** WrongPassword +1；成功归零；Locked/Corrupt/ChipUnavailable/Internal **不动计数**。
/// 计数达阈值且开了销毁且备份验得过 → 销毁，返回 FailClass::Destroyed（UI 跳终态屏）。
#[tauri::command]
pub fn unlock(app: AppHandle, crypto: State<Crypto>, password: String) -> Result<(), CryptoError> {
    let dir = config_dir(&app).map_err(internal_err)?;
    let mut state = crypto.0.lock().unwrap();
    match engine::unlock(&dir, &password) {
        Ok(dek) => {
            engine::on_unlock_success(&dir); // 失败计数归零
            state.dek = Some(dek);
            state.backed_up_session = false; // 新会话：重置"本会话已备份"标记
            // bootstrap 在解锁成功后调 db_open(encrypted=true)，从 Crypto state 取该 DEK 开 SQLCipher 库。
            Ok(())
        }
        Err(e) if e.class == FailClass::WrongPassword => {
            // 错口令 +1；达阈值且开了销毁且备份完好 → 销毁。
            match engine::on_wrong_password(&dir) {
                Ok(true) => {
                    state.dek = None;
                    Err(CryptoError {
                        class: FailClass::Destroyed,
                        code: 0,
                        message: "连续输错密码达上限，数据已按安全设置销毁".into(),
                    })
                }
                _ => Err(e), // 未达阈值/未开销毁/备份验不过(中止销毁、数据保留) → 原样返回错口令
            }
        }
        Err(e) => Err(e), // Locked/Corrupt/ChipUnavailable/Internal：不动计数
    }
}

#[tauri::command]
pub fn change_password(
    app: AppHandle,
    crypto: State<Crypto>,
    old_password: String,
    new_password: String,
) -> Result<(), CryptoError> {
    let dir = config_dir(&app).map_err(internal_err)?;
    let _op = crypto.0.lock().unwrap(); // 持锁全程：改密的两阶段原子序列必须独占（不动数据库密文）
    engine::change_password(&dir, &old_password, &new_password)
}

/// 移除密码（含密文→明文迁移）。锁/连接处理同 set_password。
#[tauri::command]
pub fn remove_password(
    app: AppHandle,
    crypto: State<Crypto>,
    db: State<Db>,
    password: String,
) -> Result<(), CryptoError> {
    let dir = config_dir(&app).map_err(internal_err)?;
    let db_file = dir.join(DB_FILE);
    let mut dek_slot = crypto.0.lock().unwrap();
    let mut conn_slot = db.0.lock().unwrap();
    *conn_slot = None;
    match engine::remove_password(&dir, &password) {
        Ok(()) => {
            let conn = open_db(&db_file, None).map_err(internal_err)?; // 库已明文
            *conn_slot = Some(conn);
            dek_slot.dek = None; // 清掉已解锁 DEK
            Ok(())
        }
        Err(e) => {
            engine::reconcile(&dir); // 与 set_password 错误路径对称：healing 任何中断的迁移再重开连接
            // 提交点前失败 ⇒ 仍是加密态；用本会话已解锁的 DEK 重开密文连接（移除失败不清 dek_slot）。
            if let Some(dek) = dek_slot.dek.as_ref() {
                if let Ok(conn) = open_db(&db_file, Some(&dek_hex(dek))) {
                    *conn_slot = Some(conn);
                }
            } else if engine::db_is_plaintext(&db_file) {
                if let Ok(conn) = open_db(&db_file, None) {
                    *conn_slot = Some(conn);
                }
            }
            Err(e)
        }
    }
}

/// 锁定：清掉已解锁 DEK + 关闭 DB 连接（自动锁 / 手动锁用）。之后 UI 回到解锁屏，重新 unlock→db_open。
#[tauri::command]
pub fn lock(crypto: State<Crypto>, db: State<Db>) -> Result<(), String> {
    let mut state = crypto.0.lock().unwrap();
    let mut conn = db.0.lock().unwrap();
    state.dek = None;
    state.backed_up_session = false; // 新会话重新要求备份才能开销毁
    *conn = None;
    Ok(())
}

/// 导出一份未加密备份到用户选定路径（dest_path 由 JS 侧原生「另存为」对话框给定）。
/// 加密库用已解锁 DEK 解出明文写出；未加密库直接明文导出。持 Crypto 锁串行化（不与迁移 rename 撞）。
/// 成功后标记"本会话已备份"（强闸门：开销毁前要求本会话内已成功备份）。
#[tauri::command]
pub fn export_backup(app: AppHandle, crypto: State<Crypto>, dest_path: String) -> Result<BackupInfo, CryptoError> {
    let dir = config_dir(&app).map_err(internal_err)?;
    let mut state = crypto.0.lock().unwrap(); // 串行化 + 取已解锁 DEK
    let info = engine::export_backup(&dir, state.dek.as_deref(), &dest_path)?;
    state.backed_up_session = true;
    Ok(info)
}

/// 开/关「错 N 次销毁」。开启要求：本次解锁会话内成功备份过 + 记录的备份仍在且完整（强闸门）。
#[tauri::command]
pub fn set_destroy_enabled(app: AppHandle, crypto: State<Crypto>, enabled: bool) -> Result<(), CryptoError> {
    let dir = config_dir(&app).map_err(internal_err)?;
    let state = crypto.0.lock().unwrap();
    engine::set_destroy_enabled(&dir, enabled, state.backed_up_session)
}

/// 销毁后「从空白重新开始」：清 sentinel + 隔离墓碑 + heng.security，下次 db_open 建全新空明文库。
#[tauri::command]
pub fn restart_after_destroy(app: AppHandle, crypto: State<Crypto>, db: State<Db>) -> Result<(), CryptoError> {
    let dir = config_dir(&app).map_err(internal_err)?;
    let mut state = crypto.0.lock().unwrap();
    let mut conn = db.0.lock().unwrap();
    *conn = None;
    state.dek = None;
    state.backed_up_session = false;
    engine::restart_after_destroy(&dir)
}
