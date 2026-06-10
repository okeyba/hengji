import type { Budget, Transaction } from './types';

export interface BudgetLine {
  accountId: string;
  limit: number;
  /** 该月该科目已用（minor）。费用科目下即支出额。 */
  spent: number;
  remaining: number;
  over: boolean;
}

/**
 * 某月各预算的使用情况。month 形如 'YYYY-MM'。
 * spent = 该月内、该科目所有 posting 之和（费用科目下即当月支出）。
 */
export function budgetUsage(txns: Transaction[], budgets: Budget[], month: string): BudgetLine[] {
  const spentByAccount = new Map<string, number>();
  for (const t of txns) {
    if (!t.date.startsWith(month)) continue;
    for (const p of t.postings) {
      spentByAccount.set(p.accountId, (spentByAccount.get(p.accountId) ?? 0) + p.amount);
    }
  }
  return budgets.map((b) => {
    const spent = spentByAccount.get(b.accountId) ?? 0;
    return {
      accountId: b.accountId,
      limit: b.monthlyLimit,
      spent,
      remaining: b.monthlyLimit - spent,
      over: spent > b.monthlyLimit,
    };
  });
}
