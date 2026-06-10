import { describe, it, expect, beforeEach } from 'vitest';
import { expandEntry, accountBalance, netWorth, incomeExpense } from '../src/index';
import type { Account, Transaction } from '../src/index';

const accounts: Account[] = [
  { id: 'bank', name: '招行卡', type: 'asset', parentId: null, currency: 'CNY', archived: false },
  { id: 'alipay', name: '支付宝', type: 'asset', parentId: null, currency: 'CNY', archived: false },
  { id: 'invest', name: '投资账户', type: 'asset', parentId: null, currency: 'CNY', archived: false },
  { id: 'card', name: '信用卡', type: 'liability', parentId: null, currency: 'CNY', archived: false },
  { id: 'food', name: '餐饮', type: 'expense', parentId: null, currency: 'CNY', archived: false },
  { id: 'supply', name: '进货成本', type: 'expense', parentId: null, currency: 'CNY', archived: false },
  { id: 'salary', name: '工资', type: 'income', parentId: null, currency: 'CNY', archived: false },
  { id: 'sales', name: '营业收入', type: 'income', parentId: null, currency: 'CNY', archived: false },
];

function build(): Transaction[] {
  let n = 0;
  const gen = (): string => `id${++n}`;
  return [
    expandEntry({ kind: 'income', date: '2026-05-01', amount: 500000, accountId: 'bank', categoryId: 'salary' }, gen),
    expandEntry({ kind: 'expense', date: '2026-05-03', amount: 3000, accountId: 'bank', categoryId: 'food' }, gen),
    expandEntry({ kind: 'transfer', date: '2026-05-05', amount: 100000, fromAccountId: 'bank', toAccountId: 'alipay' }, gen),
    expandEntry({ kind: 'income', date: '2026-06-02', amount: 200000, accountId: 'alipay', categoryId: 'sales', tags: ['business'] }, gen),
    expandEntry({ kind: 'expense', date: '2026-06-03', amount: 80000, accountId: 'card', categoryId: 'supply', tags: ['business'] }, gen),
    expandEntry({ kind: 'transfer', date: '2026-05-10', amount: 100000, fromAccountId: 'bank', toAccountId: 'invest' }, gen),
  ];
}

describe('reports', () => {
  let txns: Transaction[];
  beforeEach(() => {
    txns = build();
  });

  it('accountBalance 汇总单账户', () => {
    expect(accountBalance(txns, 'bank')).toBe(297000); // 500000 -3000 -100000 -100000
    expect(accountBalance(txns, 'alipay')).toBe(300000); // 100000 + 200000
    expect(accountBalance(txns, 'invest')).toBe(100000);
    expect(accountBalance(txns, 'card')).toBe(-80000);
  });

  it('netWorth = 资产 + 负债（有符号）', () => {
    expect(netWorth(txns, accounts)).toBe(617000); // 297000 + 300000 + 100000 - 80000
  });

  it('incomeExpense 全期', () => {
    expect(incomeExpense(txns, accounts)).toEqual({ income: 700000, expense: 83000, net: 617000 });
  });

  it('incomeExpense 按 business 标签 = 小生意利润', () => {
    expect(incomeExpense(txns, accounts, { tag: 'business' })).toEqual({ income: 200000, expense: 80000, net: 120000 });
  });

  it('incomeExpense 按时间区间（仅 5 月）', () => {
    expect(incomeExpense(txns, accounts, { period: { from: '2026-05-01', to: '2026-05-31' } })).toEqual({
      income: 500000,
      expense: 3000,
      net: 497000,
    });
  });
});
