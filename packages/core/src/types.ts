/**
 * 复式记账领域类型（平台无关、无 I/O）。
 *
 * 约定（beancount 风格的有符号记账）：
 * - 金额一律用「整数最小单位」（CNY 即「分」），杜绝浮点误差。
 * - 每笔交易的所有 posting 金额之和恒为 0（借贷平衡）。
 * - 账户余额 = 该账户所有 posting 金额之和。
 *   资产/费用余额通常为正，负债/收入/权益通常为负。
 *
 * 多账本（v0.2）：
 * - Book 是顶层容器（个人/生意/投资，可各建多个）；
 *   账户/交易/预算全部挂 bookId，一笔交易的所有分录必须属于同一账本。
 */

export type BookType = 'personal' | 'business' | 'investment';

export interface Book {
  id: string;
  name: string;
  type: BookType;
  archived: boolean;
}

export type AccountType = 'asset' | 'liability' | 'equity' | 'income' | 'expense';

export interface Account {
  id: string;
  bookId: string;
  name: string;
  type: AccountType;
  /** 层级科目；顶层为 null */
  parentId: string | null;
  /** ISO 4217；MVP 单一本位币 'CNY' */
  currency: string;
  archived: boolean;
}

/** 有符号的整数最小单位（如 CNY 的「分」）。 */
export type Minor = number;

export interface Posting {
  id: string;
  txnId: string;
  accountId: string;
  /** 有符号最小单位；同一交易下所有 posting 之和 === 0 */
  amount: Minor;
  currency: string;
}

export interface Transaction {
  id: string;
  bookId: string;
  /** 记账日期 YYYY-MM-DD */
  date: string;
  payee: string;
  note: string;
  /** 维度标签（自由扩展；生意/个人之分已由账本承担） */
  tags: string[];
  postings: Posting[];
}

export interface Budget {
  id: string;
  bookId: string;
  /** 预算针对的科目（通常是费用科目） */
  accountId: string;
  /** 每月限额（minor units） */
  monthlyLimit: number;
}
