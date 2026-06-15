[← 返回手册目录](README.md)

# 开发者：构建 · 架构 · 贡献

想读懂衡记的代码、在本机跑起来、或者 fork 一份改成自己的账本，这篇带你上手。

**适用**：开发者 / 想 fork 的人 / 想给项目提 PR 的贡献者。

衡记表面是「记一笔」，底层是一套严谨的复式记账引擎。它是开源、本地优先的桌面应用——数据 100% 存本机 SQLite，不联网、不上传。当前版本 v0.2.0（仅 Windows x64、未签名）。这篇讲它是怎么搭起来的。

---

## 一、三层 monorepo 架构

代码是一个 pnpm monorepo，分三层，**依赖严格单向向下**：UI 依赖持久层接口和内核，内核谁都不依赖。

| 包 | 名字 | 职责 |
| --- | --- | --- |
| `packages/core` | `@app/core` | 纯 TS、零 I/O 的复式记账引擎：领域类型、单式→复式展开、借贷平衡校验、报表、预算、默认科目表。 |
| `packages/store` | `@app/store` | 持久层抽象 `Repository`：内存实现 + SQLite 实现（`node:sqlite`，经 `@app/store/sqlite` 子路径导出）+ Tauri 实现（`@app/store/tauri`）。 |
| `apps/web` | `@app/web` | UI（Vite + React 19），只依赖 `Repository` 接口与 core，不直接碰数据库。 |
| `apps/desktop` | `@app/desktop` | Tauri 2 桌面外壳，把 web 前端打包成本机应用，注入 SQLite 仓库。 |

一句话记住边界：**UI 不直接碰数据库；core 不依赖 store；换平台只换外壳 + 注入对应的 `Repository` 实现。** 详细分层与每个关键决策见 [`../ARCHITECTURE.md`](../ARCHITECTURE.md)。

---

## 二、前置环境

跑核心包和 web 端只要 Node 和 pnpm；跑桌面端（Tauri）才需要 Rust 工具链。

1. **Node ≥ 24** —— 内核用到了 Node 24 内置的 `node:sqlite`，零原生编译。
2. **pnpm ≥ 9** —— 仓库锁定 `pnpm@9.15.9`（见根 `package.json` 的 `packageManager`）。
3. **只有要跑/打包桌面端时**才额外需要：
   - **Rust** + **MSVC**（Windows 上的 C++ 构建工具，装 Visual Studio Build Tools 时勾选「使用 C++ 的桌面开发」）。
   - **WebView2** Runtime（Win11 一般自带）。

只想读引擎、跑测试、改 UI 的话，跳过第 3 条即可。

---

## 三、常用命令

在仓库根目录执行。

```sh
pnpm install                       # 安装全部 workspace 依赖

pnpm -r test                       # 跑所有包的测试（core 单测 + store 契约套件）
pnpm -r typecheck                  # 所有包做 TypeScript 类型检查

pnpm --filter @app/web dev         # 启动 UI（http://localhost:5173）
pnpm --filter @app/web build       # 构建 web 产物

pnpm --filter @app/desktop dev     # 跑桌面端开发模式（需 Rust 工具链）
pnpm --filter @app/desktop build   # 打包桌面安装包（tauri build）
```

> `pnpm -r test` 和 `pnpm -r typecheck` 在根 `package.json` 里也有同名快捷脚本（`pnpm test` / `pnpm typecheck`），二者等价。

---

## 四、几条必须知道的约定

改代码前先把这几条刻进脑子，否则很容易写出「编译过但账不平」的 bug。

### 金额一律整数最小单位（分）

钱只用整数表示，单位是「分」（人民币的最小单位）。**杜绝浮点误差**，非整数金额直接被拒绝。`¥30.00` 在代码里是 `3000`。不同币种的最小单位精度（小数位）由币种注册表配置——比如 BTC 是 8 位。

### beancount 有符号约定

借贷不另存「方向」字段，而是借正、贷负：

- **每笔交易所有分录（postings）求和恒为 0**——这就是借贷平衡。
- **某个账户的余额 = 它名下所有 postings 之和。**

core 里的 `assertBalanced` 是一道**安全防火墙**：分录求和不为 0 直接抛错，落不进库。多币种交易按币种分组分别求和=0（换汇/跨币转账例外，单独豁免）。

### 单式录入、复式自动生成

用户视角永远是「记一笔」——填「支出 ¥30 · 餐饮 · 招行卡」。背后 core 自动展开成一对平衡分录（借餐饮支出 / 贷招行卡），用户感知不到借贷。所有业务单据（订单、采购、收付款）也走同一条路：操作层产出候选分录 → core 校验平衡 → 入正规账本。**报表永远从分录聚合，不另立平行账。**

### core 是纯函数

`genId`、时钟这些有副作用的东西全部**由外部注入**，core 自身无环境依赖。所以 core 的测试是确定性的，也因此 core 能在任何平台原样复用。

---

## 五、Repository 契约测试

持久层的核心是 `Repository` 接口——上层只依赖这个接口，底下可以是内存、SQLite、或（桌面的）Tauri 实现，可替换。

为了保证多个实现行为完全一致，store 用一套**共享契约套件**：`packages/store/test/contract.ts`。内存实现和 SQLite 实现**跑同一套契约**，谁的行为偏了测试就红。

给 `Repository` 加新行为时，规矩是：

1. 先把行为写进 `packages/store/test/contract.ts`（共享契约）。
2. 让内存实现和 SQLite 实现**都过**。
3. `core` 的改动则 **test-first**——先写纯逻辑单测再实现。

SQLite 驱动按运行环境分，但都藏在 `Repository` 之后：Node / 测试用 `node:sqlite`，生产桌面用自写 rusqlite + SQLCipher 桥（替代 tauri-plugin-sql；当前明文，本地加密开发中），浏览器 / PWA 用 wa-sqlite（OPFS）。SQL schema 与查询基本可平移。

---

## 六、贡献流程与 DCO 签名

完整流程见 [`../CONTRIBUTING.md`](../CONTRIBUTING.md)，要点：

1. 改动**尽量小而聚焦**，匹配现有风格。
2. 提交前本地跑过 `pnpm -r test` 和 `pnpm -r typecheck`。
3. **每个提交都要 DCO 签名**——用 `-s`：

   ```sh
   git commit -s -m "你的提交说明"
   ```

   `-s` 会在提交信息末尾追加一行 `Signed-off-by: 你的名字 <邮箱>`，表示你有权提交这些代码、并同意以本项目许可证发布。本项目采用 Developer Certificate of Origin（DCO 1.1，全文见 <https://developercertificate.org>）。

### 许可证

core 是 **Apache-2.0**（含专利授权）。贡献即同意以 Apache-2.0 发布。付费云同步层与插件市场为商业层、不进 core——任何插件最终都过同一条「分录求和=0」的 core 防火墙。

---

## 相关阅读

- [`../ARCHITECTURE.md`](../ARCHITECTURE.md) —— 架构全貌与每个关键决策的来龙去脉。
- [`../CONTRIBUTING.md`](../CONTRIBUTING.md) —— 贡献指南与 DCO 全文链接。
- 产品视角（普通用户）：[账户](accounts.md) · [账本与分账](books.md)。
