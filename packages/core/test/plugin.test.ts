import { describe, it, expect } from 'vitest';
import { expandDocumentEntry, evalAmount } from '../src/index';
import type { EvalScope, ResolvedLeg } from '../src/index';

function counter(): () => string {
  let n = 0;
  return () => `id${n++}`;
}

const scope = (over: Partial<EvalScope> = {}): EvalScope => ({ lineTotal: 0, feeFields: {}, fields: {}, ...over });
const amt = (txn: { postings: Array<{ accountId: string; amount: number }> }, acct: string): number | undefined =>
  txn.postings.find((p) => p.accountId === acct)?.amount;

describe('plugin 声明式单据运行时（插件地基）', () => {
  it('evalAmount 各来源', () => {
    const s = scope({ lineTotal: 100000, feeFields: { commission: 4000 }, fields: { tip: 500 } });
    expect(evalAmount({ src: 'lineTotal' }, s)).toBe(100000);
    expect(evalAmount({ src: 'feeField', key: 'commission' }, s)).toBe(4000);
    expect(evalAmount({ src: 'feeField', key: 'missing' }, s)).toBe(0);
    expect(evalAmount({ src: 'field', key: 'tip' }, s)).toBe(500);
    expect(evalAmount({ src: 'fixed', value: 1200 }, s)).toBe(1200);
  });

  it('平台销售单形态：收入(贷) + 两笔费用(借) + 平衡腿(应收) 自动配平', () => {
    const legs: ResolvedLeg[] = [
      { accountId: 'rev', side: 'credit', amount: { src: 'lineTotal' } },
      { accountId: 'comm', side: 'debit', amount: { src: 'feeField', key: 'commission' } },
      { accountId: 'ship', side: 'debit', amount: { src: 'feeField', key: 'shipping' } },
      { accountId: 'ar', side: 'debit', balance: true },
    ];
    const txn = expandDocumentEntry(
      legs,
      scope({ lineTotal: 100000, feeFields: { commission: 4000, shipping: 1200 } }),
      { bookId: 'b1', date: '2026-06-13' },
      counter(),
    );
    expect(amt(txn, 'rev')).toBe(-100000); // 营业收入 贷
    expect(amt(txn, 'comm')).toBe(4000); // 佣金 借
    expect(amt(txn, 'ship')).toBe(1200); // 物流 借
    expect(amt(txn, 'ar')).toBe(94800); // 平台应收款 = 1000 − 40 − 12 = 948
    expect(txn.postings.reduce((s, p) => s + p.amount, 0)).toBe(0); // 借贷平衡
  });

  it('金额为 0 的非平衡腿被丢弃（未选费用/包邮）', () => {
    const legs: ResolvedLeg[] = [
      { accountId: 'rev', side: 'credit', amount: { src: 'lineTotal' } },
      { accountId: 'comm', side: 'debit', amount: { src: 'feeField', key: 'commission' } },
      { accountId: 'ship', side: 'debit', amount: { src: 'feeField', key: 'shipping' } }, // 未选 → 0
      { accountId: 'ar', side: 'debit', balance: true },
    ];
    const txn = expandDocumentEntry(legs, scope({ lineTotal: 100000, feeFields: { commission: 4000 } }), { bookId: 'b1', date: '2026-06-13' }, counter());
    expect(txn.postings.some((p) => p.accountId === 'ship')).toBe(false); // 0 额腿不落
    expect(amt(txn, 'ar')).toBe(96000); // 1000 − 40
    expect(txn.postings.reduce((s, p) => s + p.amount, 0)).toBe(0);
  });

  it('防火墙：无平衡腿且各腿不配平 → 抛错', () => {
    const legs: ResolvedLeg[] = [
      { accountId: 'a', side: 'debit', amount: { src: 'fixed', value: 10000 } },
      { accountId: 'b', side: 'debit', amount: { src: 'fixed', value: 5000 } },
    ];
    expect(() => expandDocumentEntry(legs, scope(), { bookId: 'b1', date: '2026-06-13' }, counter())).toThrow();
  });

  it('多于一条平衡腿 → 抛错', () => {
    const legs: ResolvedLeg[] = [
      { accountId: 'a', side: 'debit', amount: { src: 'lineTotal' } },
      { accountId: 'b', side: 'credit', balance: true },
      { accountId: 'c', side: 'debit', balance: true },
    ];
    expect(() => expandDocumentEntry(legs, scope({ lineTotal: 1000 }), { bookId: 'b1', date: '2026-06-13' }, counter())).toThrow();
  });

  it('非平衡腿缺金额来源 → 抛错', () => {
    const legs: ResolvedLeg[] = [{ accountId: 'a', side: 'debit' }];
    expect(() => expandDocumentEntry(legs, scope(), { bookId: 'b1', date: '2026-06-13' }, counter())).toThrow();
  });

  it('全 0 单据（无任何分录）→ 抛错', () => {
    const legs: ResolvedLeg[] = [
      { accountId: 'rev', side: 'credit', amount: { src: 'lineTotal' } },
      { accountId: 'ar', side: 'debit', balance: true },
    ];
    expect(() => expandDocumentEntry(legs, scope({ lineTotal: 0 }), { bookId: 'b1', date: '2026-06-13' }, counter())).toThrow();
  });
});
