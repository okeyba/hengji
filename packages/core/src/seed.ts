import type { Account } from './types';

const DEFAULTS: ReadonlyArray<{ name: string; type: Account['type'] }> = [
  // 资产
  { name: '现金', type: 'asset' },
  { name: '招商银行', type: 'asset' },
  { name: '支付宝', type: 'asset' },
  { name: '微信钱包', type: 'asset' },
  { name: '投资账户', type: 'asset' },
  // 负债
  { name: '信用卡', type: 'liability' },
  { name: '花呗', type: 'liability' },
  // 权益
  { name: '期初余额', type: 'equity' },
  // 收入
  { name: '工资', type: 'income' },
  { name: '营业收入', type: 'income' },
  { name: '投资盈亏', type: 'income' },
  { name: '其他收入', type: 'income' },
  // 费用
  { name: '餐饮', type: 'expense' },
  { name: '交通', type: 'expense' },
  { name: '购物', type: 'expense' },
  { name: '居住', type: 'expense' },
  { name: '娱乐', type: 'expense' },
  { name: '医疗', type: 'expense' },
  { name: '进货成本', type: 'expense' },
  { name: '其他支出', type: 'expense' },
];

/**
 * 新用户的默认科目表（开箱即用）。id 由 genId 注入；币种默认 CNY。
 * 含「投资盈亏」（投资现值调整对方科目）与「期初余额」（设期初对方科目）。
 */
export function defaultChartOfAccounts(genId: () => string, currency = 'CNY'): Account[] {
  return DEFAULTS.map((d) => ({
    id: genId(),
    name: d.name,
    type: d.type,
    parentId: null,
    currency,
    archived: false,
  }));
}
