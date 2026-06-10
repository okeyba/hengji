import type { Account, Book, Posting } from '@app/core';
import type { StoredAccount, StoredBook, StoredBudget, StoredTransaction } from './types';

/**
 * 行映射（浏览器安全，无驱动依赖）。
 * 建表与演进见 ./migrations —— node:sqlite 与 tauri-plugin-sql 两实现共用，
 * 保证桌面与测试环境的数据形状完全一致。
 */

export interface BookRow {
  id: string;
  name: string;
  type: string;
  archived: number;
  created_at: string;
  updated_at: string;
  deleted: number;
}
export interface AccountRow {
  id: string;
  book_id: string;
  name: string;
  type: string;
  parent_id: string | null;
  currency: string;
  archived: number;
  created_at: string;
  updated_at: string;
  deleted: number;
}
export interface TxnRow {
  id: string;
  book_id: string;
  date: string;
  payee: string;
  note: string;
  tags: string;
  created_at: string;
  updated_at: string;
  deleted: number;
}
export interface PostingRow {
  id: string;
  txn_id: string;
  account_id: string;
  amount: number;
  currency: string;
}
export interface BudgetRow {
  id: string;
  book_id: string;
  account_id: string;
  monthly_limit: number;
  created_at: string;
  updated_at: string;
  deleted: number;
}

export function toBook(r: BookRow): StoredBook {
  return {
    id: r.id,
    name: r.name,
    type: r.type as Book['type'],
    archived: r.archived !== 0,
    createdAt: r.created_at,
    updatedAt: r.updated_at,
    deleted: r.deleted !== 0,
  };
}

export function toAccount(r: AccountRow): StoredAccount {
  return {
    id: r.id,
    bookId: r.book_id,
    name: r.name,
    type: r.type as Account['type'],
    parentId: r.parent_id,
    currency: r.currency,
    archived: r.archived !== 0,
    createdAt: r.created_at,
    updatedAt: r.updated_at,
    deleted: r.deleted !== 0,
  };
}

export function toPosting(r: PostingRow): Posting {
  return { id: r.id, txnId: r.txn_id, accountId: r.account_id, amount: r.amount, currency: r.currency };
}

export function toBudget(r: BudgetRow): StoredBudget {
  return {
    id: r.id,
    bookId: r.book_id,
    accountId: r.account_id,
    monthlyLimit: r.monthly_limit,
    createdAt: r.created_at,
    updatedAt: r.updated_at,
    deleted: r.deleted !== 0,
  };
}

export function toTxn(r: TxnRow, postings: Posting[]): StoredTransaction {
  return {
    id: r.id,
    bookId: r.book_id,
    date: r.date,
    payee: r.payee,
    note: r.note,
    tags: JSON.parse(r.tags) as string[],
    postings,
    createdAt: r.created_at,
    updatedAt: r.updated_at,
    deleted: r.deleted !== 0,
  };
}
