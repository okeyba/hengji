import type { Account, Budget, Transaction } from '@app/core';

/** 每条记录都带的同步元数据，为将来的云同步预留。 */
export interface SyncMeta {
  createdAt: string;
  updatedAt: string;
  /** 软删除：保留行、标记删除，便于同步与撤销 */
  deleted: boolean;
}

export type StoredAccount = Account & SyncMeta;
export type StoredTransaction = Transaction & SyncMeta;
export type StoredBudget = Budget & SyncMeta;

/** 时钟注入：返回 ISO 时间戳；默认实现用 Date，测试注入确定性时钟。 */
export type Clock = () => string;

export interface AccountPatch {
  name?: string;
  type?: Account['type'];
  parentId?: string | null;
  currency?: string;
  archived?: boolean;
}

export interface BudgetPatch {
  accountId?: string;
  monthlyLimit?: number;
}

export interface TxnQuery {
  /** 闭区间起始日期 YYYY-MM-DD */
  from?: string;
  /** 闭区间结束日期 */
  to?: string;
  /** 仅含该标签的交易（如 'business'） */
  tag?: string;
  /** 仅含触及该账户的交易 */
  accountId?: string;
}

/**
 * 平台无关的持久层接口。InMemoryRepository 是第一个实现；
 * 将来的 SQLite 实现遵循同一接口，UI 只依赖此接口。
 */
export interface Repository {
  addAccount(account: Account): Promise<StoredAccount>;
  getAccount(id: string): Promise<StoredAccount | null>;
  listAccounts(opts?: { includeArchived?: boolean }): Promise<StoredAccount[]>;
  updateAccount(id: string, patch: AccountPatch): Promise<StoredAccount>;

  addTransaction(txn: Transaction): Promise<StoredTransaction>;
  getTransaction(id: string): Promise<StoredTransaction | null>;
  listTransactions(query?: TxnQuery): Promise<StoredTransaction[]>;
  updateTransaction(id: string, txn: Transaction): Promise<StoredTransaction>;
  softDeleteTransaction(id: string): Promise<void>;

  addBudget(budget: Budget): Promise<StoredBudget>;
  listBudgets(): Promise<StoredBudget[]>;
  updateBudget(id: string, patch: BudgetPatch): Promise<StoredBudget>;
  removeBudget(id: string): Promise<void>;
}
