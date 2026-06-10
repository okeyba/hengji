import { describe, it, expect } from 'vitest';
import { adjustBalanceEntry, expandEntry, accountBalance, netWorth, isBalanced } from '../src/index';
import type { Account } from '../src/index';

function counter(): () => string {
  let n = 0;
  return () => `id${++n}`;
}

describe('adjustBalanceEntry', () => {
  it('投资现值上调：投资账户 +delta，投资盈亏 -delta，平衡', () => {
    const t = adjustBalanceEntry(
      { date: '2026-06-01', accountId: 'invest', currentBalance: 100000, targetValue: 110000, counterAccountId: 'pnl' },
      counter(),
    );
    expect(isBalanced(t.postings)).toBe(true);
    expect(t.postings.find((p) => p.accountId === 'invest')!.amount).toBe(10000);
    expect(t.postings.find((p) => p.accountId === 'pnl')!.amount).toBe(-10000);
  });

  it('投资现值下调：delta 为负', () => {
    const t = adjustBalanceEntry(
      { date: '2026-06-01', accountId: 'invest', currentBalance: 100000, targetValue: 90000, counterAccountId: 'pnl' },
      counter(),
    );
    expect(t.postings.find((p) => p.accountId === 'invest')!.amount).toBe(-10000);
    expect(t.postings.find((p) => p.accountId === 'pnl')!.amount).toBe(10000);
  });

  it('期初余额：新账户从 0 设到目标（对方=期初余额权益）', () => {
    const t = adjustBalanceEntry(
      { date: '2026-01-01', accountId: 'bank', currentBalance: 0, targetValue: 500000, counterAccountId: 'opening' },
      counter(),
    );
    expect(t.postings.find((p) => p.accountId === 'bank')!.amount).toBe(500000);
    expect(t.postings.find((p) => p.accountId === 'opening')!.amount).toBe(-500000);
  });

  it('无变化抛错', () => {
    expect(() =>
      adjustBalanceEntry(
        { date: '2026-06-01', accountId: 'invest', currentBalance: 100000, targetValue: 100000, counterAccountId: 'pnl' },
        counter(),
      ),
    ).toThrow();
  });

  it('非整数抛错', () => {
    expect(() =>
      adjustBalanceEntry(
        { date: '2026-06-01', accountId: 'invest', currentBalance: 0, targetValue: 100.5, counterAccountId: 'pnl' },
        counter(),
      ),
    ).toThrow();
  });

  it('集成：投资现值上调后，账户余额=目标、净资产随浮盈增加', () => {
    const accounts: Account[] = [
      { id: 'invest', name: '投资账户', type: 'asset', parentId: null, currency: 'CNY', archived: false },
      { id: 'bank', name: '招行卡', type: 'asset', parentId: null, currency: 'CNY', archived: false },
      { id: 'pnl', name: '投资盈亏', type: 'income', parentId: null, currency: 'CNY', archived: false },
    ];
    const gen = counter();
    const txns = [
      expandEntry({ kind: 'transfer', date: '2026-05-01', amount: 100000, fromAccountId: 'bank', toAccountId: 'invest' }, gen),
      adjustBalanceEntry(
        { date: '2026-06-01', accountId: 'invest', currentBalance: 100000, targetValue: 130000, counterAccountId: 'pnl' },
        gen,
      ),
    ];
    expect(accountBalance(txns, 'invest')).toBe(130000);
    // bank -100000 + invest 130000 = 30000；浮盈 30000 计入净资产，pnl(收入) 不计入
    expect(netWorth(txns, accounts)).toBe(30000);
  });
});
