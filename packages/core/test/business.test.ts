import { describe, it, expect } from 'vitest';
import {
  lineTotal,
  orderTotal,
  orderRevenueEntry,
  collectionEntry,
  allocateCustomerPayments,
  accountBalance,
  isBalanced,
  toMinor,
} from '../src/index';

function counter(): () => string {
  let n = 0;
  return () => `id${++n}`;
}

describe('orderTotal / lineTotal', () => {
  it('整数数量 × 单价', () => {
    expect(lineTotal({ qty: 3, unitPrice: toMinor(12.5) })).toBe(3750);
  });

  it('小数数量四舍五入到整数分', () => {
    // 1.5 × ¥3.33 = ¥4.995 → 499.5 分 → 500
    expect(lineTotal({ qty: 1.5, unitPrice: 333 })).toBe(500);
  });

  it('总额 = 各行之和（逐行取整后相加）', () => {
    const lines = [
      { qty: 2, unitPrice: toMinor(10) },
      { qty: 1.5, unitPrice: 333 },
    ];
    expect(orderTotal(lines)).toBe(2000 + 500);
  });

  it('空订单总额为 0', () => {
    expect(orderTotal([])).toBe(0);
  });
});

describe('orderRevenueEntry（确认收入）', () => {
  it('借应收（资产+）贷营业收入（收入−），平衡', () => {
    const t = orderRevenueEntry(
      { bookId: 'b1', date: '2026-06-10', amount: toMinor(2500), receivableAccountId: 'ar-c1', revenueAccountId: 'rev', payee: '张三' },
      counter(),
    );
    expect(isBalanced(t.postings)).toBe(true);
    expect(t.bookId).toBe('b1');
    expect(t.payee).toBe('张三');
    expect(t.postings.find((p) => p.accountId === 'ar-c1')!.amount).toBe(250000);
    expect(t.postings.find((p) => p.accountId === 'rev')!.amount).toBe(-250000);
  });
});

describe('collectionEntry（收款核销）', () => {
  it('钱从应收转入收款账户，平衡', () => {
    const t = collectionEntry(
      { bookId: 'b1', date: '2026-06-11', amount: toMinor(1000), receivableAccountId: 'ar-c1', assetAccountId: 'wechat' },
      counter(),
    );
    expect(isBalanced(t.postings)).toBe(true);
    expect(t.postings.find((p) => p.accountId === 'wechat')!.amount).toBe(100000);
    expect(t.postings.find((p) => p.accountId === 'ar-c1')!.amount).toBe(-100000);
  });
});

describe('应收余额 = 应收子科目余额（从分录聚合）', () => {
  it('完成订单后应收=总额；部分收款后应收=余款', () => {
    const gen = counter();
    const rev = orderRevenueEntry(
      { bookId: 'b1', date: '2026-06-10', amount: toMinor(2500), receivableAccountId: 'ar-c1', revenueAccountId: 'rev' },
      gen,
    );
    expect(accountBalance([rev], 'ar-c1')).toBe(250000);

    const collect = collectionEntry(
      { bookId: 'b1', date: '2026-06-11', amount: toMinor(1000), receivableAccountId: 'ar-c1', assetAccountId: 'wechat' },
      gen,
    );
    const txns = [rev, collect];
    expect(accountBalance(txns, 'ar-c1')).toBe(150000); // 还欠 ¥1500
    expect(accountBalance(txns, 'wechat')).toBe(100000); // 已收 ¥1000
    expect(accountBalance(txns, 'rev')).toBe(-250000); // 收入累计 ¥2500（收入科目余额为负）
  });
});

describe('allocateCustomerPayments（FIFO 应收状态）', () => {
  const o = (id: string, total: number, date: string) => ({ id, total, date });

  it('未收：collected=0 全部 unpaid，应收=总额', () => {
    const r = allocateCustomerPayments([o('a', 10000, '2026-06-01')], 0);
    expect(r.allocations[0]!.status).toBe('unpaid');
    expect(r.receivable).toBe(10000);
    expect(r.prepaid).toBe(0);
  });

  it('部分收：单笔收一半 → partial，应收=余款', () => {
    const r = allocateCustomerPayments([o('a', 10000, '2026-06-01')], 8000);
    expect(r.allocations[0]).toMatchObject({ collected: 8000, status: 'partial' });
    expect(r.receivable).toBe(2000);
  });

  it('已收清：收满 → paid', () => {
    const r = allocateCustomerPayments([o('a', 10000, '2026-06-01')], 10000);
    expect(r.allocations[0]!.status).toBe('paid');
    expect(r.receivable).toBe(0);
    expect(r.prepaid).toBe(0);
  });

  it('多收：收款 > 总额 → 全部 paid + 预收 credit', () => {
    const r = allocateCustomerPayments([o('a', 10000, '2026-06-01')], 12000);
    expect(r.allocations[0]!.status).toBe('paid');
    expect(r.receivable).toBe(0);
    expect(r.prepaid).toBe(2000);
  });

  it('FIFO：先还最早的单，多付滚到后续单', () => {
    // 两单 ¥100 + ¥50，共收 ¥120 → 早单还清(100)、晚单部分(20)、欠 30
    const r = allocateCustomerPayments([o('late', 5000, '2026-06-05'), o('early', 10000, '2026-06-01')], 12000);
    const early = r.allocations.find((a) => a.orderId === 'early')!;
    const late = r.allocations.find((a) => a.orderId === 'late')!;
    expect(early).toMatchObject({ collected: 10000, status: 'paid' });
    expect(late).toMatchObject({ collected: 2000, status: 'partial' });
    expect(r.receivable).toBe(3000);
    // 分摊按日期升序：early 在前
    expect(r.allocations.map((a) => a.orderId)).toEqual(['early', 'late']);
  });
});
