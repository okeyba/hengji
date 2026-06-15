[← Back to manual index](README.md)

# For Developers: Build · Architecture · Contribute

If you want to understand Hengji's code, run it locally, or fork it into your own ledger, this guide gets you started.

**Audience**: developers / anyone wanting to fork / contributors who want to open a PR.

On the surface Hengji is just "log an entry," but underneath it's a rigorous double-entry accounting engine. It's an open-source, local-first desktop app — your data lives 100% in local SQLite, never goes online, never gets uploaded. The current version is v0.2.0 (Windows x64 only, unsigned). This page explains how it's put together.

---

## 1. Three-layer monorepo architecture

The code is a pnpm monorepo split into three layers, with **strictly one-directional, downward dependencies**: the UI depends on the persistence-layer interface and the kernel; the kernel depends on nothing.

| Package | Name | Responsibility |
| --- | --- | --- |
| `packages/core` | `@app/core` | Pure TS, zero-I/O double-entry engine: domain types, single-entry → double-entry expansion, debit/credit balance validation, reports, budgets, the default chart of accounts. |
| `packages/store` | `@app/store` | Persistence-layer abstraction `Repository`: an in-memory implementation + a SQLite implementation (`node:sqlite`, exported via the `@app/store/sqlite` subpath) + a Tauri implementation (`@app/store/tauri`). |
| `apps/web` | `@app/web` | UI (Vite + React 19); depends only on the `Repository` interface and core, never touches the database directly. |
| `apps/desktop` | `@app/desktop` | Tauri 2 desktop shell; bundles the web frontend into a native app and injects the SQLite repository. |

The boundary in one sentence: **the UI never touches the database directly; core depends on store nothing; to switch platforms you only swap the shell + inject the matching `Repository` implementation.** For the detailed layering and the rationale behind every key decision, see [`../ARCHITECTURE.md`](../ARCHITECTURE.md).

---

## 2. Prerequisites

Running the core packages and the web app only needs Node and pnpm; you only need the Rust toolchain to run the desktop app (Tauri).

1. **Node ≥ 24** — the kernel uses Node 24's built-in `node:sqlite`, so there's zero native compilation.
2. **pnpm ≥ 9** — the repo pins `pnpm@9.15.9` (see `packageManager` in the root `package.json`).
3. **Only when you want to run/package the desktop app** do you additionally need:
   - **Rust** + **MSVC** (the C++ build tools on Windows — when installing Visual Studio Build Tools, check "Desktop development with C++").
   - **WebView2** Runtime (usually preinstalled on Win11).

If you only want to read the engine, run the tests, or tweak the UI, just skip step 3.

---

## 3. Common commands

Run these from the repository root.

```sh
pnpm install                       # install all workspace dependencies

pnpm -r test                       # run every package's tests (core unit tests + store contract suite)
pnpm -r typecheck                  # TypeScript type-check across all packages

pnpm --filter @app/web dev         # start the UI (http://localhost:5173)
pnpm --filter @app/web build       # build the web artifacts

pnpm --filter @app/desktop dev     # run the desktop app in dev mode (requires the Rust toolchain)
pnpm --filter @app/desktop build   # package the desktop installer (tauri build)
```

> `pnpm -r test` and `pnpm -r typecheck` also have same-named shortcut scripts in the root `package.json` (`pnpm test` / `pnpm typecheck`); the two are equivalent.

---

## 4. A few conventions you must know

Burn these into your brain before touching the code, or you'll easily write a bug that "compiles but doesn't balance."

### Amounts are always integers in the smallest unit (cents)

Money is represented only as integers, in the smallest unit ("cents," the smallest unit of the renminbi). This **eliminates floating-point error**; non-integer amounts are rejected outright. `¥30.00` is `3000` in the code. Each currency's smallest-unit precision (number of decimal places) is configured by the currency registry — for example, BTC has 8.

### The beancount signed convention

There's no separate "direction" field for debit/credit; instead debit is positive, credit is negative:

- **Every transaction's postings always sum to 0** — that's the debit/credit balance.
- **An account's balance = the sum of all postings under it.**

In core, `assertBalanced` is a **safety firewall**: if postings don't sum to 0 it throws immediately and nothing lands in the database. Multi-currency transactions are grouped by currency and each group sums to 0 (FX / cross-currency transfers are the exception and are exempted separately).

### Single-entry input, double-entry generated automatically

From the user's point of view it's always "log an entry" — fill in "expense ¥30 · Dining · CMB card." Behind the scenes core automatically expands it into a balanced pair of postings (debit Dining expense / credit CMB card); the user never perceives debits and credits. All business documents (orders, purchases, receipts/payments) go down the same path: the operations layer produces candidate postings → core validates the balance → they land in the proper ledger. **Reports always aggregate from postings; there's never a parallel set of books.**

### core is pure functions

Side-effecting things like `genId` and the clock are all **injected from outside**; core itself has no environment dependencies. That's why core's tests are deterministic, and also why core can be reused verbatim on any platform.

---

## 5. Repository contract tests

The heart of the persistence layer is the `Repository` interface — upper layers depend only on this interface, and underneath it can be the in-memory, SQLite, or (desktop) Tauri implementation, all interchangeable.

To guarantee that the multiple implementations behave identically, store uses a single **shared contract suite**: `packages/store/test/contract.ts`. The in-memory implementation and the SQLite implementation **run the same contract**, and if either one's behavior drifts the test goes red.

When adding new behavior to `Repository`, the rule is:

1. First write the behavior into `packages/store/test/contract.ts` (the shared contract).
2. Make **both** the in-memory and SQLite implementations pass.
3. Changes to `core` are **test-first** — write the pure-logic unit test before implementing.

The SQLite driver varies by runtime, but all of them hide behind `Repository`: Node / tests use `node:sqlite`, production desktop uses a self-written rusqlite + SQLCipher bridge (replacing tauri-plugin-sql; currently plaintext, local encryption in progress), and browser / PWA uses wa-sqlite (OPFS). The SQL schema and queries are largely portable across them.

---

## 6. Contribution flow and the DCO sign-off

See [`../CONTRIBUTING.md`](../CONTRIBUTING.md) for the full flow. The key points:

1. Keep changes **small and focused** and match the existing style.
2. Run `pnpm -r test` and `pnpm -r typecheck` locally before committing.
3. **Every commit needs a DCO sign-off** — use `-s`:

   ```sh
   git commit -s -m "your commit message"
   ```

   `-s` appends a line `Signed-off-by: Your Name <email>` to the end of the commit message, attesting that you have the right to submit this code and agree to release it under the project's license. This project uses the Developer Certificate of Origin (DCO 1.1; full text at <https://developercertificate.org>).

### License

core is **Apache-2.0** (including a patent grant). Contributing means agreeing to release under Apache-2.0. The paid cloud-sync layer and the plugin marketplace are commercial layers and don't go into core — any plugin ultimately passes through the same "postings sum to 0" core firewall.

---

## Related reading

- [`../ARCHITECTURE.md`](../ARCHITECTURE.md) — the full architecture and the backstory of every key decision.
- [`../CONTRIBUTING.md`](../CONTRIBUTING.md) — the contribution guide and a link to the full DCO text.
- The product view (for everyday users): [Accounts](accounts.md) · [Books & split-ledger accounting](books.md).
