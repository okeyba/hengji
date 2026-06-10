/**
 * 复式记账领域类型（平台无关、无 I/O）。
 *
 * 约定（beancount 风格的有符号记账）：
 * - 金额一律用「整数最小单位」（CNY 即「分」），杜绝浮点误差。
 * - 每笔交易的所有 posting 金额之和恒为 0（借贷平衡）。
 * - 账户余额 = 该账户所有 posting 金额之和。
 *   资产/费用余额通常为正，负债/收入/权益通常为负。
 */

export type AccountType = 'asset' | 'liability' | 'equity' | 'income' | 'expense';

export interface Account {
  id: string;
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
  /** 记账日期 YYYY-MM-DD */
  date: string;
  payee: string;
  note: string;
  /** 维度标签，含 'business' / 'personal' */
  tags: string[];
  postings: Posting[];
}

export interface Budget {
  id: string;
  /** 预算针对的科目（通常是费用科目） */
  accountId: string;
  /** 每月限额（minor units） */
  monthlyLimit: number;
}
