# 架构与关键决策

## 愿景
开源、本地优先、跨平台的个人财务应用；底层复式记账严谨、上层单式体验友好；覆盖个人开支 / 小生意 / 投资；open-core（本地免费开源 + 可选付费云同步）。

## 分层（严格解耦，依赖单向向下）
1. **core（`@app/core`）** — 纯 TS、零 I/O 的复式记账引擎：领域类型、单式→复式展开、借贷平衡校验、余额调整、报表、预算计算、默认科目表。
2. **store（`@app/store`）** — 持久层抽象 `Repository`；内存实现 + SQLite 实现；同步元数据预留。
3. **shell（计划）** — 桌面 / 移动 / Web 外壳 + UI；只依赖 `Repository` 接口与 core。

> UI 不直接碰数据库；core 不依赖 store；换平台只换 shell + 注入对应 `Repository`。

## 关键决策
- **复式内核 + 单式 UX**：用户记「支出 ¥30·餐饮·招行卡」，core 自动展开平衡分录。
- **金额 = 整数最小单位（分）**：杜绝浮点误差；非整数直接拒绝。
- **beancount 有符号约定**：每笔交易 postings 求和恒为 0；账户余额 = 其 postings 之和。
- **纯函数 core**：`genId` / 时钟由外部注入，无环境依赖、测试确定。
- **`Repository` 接口先行**：上层只依赖接口；内存 / SQLite /（未来）Tauri 实现可替换。
- **同步元数据**：每条记录带 `createdAt / updatedAt / deleted`；软删除而非物理删除。
- **投资极简档**：`adjustBalanceEntry` 把现值差额记入对方科目（投资盈亏 / 期初余额），保持平衡；不做持仓 / 价格源 / 逐笔损益（留 v0.3）。

## 平台与 SQLite 路线（已核实）
- **Shell：Tauri 2，桌面优先**（稳定）；移动随后（移动端 SQLite 上线前需真机 spike 验证）。一套 React/TS 前端桌面 + 移动复用。
- **SQLite 驱动按运行环境分，均藏在 `Repository` 之后**：
  - Node / 测试：`node:sqlite`（Node 24 内置、零原生编译）
  - 生产桌面：`tauri-plugin-sql`（sqlx）
  - 浏览器 / PWA：wa-sqlite（OPFS）
  - SQL schema / 查询基本可平移。
- 工具链：Node 24、pnpm 9、Vitest 3、TypeScript strict。

## open-core 与许可证
- 本项目 core：**Apache-2.0**（含专利授权）+ 贡献者 **DCO**，保留闭源付费云同步层与将来防御性 relicense 的空间。
- 参考项目 license（借鉴前务必核对）：**Actual Budget = MIT（可借代码）**；Maybe / Firefly III / Ghostfolio = **AGPL（仅可学习、不可抄，否则危及闭源云层）**；beancount / GnuCash = GPL（且非 TS 栈）。

## 同步（未来 v1.0）
- 本地优先；可选付费云同步。
- 候选：**PowerSync**（服务端可在事务里强制借贷平衡）/ **Actual 式 HLC + 每字段 LWW 消息日志**（MIT、领域契合、可 E2E 加密、可自托管）。
- **关键纪律**：复式有「分录求和 = 0」的跨行不变式，通用 CRDT 只保证收敛、不保证平衡 → 每笔交易当**不可变原子单元**同步、合并时重校验。当前模型已是整笔存取、不就地改单条 posting，天然合规。

## 测试
- core：test-first，纯逻辑单测。
- store：**共享 `Repository` 契约套件**（`packages/store/test/contract.ts`），内存与 SQLite 跑同一套以保证行为一致；另含 SQLite 文件持久化测试。
