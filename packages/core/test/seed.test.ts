import { describe, it, expect } from 'vitest';
import { defaultChartOfAccounts } from '../src/index';

function counter(): () => string {
  let n = 0;
  return () => `id${++n}`;
}

describe('defaultChartOfAccounts', () => {
  it('返回默认科目表，数量/类型分布正确、id 唯一、币种 CNY', () => {
    const acc = defaultChartOfAccounts(counter());
    expect(acc.length).toBe(20);
    expect(new Set(acc.map((a) => a.id)).size).toBe(20);
    expect(acc.every((a) => a.currency === 'CNY')).toBe(true);

    const byType = (t: string): number => acc.filter((a) => a.type === t).length;
    expect(byType('asset')).toBe(5);
    expect(byType('liability')).toBe(2);
    expect(byType('equity')).toBe(1);
    expect(byType('income')).toBe(4);
    expect(byType('expense')).toBe(8);

    expect(acc.some((a) => a.name === '投资盈亏')).toBe(true);
    expect(acc.some((a) => a.name === '期初余额')).toBe(true);
  });

  it('支持自定义币种', () => {
    const acc = defaultChartOfAccounts(counter(), 'USD');
    expect(acc.every((a) => a.currency === 'USD')).toBe(true);
  });
});
