import { describe, it, expect } from 'vitest';
import { expandEntry, budgetUsage } from '../src/index';
import type { Budget } from '../src/index';

const B = 'b1';

function counter(): () => string {
  let n = 0;
  return () => `id${++n}`;
}

describe('budgetUsage', () => {
  it('按月按分类统计已用/限额/超支，跨月不混入', () => {
    const gen = counter();
    const txns = [
      expandEntry({ kind: 'expense', bookId: B, date: '2026-06-03', amount: 30000, accountId: 'bank', categoryId: 'food' }, gen),
      expandEntry({ kind: 'expense', bookId: B, date: '2026-06-15', amount: 25000, accountId: 'bank', categoryId: 'food' }, gen),
      expandEntry({ kind: 'expense', bookId: B, date: '2026-06-10', amount: 20000, accountId: 'bank', categoryId: 'shopping' }, gen),
      expandEntry({ kind: 'expense', bookId: B, date: '2026-05-20', amount: 99999, accountId: 'bank', categoryId: 'food' }, gen), // 5 月，不计入 6 月
    ];
    const budgets: Budget[] = [
      { id: 'b1', bookId: B, accountId: 'food', monthlyLimit: 50000 },
      { id: 'b2', bookId: B, accountId: 'shopping', monthlyLimit: 100000 },
    ];
    const lines = budgetUsage(txns, budgets, '2026-06');
    expect(lines.find((l) => l.accountId === 'food')).toEqual({
      accountId: 'food',
      limit: 50000,
      spent: 55000,
      remaining: -5000,
      over: true,
    });
    expect(lines.find((l) => l.accountId === 'shopping')).toEqual({
      accountId: 'shopping',
      limit: 100000,
      spent: 20000,
      remaining: 80000,
      over: false,
    });
  });

  it('无消费的预算 spent=0', () => {
    const budgets: Budget[] = [{ id: 'b1', bookId: B, accountId: 'food', monthlyLimit: 50000 }];
    expect(budgetUsage([], budgets, '2026-06')[0]).toEqual({
      accountId: 'food',
      limit: 50000,
      spent: 0,
      remaining: 50000,
      over: false,
    });
  });
});
