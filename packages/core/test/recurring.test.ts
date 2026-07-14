import { describe, it, expect } from 'vitest';
import { advanceDueDate, expandEntry, firstDueDate, pendingRecurring, templateToEntryInput, upcomingRecurring } from '../src/index';
import type { RecurringRule } from '../src/index';

const B = 'b1';

function rule(overrides: Partial<RecurringRule> = {}): RecurringRule {
  return {
    id: 'r1',
    bookId: B,
    active: true,
    kind: 'expense',
    categoryAccountId: 'rent',
    assetAccountId: 'bank',
    fromAccountId: null,
    toAccountId: null,
    amount: 300000,
    currency: 'CNY',
    payee: '房租',
    note: '',
    tags: [],
    dayOfMonth: 5,
    nextDueDate: '2026-06-05',
    endDate: null,
    ...overrides,
  };
}

describe('advanceDueDate', () => {
  it('月末夹断：31 号规则 1 月→2 月夹到 28（非闰年），2 月→3 月跳回 31（不从上次实际日期递推）', () => {
    const r1 = advanceDueDate({ nextDueDate: '2026-01-31', dayOfMonth: 31 });
    expect(r1.nextDueDate).toBe('2026-02-28');
    const r2 = advanceDueDate({ nextDueDate: r1.nextDueDate, dayOfMonth: 31 });
    expect(r2.nextDueDate).toBe('2026-03-31');
  });

  it('29 号规则：非闰年 1→2 月夹到 28，闰年 1→2 月保留 29', () => {
    expect(advanceDueDate({ nextDueDate: '2026-01-29', dayOfMonth: 29 }).nextDueDate).toBe('2026-02-28');
    expect(advanceDueDate({ nextDueDate: '2028-01-29', dayOfMonth: 29 }).nextDueDate).toBe('2028-02-29');
  });

  it('30 号规则：1→2 月夹到 28，2→3 月恢复 30', () => {
    const r1 = advanceDueDate({ nextDueDate: '2026-01-30', dayOfMonth: 30 });
    expect(r1.nextDueDate).toBe('2026-02-28');
    expect(advanceDueDate({ nextDueDate: r1.nextDueDate, dayOfMonth: 30 }).nextDueDate).toBe('2026-03-30');
  });

  it('跨年边界：12 月推进到次年 1 月', () => {
    expect(advanceDueDate({ nextDueDate: '2026-12-15', dayOfMonth: 15 }).nextDueDate).toBe('2027-01-15');
  });
});

describe('firstDueDate', () => {
  it('本月目标日未到（today < day）→ 用本月', () => {
    expect(firstDueDate(20, '2026-06-05')).toBe('2026-06-20');
  });

  it('本月目标日已过 → 用下月', () => {
    expect(firstDueDate(3, '2026-06-05')).toBe('2026-07-03');
  });

  it('目标日恰好是今天 → 用本月（>= 而非 >）', () => {
    expect(firstDueDate(5, '2026-06-05')).toBe('2026-06-05');
  });

  it('月末夹断：本月 31 号已过短月最后一天时的边界', () => {
    // 2 月只有 28 天，today=2026-02-20 时目标日 31 夹到 28，28 < 20? 不对：28>=20 用本月
    expect(firstDueDate(31, '2026-02-20')).toBe('2026-02-28');
    // today=2026-02-28 当天，28>=28 仍用本月
    expect(firstDueDate(31, '2026-02-28')).toBe('2026-02-28');
  });
});

describe('pendingRecurring', () => {
  it('active=false 的规则不出现', () => {
    expect(pendingRecurring([rule({ active: false, nextDueDate: '2026-06-01' })], '2026-06-05')).toEqual([]);
  });

  it('nextDueDate > today 不出现', () => {
    expect(pendingRecurring([rule({ nextDueDate: '2026-07-01' })], '2026-06-05')).toEqual([]);
  });

  it('nextDueDate === today → 出现，periodsDue=1', () => {
    const out = pendingRecurring([rule({ nextDueDate: '2026-06-05' })], '2026-06-05');
    expect(out).toHaveLength(1);
    expect(out[0]!.periodsDue).toBe(1);
  });

  it('逾期多期：nextDueDate 落后两期未开 → periodsDue=3（含当期，只报数不批量生成）', () => {
    const out = pendingRecurring([rule({ nextDueDate: '2026-04-05' })], '2026-06-05');
    expect(out[0]!.periodsDue).toBe(3);
  });

  it('多条规则按 nextDueDate 升序排序', () => {
    const out = pendingRecurring(
      [rule({ id: 'late', nextDueDate: '2026-06-01' }), rule({ id: 'earlier', nextDueDate: '2026-05-01' })],
      '2026-06-05',
    );
    expect(out.map((p) => p.rule.id)).toEqual(['earlier', 'late']);
  });
});

describe('templateToEntryInput', () => {
  function counter(): () => string {
    let n = 0;
    return () => `id${++n}`;
  }

  it('expense：映射为 accountId/categoryId，接 expandEntry 生成平衡交易', () => {
    const input = templateToEntryInput(rule(), '2026-06-05');
    expect(input).toMatchObject({ kind: 'expense', accountId: 'bank', categoryId: 'rent', amount: 300000 });
    const txn = expandEntry(input, counter());
    expect(txn.postings.reduce((s, p) => s + p.amount, 0)).toBe(0);
  });

  it('income：映射为 accountId/categoryId', () => {
    const input = templateToEntryInput(
      rule({ kind: 'income', categoryAccountId: 'salary', assetAccountId: 'bank' }),
      '2026-06-05',
    );
    expect(input).toMatchObject({ kind: 'income', accountId: 'bank', categoryId: 'salary' });
  });

  it('transfer：映射为 fromAccountId/toAccountId', () => {
    const input = templateToEntryInput(
      rule({ kind: 'transfer', categoryAccountId: null, assetAccountId: null, fromAccountId: 'bank', toAccountId: 'savings' }),
      '2026-06-05',
    );
    expect(input).toMatchObject({ kind: 'transfer', fromAccountId: 'bank', toAccountId: 'savings' });
  });

  it('expense/income 缺账户字段（脏数据）抛错', () => {
    expect(() => templateToEntryInput(rule({ assetAccountId: null }), '2026-06-05')).toThrow(/缺少账户/);
  });

  it('transfer 缺转出/转入账户抛错', () => {
    expect(() =>
      templateToEntryInput(rule({ kind: 'transfer', fromAccountId: null, toAccountId: null }), '2026-06-05'),
    ).toThrow(/缺少转出/);
  });

  it('金额可编辑：覆盖返回对象的 amount 后传 expandEntry，生成交易用覆盖后的金额', () => {
    const input = templateToEntryInput(rule({ amount: 300000 }), '2026-06-05');
    input.amount = 500000;
    const txn = expandEntry(input, counter());
    expect(Math.max(...txn.postings.map((p) => p.amount))).toBe(500000);
  });
});

describe('upcomingRecurring', () => {
  it('窗口内（含已到期）的启用规则按 nextDueDate 升序，窗口外排除', () => {
    const rules = [
      rule({ id: 'past', nextDueDate: '2026-05-20' }),
      rule({ id: 'soon', nextDueDate: '2026-06-10' }),
      rule({ id: 'far', nextDueDate: '2026-08-01' }),
    ];
    const out = upcomingRecurring(rules, '2026-06-05', 30);
    expect(out.map((r) => r.id)).toEqual(['past', 'soon']);
  });

  it('active=false 的规则排除', () => {
    expect(upcomingRecurring([rule({ active: false, nextDueDate: '2026-06-10' })], '2026-06-05', 30)).toEqual([]);
  });
});
