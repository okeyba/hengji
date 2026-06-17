# 衡记 · 应用级密码与本地加密 — 实现说明（as-built）

> **English version: [`en/encryption.md`](en/encryption.md).**
>
> 状态：**已实现并落地（分支 `feat/local-encryption`，仅本地提交、未发版）。** 本文描述**实际做出来的东西**，不是提案。设计演变的来龙去脉见文末「附录 · 设计演变」与配套文档：
> - [`spike-results.md`](spike-results.md) —— 动手前四项技术验证（Spike）的实测结论。
> - [`soft-path-plan.md`](soft-path-plan.md) —— 分阶段实现契约与落地记录。
>
> 范围：**仅 Windows 桌面端（Tauri）**。浏览器演示版是纯内存（`InMemoryRepository`）、无磁盘文件，**不涉及加密**。`packages/core` 复式引擎、业务逻辑、SQL 形状**全程未动**。

---

## 0. 一句话讲清（给非技术读者）

- 你的账本被一把**随机钥匙（DEK）**锁住；这把钥匙再被**这台电脑的安全芯片（TPM）**收着——要**在这台电脑上 + 输对密码**才能放出来。
- **把数据库文件拷到别的电脑：解不开。** 钥匙绑死这台芯片、拿不出来，也没有「光靠密码就能算出钥匙」的路。这一档是真的，比纯软件加密强。
- **删除是你主动按的，不是「错 N 次自动销毁」。** 设置里有「清空全部数据」按钮：加密状态下要输对当前密码才执行，明文状态下二次确认即可。一旦执行，**永久、不可恢复**。
- **诚实的差距**：电脑的 TPM **不是** iPhone 的安全岛——它没有「给我这把钥匙单独数到 5 次错就自毁」的硬件能力。它的防猜测是**全局限速**（连错几次后芯片临时拒绝再试，过一阵才恢复），由系统/芯片策略管，我们的软件改不了。所以真正的护城河是**「加密本身 + 强密码 + 芯片限速」**，不是某个销毁动作。
- **代价（已确认接受）**：本地**没有后门**。忘记密码 / 清掉 TPM / 换主板或 CPU（fTPM 绑 CPU）/ 重装系统失效 = 数据**彻底找不回**。这是这套强度的代价。**请务必自己留备份**（设置里可导出一份明文备份，见 §7）。

---

## 1. 威胁模型与诚实边界

**威胁模型 ＝ A「设备级隐私」**：防捡到 / 借用 / 共用这台电脑的人翻看你的账。

### 这套能给的（真）
- **拷文件无法离线爆破**：DEK 是随机数、被安全芯片非导出钥匙封住、拿不出芯片；芯片对每次「用密码解封」的尝试有**全局防猜测限速（DA，Dictionary Attack lockout）**。这是比纯软件强的核心一档。

### 这套给不了 / 不防（UI 与本文都讲清，不夸大）
- **电脑 TPM ≠ iPhone 安全岛**：没有硬件自治的「错 N 次毁钥匙」。本实现**不做**软件层的「错 N 次自动销毁」（原因见 §5）。
- **TPM 防猜测是全局的**：和 BitLocker / Windows Hello 共用同一套 DA 计数，会互相干扰；阈值与恢复时间是芯片/系统策略，app 改不了。密码输错可能连带触发系统级锁定。
- **本机实测：fTPM 的 DA 很快就锁**（见 §4「DA 抢占」）——连错约 3 次，芯片就临时拒绝再用这把钥匙（返回 `TPM_20_E_LOCKOUT`），靠时间慢慢恢复。**芯片限速是主防护**，这是好事（爆破被卡死），但也意味着「在 app 内瞎试密码」很快就被芯片自己顶回去。
- **物理接触者可「清 TPM」删库**：进 UEFI 或管理员可 Clear TPM → 封装钥匙没了 = 数据永久不可解。**不需要密码、绕过一切确认**。这是物理对手最省事的破坏路径（破坏性，非窃取）。
- **总线嗅探 / 故障注入**：独立 TPM（dTPM）的 CPU↔芯片总线若无加密会话，物理接触者理论上可嗅探解封数据（BitLocker 有实证）；固件 TPM（fTPM）有 faulTPM（电压注入）/ TPM-Fail（时序）等公开攻击。**本实现经 NCrypt PCP，拿不到「加密+HMAC 的 TPM 会话」原语**（Spike 已证 NCrypt 不暴露此能力），故此项**未缓解**；定位为「本机 fTPM 残余嗅探风险低，接受、不上裸 TBS」。
- **解锁运行期**：解出的明文 DEK 在进程内存；OS 可能换页到 swap / 休眠写 `hiberfil.sys`。缓解靠 OS 整盘加密（BitLocker）。
- **设密码前的历史明文 / 迁移残留**：SSD 的 TRIM / wear-leveling / CoW 使「覆盖删除」不保证物理擦除。
- **运行时内存取证 / 恶意软件 / 国家级对手**：能力之外。

**诚实上限**：定位是「**对拿到这台电脑的普通人很难**」，**不是**「绝对不可恢复」。

---

## 2. opt-in 与状态行

- **加密 opt-in，默认明文**：沿用现状，不强迫随手记账的个体户加门。
- **状态行（设置页「安全 · 本地加密」卡常驻）**：
  - `未加密（明文）`
  - `已加密 · 安全芯片保护（强）`（本期唯一的「已加密」形态：方案 `tpm-pcp`）
  - 信封损坏 → 仍显示「已加密」但标注异常态（见 §3 `scheme=None`）。
- **无芯片 / 纯软件弱版：本期未实现（后置）。** 当前加密路径**要求一块可用的 TPM**；打不开「Microsoft Platform Crypto Provider」时，设置密码会失败并报「芯片暂不可用」，而**不会**静默退化成弱加密。原设计里「纯软件较弱版 + 第三态状态行」留作未来增强。
- `core` / 业务逻辑 / SQL：**未动**。

---

## 3. 密钥架构（随机 DEK + 安全芯片绑定封装）

实现于 `apps/desktop/src-tauri/src/crypto.rs`（密钥层）+ `db.rs`（SQLCipher 开库）。

- **DEK（数据钥匙）**：随机 256-bit（`BCryptGenRandom`），**真正加密数据库**。不是从密码算出来的——这是「拷文件无法离线爆破」的根。
- **SQLCipher raw-key 开库**：`PRAGMA key = "x'<64位hex>'"`（raw key，跳过 SQLCipher 自己的 PBKDF2；**必为开库第一条 PRAGMA**），随后 `journal_mode=WAL` / `foreign_keys=ON` / `busy_timeout`，再跑迁移。
- **安全芯片封装**：NCrypt + **Microsoft Platform Crypto Provider（PCP）** 在 TPM 里建一把**非导出** RSA-2048 钥匙；密码作为这把钥匙的**使用授权 `PCP_USAGEAUTH`**（值＝`SHA256(UTF-16LE(密码))`）。封装算法 **OAEP-SHA256**。解封 DEK 必须 ① 在本机芯片上 ② 输对密码（Spike #2 实测：错密码被芯片 `NTE_PERM` 拒绝并计入全局 DA）。
- **两个固定 slot（`heng-dek-wrap-a` / `-b`）**：改密时 ping-pong——新钥匙建在另一个 slot，验证 + 提交后才删旧（见 §6）。
- **信封文件 `heng.dek.tpm`**（库同目录，JSON）：`{ version, scheme:"tpm-pcp", alg:"rsa2048-oaep-sha256", slot:"a"|"b", created_unix, wrapped_dek_hex }`。
- **DEK 绝不跨 IPC 回传 JS**：解出的 DEK 只存在 Rust 侧 `Crypto` state（`Zeroizing`，替换/drop 时清零）；JS 侧只发命令（`set_password` / `unlock` / `db_open(encrypted)` …），开库时 Rust 内部直接拿 state 里的 DEK 当 raw key。
- **命令全程串行化**：5 个 crypto 命令各自全程持 `Crypto` 互斥锁——改密 / 解锁 / 移除 / 清空都在固定 slot + 单一信封上做多步非原子操作，并发会互踩（如改密 commit 与另一路 reconcile 删 slot 撞车 = 库永久不可解）。

> 诚实边界：PCP 给的是「非导出 key + 密码授权 + 全局 DA 限速」；**拿不到 / 改不了** TPM 的精确失败计数与阈值（那是系统级策略）。所以本实现**不依赖**读芯片硬件计数。

---

## 4. 失败分流与「DA 抢占」

解锁 / 封装失败按 `FailClass` 分流（`crypto.rs`），UI 据此分屏：

| 分类 | 触发（HRESULT） | 含义 / UI |
| --- | --- | --- |
| `WrongPassword` | `NTE_PERM 0x80090010` | 密码错（芯片拒绝使用授权）。**消耗 1 次 DA strike。** |
| `Locked` | `TPM_20_E_LOCKOUT 0x80280921` / `TPM_E_DEFEND_LOCK_RUNNING 0x80280210` | 连错过多触发芯片 DA 防爆破锁定。**不判定密码对错、不计销毁**；等冷却或正确密码复位（reboot 不一定解，DA 计数靠时间递减）。UI 显「芯片临时锁定」+ 原始 HRESULT。 |
| `Corrupt` | `NTE_BAD_DATA 0x80090005` / 信封解析失败 | 信封或密文损坏 → 「数据可能损坏」。 |
| `ChipUnavailable` | `NTE_NOT_FOUND` / `NTE_BAD_KEYSET` / 句柄异常 / 其它 | 芯片占用、钥匙缺失、TBS 异常 → 「芯片暂不可用，请重启」。**不计销毁。** |
| `Internal` | 目录解析 / IO / 序列化 | 基建错（非解锁三态）。 |

**DA 抢占（本机实测，诚实写进文档）**：本机是 **fTPM**，DA 阈值很低——**连错约 3 次** 就从 `WrongPassword` 翻成 `Locked`（芯片返回 `TPM_20_E_LOCKOUT`），之后每隔约 2 小时恢复 1 次尝试额度。后果：

- **「在 app 内瞎试密码」很快被芯片自己顶死**——这正是我们要的（爆破被硬件限速卡住）。
- 这也是当初设计的「软件层错 5 次自动销毁」**在本机根本触发不了**的原因：计数还没数到 5，芯片已经 `Locked` 了（且 `Locked` 按设计不计数）。芯片限速抢在前面 = 更安全，但让「软件自管销毁」变得又慢又不确定。**这是 §5 删掉自动销毁的实测依据之一。**
- **测试纪律**：每次错密码耗 1 DA。**别为了试而反复输错密码。** TPM 集成测试里只有**唯一一次**蓄意错密码（`tpm_wrong_password_one_strike`，消耗恰好 1 strike），其余全用正确密码（0 DA）。

> `NTE_NOT_FOUND` / `NTE_BAD_KEYSET` 现粗归 `ChipUnavailable`，但它也可能是 **Clear TPM / 换主板 / 把信封拷到他机** = 钥匙**永久没了**（库不可解）。UI 保留了原始 code，未来可把「永久丢失」从「暂不可用·重启」里细分出来给专门提示。

---

## 5. 删除＝用户主动「清空数据」（**不做**错 N 次自动销毁）

> **重大决策（2026-06-16，用户拍板）：删掉「错 N 次自动销毁」，改成用户主动「清空全部数据」。** 原设计里整套「软件自管失败计数 → 到 N 次删钥匙 + 隔离区后悔药 + 销毁终态屏 + sentinel」**已全部撤除**，仅作历史留在附录。

**为什么删**（用户理由）：
1. **锁定（防别人解密）该归芯片管**——TPM 的全局 DA 限速本就是干这个的，且本机 fTPM ~3 次即锁（见 §4），软件再叠一层「数到 5」既多余又触发不到。
2. **删除应当是用户主动行为**，不是系统替你「手滑误删」。自动销毁带来误删 / 反噬风险（把「删库」递给能物理碰电脑、故意输错的人），收益却被芯片限速抢占。
3. 软件触发的销毁本就弱：对手在第 N 次前拔电 / 杀进程 / 拷走信封就能躲过那一次。真护城河是「加密 + 强密码 + 芯片限速」，不是销毁动作。

**改成什么（实现态）**——`wipe_data` 命令 + `engine::wipe`（`crypto.rs`）：

- **设置页「安全」卡 → 「清空全部数据…」按钮**：
  - **加密态**：必须**输对当前密码**（命令层先 `unlock` 验证密码，解封成功才算对；错则原样返回 `WrongPassword`/`Locked`/… **绝不删**）+ 二次确认。
  - **明文态**：无密码，仅二次确认。
- **执行＝直接永久删**（无后悔药、无隔离区）：删 TPM 两个 slot 钥匙（加密库即刻不可解）→ 删信封 `heng.dek.tpm` / staging `.new` / 迁移标记 `heng.migrate` / 预解锁状态 `heng.security` → 删 `heng.db` 及 `-wal/-shm` 边车 → 清掉任何历史遗留的 `heng.destroyed*` 隔离残留（保险，正常没有）。
- **删 slot 的容错**：TPM 偶发占用 → 小重试 3 次；仍删不掉就放过——剩下的是**良性孤儿钥匙**（它保护的库已删、解不开任何东西；下次设密码 `create_slot_key(OVERWRITE)` + reconcile 孤儿清理会收掉它）。
- **之后**：JS 侧 `handleWiped` 清空内存 → `resetDesktopRepo` → 开一个**全新空明文库**（一个默认账本）→ 回总表。从干净的初始态重新开始。

两个明确取舍（用户拍板）：① **直接永久删**，无后悔药；② **明文态下按钮也可用**（不是加密专属）。

---

## 6. 解锁 / 设密码 / 改密码 / 移除 UX

实现：`apps/web/src/db.ts`（bootstrap 门）+ `App.tsx`（启动状态机 + 自动锁）+ `UnlockScreen` + `SecurityCard`。

- **bootstrap 门**：桌面启动先 `security_status()`（内部先 `reconcile` 自愈任何中断的迁移/改密）→ 已加密则**先渲染解锁屏**（repo ready 之前），解出 DEK 后才 `db_open(encrypted)`；未加密则照常开明文库。
- **解锁屏**：密码框；失败按 §4 分屏（密码错 / 锁定 / 损坏 / 芯片不可用），锁定态显原始 HRESULT。密码由用户在原生输入框输入，助手不代填。
- **设置「安全」卡**：状态行（§2）+ 设 / 改 / 移除密码 + 导出明文备份（§7）+ 清空数据（§5）+ 自动锁开关。
- **自动锁**：默认 **15 分钟**无操作 → `lock` 命令清 DEK + 关库 → 回解锁屏。设置里可开关 / 调时长（`autoLockMinOf`）。
- **改密码 = 重封 under 新钥匙的两阶段原子协议**（Spike 定 `PCP_CHANGEPASSWORD` 不可用，会返回 `NTE_INVALID_PARAMETER` 且有 DA 成本）：
  1. 旧密码解出 DEK（验旧密码）→ 2. 另一 slot 用新密码封同一把 DEK → 3. 写 staging `heng.dek.tpm.new`（+fsync）→ 4. 从 staging 用新密码验证可解且 ==DEK → 5. 原子 rename `.new`→信封（**commit 点**）→ 6. 删旧 slot 钥匙（失败不致命，reconcile 兜底）。
  - **不动数据库密文**（只换封 DEK 的钥匙），秒级。
  - **启动自愈认旧/新两个封装文件**：commit 前崩溃（`.new` 在、未 rename）→ reconcile 删 staging slot + `.new`，旧密码仍解出原 DEK；杜绝「改密断电后新旧都解不出 = 静默锁死」。
- **移除密码 = 密文→明文反向迁移**（见 §8）：验密码 → 迁移库回明文 → 删信封 + 两 slot。后果文案讲清「移除＝数据库变回明文，任何拿到文件的人都能直接打开」。

---

## 7. 备份与找回

### 找回 —— 本地无后门（用户拍板：忘密 = 彻底没了）
- **铁律**：本地**不留任何「光靠密码就能解」的后门**。后果完整枚举：**忘密 / 清 TPM / 重装系统 / 换主板或 CPU（fTPM 绑 CPU）/ TPM 固件升级失效 = 本地数据彻底找不回。**
- **不做恢复码**（本质是离线可用的后门，与「真硬」冲突）。
- **真正的找回靠未来端到端云同步**（§9，本期后置；注意其单设备限制）。

### 明文备份导出（阶段 4a，已实现）
`export_backup` 命令 + `engine::export_backup`：
- 走原生「另存为」对话框（`tauri-plugin-dialog`）选路径；**实际写文件仍在 Rust 内完成**（不引 tauri-plugin-fs）。
- 统一 `sqlcipher_export`（不分明/密两套）：写 `.tmp` → export + 补传 `PRAGMA user_version`（sqlcipher_export 不复制它）+ `integrity_check` + 逐表行数校验 → DETACH + close → 无 WAL 边车 → 同卷原子 rename 到目标。加密库须用已解锁 DEK 解出明文写出；明文库直接导。
- **备份恒明文**——明标这是「**关闭加密的等价物、不受密码保护**」，请移到离线介质、别和电脑放一起。
- **路径防撞**：拒绝导到应用数据目录里的 `heng.*` 控制文件（防覆盖活动库/信封）。
- **新鲜度感知**：成功后把 `last_backup_unix/path` 写进 `heng.security`（锁定态可读的明文状态文件）；设置卡显示「上次备份 N 天前」、超期软提醒。

---

## 8. 明↔密库原子迁移（§9 协议，已实现）

实现于 `crypto.rs::engine`（`migrate_encrypt` / `migrate_decrypt` / `reconcile`）。**整个迁移在 Rust 内完成**（Rust `std::fs::rename` 同卷即原子替换）。

**迁移协议**（明→密，反向同理）：
1. 关闭 State 连接（heng.db 文件不被占用）→ 清残留 tmp + 边车。
2. 开源库 → `wal_checkpoint(TRUNCATE)` 折叠 WAL → ATTACH 目标 tmp（密文用 raw key / 明文用 `KEY ''`）。
3. `sqlcipher_export` 整库导出 + **补传 user_version** + `integrity_check` + **逐表行数与源库一致校验**（§9「逐行一致」的低成本等价；全行 hash 比对留 Phase 4 硬化）。
4. DETACH → 显式 `close`（同步，避免 Windows 延迟释放句柄导致随后 rename `ACCESS_DENIED`）。
5. **原子 rename tmp → heng.db（提交点）** → 清明文边车。失败前回滚（删 tmp）。

**`rename_with_retry`**：Windows 下刚关闭的库文件可能被 AV/索引器/close-pending 短暂持锁，rename 报 `ACCESS_DENIED (os error 5)`——Spike #2 probe2 定为**可重试**：小退避重试 12 次再放弃。

**启动自愈 `reconcile`（DA-free，绝不靠猜密码）**，三段按依赖序：
1. **迁移标记 `heng.migrate`**（含方向 + 信封）：据**库文件头**（明文 `SQLite format 3\0` vs 随机密文）判定在原子 rename 提交点哪一侧 → 前滚（提交信封）/ 回滚（删未提交信封 + tmp + staging slot）。标记损坏时做**方向无关**自愈（明文库 ⇒ 删过时信封/slot；密文库 ⇒ 不动）。slot 没删净则保留标记作下次重试锚点。
2. **改密 staging `heng.dek.tpm.new`**（commit 前中断）：删 staging slot + `.new`（旧密码仍可解）。
3. **孤儿 slot**：清掉 live 信封的非 live slot 残留钥匙。

`set_password` / `remove_password` 命令全程持 **Crypto→Db 两把锁**（固定加锁序），关连接 → 迁移 → 重开。`security_status` 先 `reconcile` 再判定（启动门自愈）。

---

## 9. 未来端到端云同步（本期后置，留接口）

- **目标（像 WhatsApp）**：数据在你设备上加密、服务器只存乱码、永远看不到明文；复用同一把 DEK。
- **跨设备密钥分发 = 每设备密钥包裹 + 新设备走已有端授权**（可 per-device 撤销，避免把 E2E 安全坍缩到密码强度）。
- **⚠️ 单设备用户的死锁**：目标用户多为单台电脑。每设备包裹意味着云端那份只被这台设备的密钥包裹；这台坏了 / 清了芯片，**没有「另一台已存活设备」来授权新设备 → 云端 blob 也永远解不开**。即「云找回」在最该兜底的场景（唯一设备损坏）失效。
- **取舍（云同步阶段再拍板）**：要让单设备用户也能云找回，必须引入一把**不绑设备的恢复因子**（密码派生 KEK 或恢复码包裹 DEK 存云）——但这又把强度降回密码爆破。「对别人很难恢复」vs「单设备用户对自己能找回」不可兼得。**本期诚实告知：现在没有任何云兜底。**
- 与本期衔接：DEK 已是「随机、芯片绑定」的独立钥匙，将来加云时向云端做「每设备包裹」即可，不必重构本地。

---

## 10. 分层改动 + 构建前提

| 层 | 改动 |
| --- | --- |
| **Rust 新核心** | `crypto.rs`：NCrypt PCP 建非导出封装钥匙、密码作授权、封装/解封 DEK、改密重封、明↔密迁移、`reconcile` 自愈、`wipe` 清空、`export_backup` 备份、`NCryptDeleteKey` 删钥匙。`db.rs`：rusqlite + SQLCipher raw-key 开库、单连接 `Mutex<Connection>`、`db_batch` 事务。 |
| **store** | tauri-plugin-sql 整体替换为自写 rusqlite+SQLCipher 桥（`$N→?N`、column_name→JSON、`PRAGMA key` first）。受同一 `Repository` 契约约束、SQL 形状零改。`@app/store/crypto` 暴露命令 + `FailClass`。 |
| **app 启动** | bootstrap 前插解锁/设密码门 + 解锁屏（四类失败分屏）+ 自动锁。 |
| **设置** | 「安全」卡：状态行 + 设/改/移除密码 + 备份导出（新鲜度）+ 清空数据 + 自动锁。 |
| **平台** | **仅 Windows**（TPM 2.0）。无芯片软件弱版后置（§2）。Mac（安全岛）/ Linux 后续。 |

### 构建前提（Windows）
SQLCipher 经 `rusqlite` 的 `bundled-sqlcipher-vendored-openssl` feature 编 C 源 + vendored OpenSSL（libcrypto 做 AES/HMAC）。本机构建需：
- **MSVC 构建环境**（`vcvars64.bat`，即 VS Build Tools「使用 C++ 的桌面开发」）。
- **Strawberry Perl**（vendored OpenSSL 的配置脚本需要 perl；**必须 Strawberry Perl，不是 msys perl**）。
- **不需要 nasm**（Spike #1 已证）。
- 首次会编一次 OpenSSL（~8 分钟，一次性）。`tauri dev` 用 `--no-default-features`、与 `cargo test` 缓存键不同，故首次各编一次。

详见 [开发者手册 · 前置环境](../development.md)。

---

## 11. 测试

`cargo test`（16 passed / 4 ignored；须 vcvars + Strawberry Perl 环境）：

- **纯逻辑 always-run（0 DA，无 TPM）**：UTF-16LE 摘要钉死（含空串/多码元/无尾 NUL）、hex roundtrip、信封 serde、`classify` 映射、slot 助手、迁移标记 serde、库头探测。
- **SQLCipher / 迁移 / 备份 / 清空 自动跑（0 DA，纯 DEK 无 TPM）**：raw-key 真加密 roundtrip（头非明文 + 错 key fail-fast）、明→密→明迁移保数据 + user_version + 库头翻转、备份导出（明/密两路 + 路径防撞）、`wipe` 删全部本地数据。
- **真 TPM 集成测试 `#[ignore]`（手动 `cargo test -- --ignored --test-threads=1`，固定 slot 须串行）**：① 设→解→改→解→移除〔0 DA〕② 改密 commit 前崩溃回滚自愈〔0 DA〕③ 加密测试库锁→解→读回〔0 DA〕④ **唯一一次蓄意错密码〔消耗恰好 1 DA strike〕**。Phase 2 已实测绿。

---

## 附录 · 设计演变（落地中改了原计划，留作历史）

原设计稿（`encryption.md` v4）经三轮红队打磨，落地时有四处重大演变：

1. **硬路 NO-GO → 软路**：原想用 TPM **NV 单调计数器 + PolicyNV（「计数<N」）** 让芯片自己拒绝解封（逼近 iPhone 式确定性）。Spike #3 实测：非提权 `TPM2_NV_DefineSpace` 被 Windows TBS 命令过滤器拦截（`TPM_E_COMMAND_BLOCKED 0x80280400`，与 owner-auth 是否为空无关）→ 退**软路**（DEK 由 PCP 钥匙 + 密码授权封装，§3）。
2. **加密不用「密码 KDF 派生密钥」**：改**随机 DEK 真加密 + DEK 由 TPM 非导出钥匙封装、密码作 PCP_USAGEAUTH**——拷文件无法离线爆破，比 KDF 强（KDF 路线下拷文件可离线对密码爆破）。
3. **§9 明↔密原子迁移并入「解锁/设密码」阶段**（原计划在更后的阶段）：因端到端「设密码」不可能脱离迁移成立（否则设密码后留下「明文库 + 信封」损坏态）。
4. **「错 N 次自动销毁」被否决删除**（见 §5）：原设计有整套「软件自管计数 + 隔离区后悔药 + 销毁终态 sentinel 屏 + 三类失败分流不计销毁」。本机 fTPM DA ~3 次即锁（< N=5）、`Locked` 不计数 → 自动销毁慢且被芯片抢占，且带误删/反噬风险 → 改**用户主动「清空数据」**（§5）。**锁定归芯片、删除归用户主动。**

> v4 红队的有效部分（仍保留在实现里）：opt-in 默认明文 + 状态行、威胁模型诚实披露（swap/hiberfil/SSD 残留/Clear TPM 删库 DoS/总线嗅探）、解锁失败「损坏 vs 密码错」分流、迁移三段式原子 + 启动自愈、撤销「未加密备份」推荐改「关闭加密等价物」明示 + 新鲜度感知、改密独立 TPM 对象切换原子协议、单设备云找回死锁摊开。
