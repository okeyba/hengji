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
//! 本阶段边界（清晰标注，避免越界到后续阶段）：
//! - **不**做明文↔密文库迁移（Phase 4 §9）；set/remove 仅建立/拆除封装。
//! - **不**接 bootstrap / 解锁 UI（Phase 3）；命令只把 DEK 存在 Rust 侧，**绝不**回传 JS。
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

use crate::db::config_dir;

/// 已解锁的 DEK（Zeroizing：替换/drop 时清零）。仅存在 Rust 侧，绝不跨 IPC 回传 JS。
/// 这把锁还**串行化所有 crypto 命令**：Tauri 同步命令跑在线程池上，可并发；改密/解锁/移除都在
/// 固定 slot a/b + 单一 heng.dek.tpm 上做多步非原子操作，若并发会互踩（如改密 commit 与另一路
/// reconcile 的删 slot 撞车 → 封装文件指向的 slot 钥匙被删 = 库永久不可解）。命令全程持此锁互斥。
pub struct Crypto(pub Mutex<Option<Zeroizing<[u8; 32]>>>);

/// 解锁/封装失败的分流（§5 三类 + Internal 基建错）。UI 据此分屏：
/// - WrongPassword：口令错/锁定 → 计入销毁计数（Phase 4）。
/// - Corrupt：信封/密文损坏 → “数据可能损坏”。
/// - ChipUnavailable：芯片占用/句柄异常/钥匙缺失 → “芯片暂不可用，请重启”，**不计销毁**。
/// - Internal：目录解析/IO/序列化等基建错（非解锁三态）。
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailClass {
    WrongPassword,
    Corrupt,
    ChipUnavailable,
    Internal,
}

/// 跨 IPC 回传给 JS 的错误：粗分类 + 原始 HRESULT（供 UI 细化，如锁定 vs 单纯错口令）+ 文案。
#[derive(Serialize, Debug, Clone)]
pub struct CryptoError {
    pub class: FailClass,
    pub code: u32,
    pub message: String,
}

/// 给 Phase 3 状态行的三态判定输入（强/弱/无 由 scheme + 本期仅 tpm-pcp 推出）。
#[derive(Serialize, Debug, Clone)]
pub struct SecurityStatus {
    /// heng.dek.tpm 信封是否存在。
    pub encrypted: bool,
    /// 封装方案，本期只有 "tpm-pcp"（弱软件版后置）；信封损坏时为 None。
    pub scheme: Option<String>,
    /// 能否打开 PCP 提供程序（§5 解锁前的芯片健康 ping）。
    pub tpm_available: bool,
}

mod engine {
    //! dir-based 纯引擎：所有 NCrypt FFI + 信封 IO 都在此，便于单测（不依赖 Tauri runtime）。
    use super::*;
    use sha2::{Digest, Sha256};
    use windows::core::{w, PCWSTR};
    use windows::Win32::Security::Cryptography::*;
    use zeroize::Zeroize;

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

    // 关注的 NCrypt HRESULT（数值比较，避免 import 不确定性）。
    const NTE_BAD_DATA: u32 = 0x8009_0005; // OAEP 解包失败（口令对但密文坏）→ Corrupt
    const NTE_PERM: u32 = 0x8009_0010; // 使用授权失败（错口令/锁定）→ WrongPassword
    const NTE_NOT_FOUND: u32 = 0x8009_0011; // 钥匙不存在 → 幂等删除 / ChipUnavailable
    const NTE_BAD_KEYSET: u32 = 0x8009_0016; // keyset 缺失 → ChipUnavailable

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
            NTE_PERM => FailClass::WrongPassword, // 注：DA 锁定也可能落此/类似码；Phase 3 UI 持 live code 细化
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

    /// 启动/操作前自愈（DA-free，绝不靠猜口令）：
    /// - `heng.dek.tpm.new` 存在 ⇒ 改密在 commit(rename) 前中断 → **回滚**：删该 staging slot 的钥匙 + 删 .new 文件。
    ///   （此时 heng.dek.tpm 仍是旧信封、旧口令可解；改密视为未发生，UI 应提示用旧口令。）
    /// - 顺带清掉 commit 后遗留的旧 slot 孤儿钥匙（rename 已成、.new 不在，但删旧 slot 未成功的情况）。
    fn reconcile(dir: &Path) {
        let newp = dir.join(ENVELOPE_NEW);
        if newp.exists() {
            if let Ok(env) = read_envelope(&newp) {
                if let (Some(slot), Ok(prov)) = (slot_from(&env.slot), open_provider()) {
                    let _ = delete_slot_key(&prov, slot);
                }
            }
            let _ = std::fs::remove_file(&newp);
        }
        if let Ok(env) = read_envelope(&dir.join(ENVELOPE)) {
            if let (Some(live), Ok(prov)) = (slot_from(&env.slot), open_provider()) {
                let _ = delete_slot_key(&prov, other_slot(live)); // 删非 live 的 slot（孤儿）
            }
        }
    }

    // ---- 引擎操作（命令薄包装它们；测试直接调） ----

    pub(super) fn status(dir: &Path) -> SecurityStatus {
        let envelope = dir.join(ENVELOPE);
        let (encrypted, scheme) = match read_envelope(&envelope) {
            Ok(env) => (true, Some(env.scheme)),
            Err(_) if envelope.exists() => (true, None), // 文件在但坏 → 仍算已加密（损坏态）
            Err(_) => (false, None),
        };
        SecurityStatus {
            encrypted,
            scheme,
            tpm_available: open_provider().is_ok(),
        }
    }

    /// 首次设密码：随机 DEK → slot A 建钥匙(口令) → 封 DEK → 原子写信封。返回 DEK（命令存 Rust 侧）。
    /// 注意：本阶段**不**迁移 heng.db（Phase 4）；调用前应确保库尚明文。
    pub(super) fn set_password(dir: &Path, pw: &str) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
        if dir.join(ENVELOPE).exists() {
            return Err(internal("already encrypted".into()));
        }
        let prov = open_provider()?;
        let dek = gen_dek()?;
        let key = create_slot_key(&prov, Slot::A, pw)?;
        let ct = wrap_dek(&key, &dek)?;
        let env = WrapEnvelope::new(Slot::A, &ct);
        let tmp = dir.join(format!("{ENVELOPE}.tmp"));
        write_sync(&tmp, &env)?;
        std::fs::rename(&tmp, dir.join(ENVELOPE)).map_err(internal_io)?;
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
        std::fs::rename(&newp, dir.join(ENVELOPE)).map_err(internal_io)?; // commit
        let _ = delete_slot_key(&prov, live); // 失败不致命，reconcile 兜底
        Ok(())
    }

    /// 移除密码：验证口令（解出 DEK）后拆封装。
    /// 本阶段仅删信封 + slot 钥匙；Phase 4 在此用 DEK 把密文库解密回明文（§9 反向迁移）。
    /// 删序：**先删信封**（删后即读作未加密），再删钥匙 —— 避免“信封在但钥匙没了＝打不开”的死局。
    pub(super) fn remove_password(dir: &Path, pw: &str) -> Result<(), CryptoError> {
        reconcile(dir);
        let env = read_envelope(&dir.join(ENVELOPE))?;
        let slot = slot_from(&env.slot).ok_or_else(|| corrupt("unknown slot"))?;
        let ct = decode_hex(&env.wrapped_dek_hex).ok_or_else(|| corrupt("bad ciphertext hex"))?;
        let prov = open_provider()?;
        let _dek = {
            let k = open_slot_key(&prov, slot, pw)?;
            unwrap_dek(&k, &ct)? // 验证口令；错则 WrongPassword
        };
        std::fs::remove_file(dir.join(ENVELOPE)).map_err(internal_io)?;
        let _ = delete_slot_key(&prov, slot);
        let _ = delete_slot_key(&prov, other_slot(slot));
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

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
            use crate::db::open_db;
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

fn dir_err(m: String) -> CryptoError {
    CryptoError {
        class: FailClass::Internal,
        code: 0,
        message: m,
    }
}

#[tauri::command]
pub fn security_status(app: AppHandle, crypto: State<Crypto>) -> Result<SecurityStatus, String> {
    let dir = config_dir(&app)?;
    let _op = crypto.0.lock().unwrap(); // 串行化（不与改密的 rename 撞读）
    Ok(engine::status(&dir))
}

#[tauri::command]
pub fn set_password(app: AppHandle, crypto: State<Crypto>, password: String) -> Result<(), CryptoError> {
    let dir = config_dir(&app).map_err(dir_err)?;
    let mut dek_slot = crypto.0.lock().unwrap(); // 持锁全程：串行化 + 存 DEK
    let dek = engine::set_password(&dir, &password)?;
    *dek_slot = Some(dek);
    // Phase 4：在此把明文 heng.db 原子迁移为 SQLCipher 密文库（§9）。本阶段仅建立封装。
    Ok(())
}

#[tauri::command]
pub fn unlock(app: AppHandle, crypto: State<Crypto>, password: String) -> Result<(), CryptoError> {
    let dir = config_dir(&app).map_err(dir_err)?;
    let mut dek_slot = crypto.0.lock().unwrap();
    let dek = engine::unlock(&dir, &password)?;
    *dek_slot = Some(dek);
    // Phase 3：bootstrap 在解锁成功后取该 DEK 开 SQLCipher 库（db_open 带 key）。
    Ok(())
}

#[tauri::command]
pub fn change_password(
    app: AppHandle,
    crypto: State<Crypto>,
    old_password: String,
    new_password: String,
) -> Result<(), CryptoError> {
    let dir = config_dir(&app).map_err(dir_err)?;
    let _op = crypto.0.lock().unwrap(); // 持锁全程：改密的两阶段原子序列必须独占
    engine::change_password(&dir, &old_password, &new_password)
}

#[tauri::command]
pub fn remove_password(app: AppHandle, crypto: State<Crypto>, password: String) -> Result<(), CryptoError> {
    let dir = config_dir(&app).map_err(dir_err)?;
    let mut dek_slot = crypto.0.lock().unwrap();
    engine::remove_password(&dir, &password)?;
    *dek_slot = None; // 清掉已解锁 DEK
    Ok(())
}
