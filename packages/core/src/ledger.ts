import type { Posting, Transaction } from './types';
import { assertMinor } from './money';

/** 一组 posting 的金额之和。 */
export function balanceOf(postings: Posting[]): number {
  return postings.reduce((sum, p) => sum + p.amount, 0);
}

export function isBalanced(postings: Posting[]): boolean {
  return balanceOf(postings) === 0;
}

export function assertBalanced(postings: Posting[]): void {
  const b = balanceOf(postings);
  if (b !== 0) {
    throw new Error(`交易未平衡：postings 之和为 ${b}，应为 0`);
  }
}

export type EntryKind = 'expense' | 'income' | 'transfer';

interface EntryBase {
  bookId: string;
  date: string;
  /** 正的最小单位金额 */
  amount: number;
  currency?: string;
  payee?: string;
  note?: string;
  tags?: string[];
}

/**
 * 单式记账输入（面向用户的简单语义），由 expandEntry 自动展开成平衡的复式分录。
 * - expense：钱从 accountId（资产/负债）流出，归类到 categoryId（费用科目）
 * - income：钱进入 accountId（资产），来源为 categoryId（收入科目）
 * - transfer：从 fromAccountId 转到 toAccountId
 */
export type EntryInput =
  | (EntryBase & { kind: 'expense'; accountId: string; categoryId: string })
  | (EntryBase & { kind: 'income'; accountId: string; categoryId: string })
  | (EntryBase & { kind: 'transfer'; fromAccountId: string; toAccountId: string });

/**
 * 把单式输入展开为一笔平衡的复式交易。
 * genId 由调用方注入（store/shell 传 crypto.randomUUID，测试传确定性计数器），
 * 以保持 core 纯函数、无环境依赖。
 */
export function expandEntry(input: EntryInput, genId: () => string): Transaction {
  assertMinor(input.amount, 'amount');
  if (input.amount <= 0) {
    throw new Error('amount 必须为正数（最小单位）');
  }
  const currency = input.currency ?? 'CNY';
  const txnId = genId();
  const mk = (accountId: string, amount: number): Posting => ({
    id: genId(),
    txnId,
    accountId,
    amount,
    currency,
  });

  let postings: Posting[];
  switch (input.kind) {
    case 'expense':
      postings = [mk(input.categoryId, input.amount), mk(input.accountId, -input.amount)];
      break;
    case 'income':
      postings = [mk(input.accountId, input.amount), mk(input.categoryId, -input.amount)];
      break;
    case 'transfer':
      postings = [mk(input.toAccountId, input.amount), mk(input.fromAccountId, -input.amount)];
      break;
    default: {
      const _exhaustive: never = input;
      throw new Error(`未知的记账类型：${String(_exhaustive)}`);
    }
  }

  assertBalanced(postings);
  return {
    id: txnId,
    bookId: input.bookId,
    date: input.date,
    payee: input.payee ?? '',
    note: input.note ?? '',
    tags: input.tags ?? [],
    postings,
  };
}
