import { assertBalanced } from '@app/core';
import type { Account, Budget, Transaction } from '@app/core';
import type {
  AccountPatch,
  BudgetPatch,
  Clock,
  Repository,
  StoredAccount,
  StoredBudget,
  StoredTransaction,
  TxnQuery,
} from './types';

const defaultClock: Clock = () => new Date().toISOString();

/** 深拷贝，隔离 store 内部状态与调用方（DTO 均为 JSON 安全的纯数据）。 */
function clone<T>(v: T): T {
  return JSON.parse(JSON.stringify(v)) as T;
}

/**
 * 内存实现：用于测试与早期 UI 联调。语义与未来 SQLite 实现保持一致：
 * - 写入交易时强制借贷平衡（复用 core 的 assertBalanced）
 * - 软删除（deleted 标记），读取默认排除
 * - 读写边界深拷贝，调用方拿到/传入的对象不会污染内部状态
 */
export class InMemoryRepository implements Repository {
  private readonly accounts = new Map<string, StoredAccount>();
  private readonly txns = new Map<string, StoredTransaction>();
  private readonly budgets = new Map<string, StoredBudget>();
  private readonly now: Clock;

  constructor(opts: { now?: Clock } = {}) {
    this.now = opts.now ?? defaultClock;
  }

  async addAccount(account: Account): Promise<StoredAccount> {
    if (this.accounts.has(account.id)) {
      throw new Error(`账户已存在：${account.id}`);
    }
    const ts = this.now();
    const stored: StoredAccount = { ...clone(account), createdAt: ts, updatedAt: ts, deleted: false };
    this.accounts.set(account.id, stored);
    return clone(stored);
  }

  async getAccount(id: string): Promise<StoredAccount | null> {
    const a = this.accounts.get(id);
    return a && !a.deleted ? clone(a) : null;
  }

  async listAccounts(opts: { includeArchived?: boolean } = {}): Promise<StoredAccount[]> {
    const out: StoredAccount[] = [];
    for (const a of this.accounts.values()) {
      if (a.deleted) continue;
      if (!opts.includeArchived && a.archived) continue;
      out.push(clone(a));
    }
    return out;
  }

  async updateAccount(id: string, patch: AccountPatch): Promise<StoredAccount> {
    const a = this.accounts.get(id);
    if (!a || a.deleted) throw new Error(`账户不存在：${id}`);
    const updated: StoredAccount = { ...a, ...patch, updatedAt: this.now() };
    this.accounts.set(id, updated);
    return clone(updated);
  }

  async addTransaction(txn: Transaction): Promise<StoredTransaction> {
    if (this.txns.has(txn.id)) throw new Error(`交易已存在：${txn.id}`);
    assertBalanced(txn.postings);
    const ts = this.now();
    const stored: StoredTransaction = { ...clone(txn), createdAt: ts, updatedAt: ts, deleted: false };
    this.txns.set(txn.id, stored);
    return clone(stored);
  }

  async getTransaction(id: string): Promise<StoredTransaction | null> {
    const t = this.txns.get(id);
    return t && !t.deleted ? clone(t) : null;
  }

  async listTransactions(query: TxnQuery = {}): Promise<StoredTransaction[]> {
    const out: StoredTransaction[] = [];
    for (const t of this.txns.values()) {
      if (t.deleted) continue;
      if (query.from && t.date < query.from) continue;
      if (query.to && t.date > query.to) continue;
      if (query.tag && !t.tags.includes(query.tag)) continue;
      if (query.accountId && !t.postings.some((p) => p.accountId === query.accountId)) continue;
      out.push(clone(t));
    }
    // 最近的在前：date 倒序，其次 createdAt 倒序
    out.sort((a, b) =>
      a.date < b.date ? 1 : a.date > b.date ? -1 : a.createdAt < b.createdAt ? 1 : a.createdAt > b.createdAt ? -1 : 0,
    );
    return out;
  }

  async updateTransaction(id: string, txn: Transaction): Promise<StoredTransaction> {
    const existing = this.txns.get(id);
    if (!existing || existing.deleted) throw new Error(`交易不存在：${id}`);
    assertBalanced(txn.postings);
    const updated: StoredTransaction = {
      ...clone(txn),
      id, // 保持 id 稳定
      createdAt: existing.createdAt,
      updatedAt: this.now(),
      deleted: false,
    };
    this.txns.set(id, updated);
    return clone(updated);
  }

  async softDeleteTransaction(id: string): Promise<void> {
    const t = this.txns.get(id);
    if (!t || t.deleted) throw new Error(`交易不存在：${id}`);
    this.txns.set(id, { ...t, deleted: true, updatedAt: this.now() });
  }

  // ---- budgets ----
  async addBudget(budget: Budget): Promise<StoredBudget> {
    if (this.budgets.has(budget.id)) throw new Error(`预算已存在：${budget.id}`);
    const ts = this.now();
    const stored: StoredBudget = { ...clone(budget), createdAt: ts, updatedAt: ts, deleted: false };
    this.budgets.set(budget.id, stored);
    return clone(stored);
  }

  async listBudgets(): Promise<StoredBudget[]> {
    const out: StoredBudget[] = [];
    for (const b of this.budgets.values()) {
      if (!b.deleted) out.push(clone(b));
    }
    return out;
  }

  async updateBudget(id: string, patch: BudgetPatch): Promise<StoredBudget> {
    const b = this.budgets.get(id);
    if (!b || b.deleted) throw new Error(`预算不存在：${id}`);
    const updated: StoredBudget = { ...b, ...patch, updatedAt: this.now() };
    this.budgets.set(id, updated);
    return clone(updated);
  }

  async removeBudget(id: string): Promise<void> {
    const b = this.budgets.get(id);
    if (!b || b.deleted) throw new Error(`预算不存在：${id}`);
    this.budgets.set(id, { ...b, deleted: true, updatedAt: this.now() });
  }
}
