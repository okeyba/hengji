import type { Posting, Transaction } from './types';
import { assertMinor } from './money';
import { assertBalanced } from './ledger';

/**
 * 把某账户的余额"设到"一个目标值，差额记入对方科目，产出一笔平衡交易。
 *
 * 复式记账里不能凭空改一个账户的余额——必须有配对分录。两个典型用途：
 * - 投资现值调整（极简档）：accountId=投资账户(asset)，counterAccountId=投资盈亏(income)；
 *   差额即未实现损益，计入净资产、不动持仓/价格。
 * - 期初余额：新账户从 0 设到初始值，counterAccountId=期初余额(equity)。
 */
export interface AdjustBalanceInput {
  bookId: string;
  date: string;
  /** 要调整余额的账户 */
  accountId: string;
  /** 该账户当前余额（minor，由 accountBalance() 得出；新账户为 0） */
  currentBalance: number;
  /** 目标余额（minor） */
  targetValue: number;
  /** 吸收差额的对方科目（投资→投资盈亏 income；期初→期初余额 equity） */
  counterAccountId: string;
  currency?: string;
  payee?: string;
  note?: string;
  tags?: string[];
}

export function adjustBalanceEntry(input: AdjustBalanceInput, genId: () => string): Transaction {
  assertMinor(input.currentBalance, 'currentBalance');
  assertMinor(input.targetValue, 'targetValue');
  const delta = input.targetValue - input.currentBalance;
  if (delta === 0) {
    throw new Error('余额无变化，无需调整');
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
  const postings = [mk(input.accountId, delta), mk(input.counterAccountId, -delta)];
  assertBalanced(postings);
  return {
    id: txnId,
    bookId: input.bookId,
    date: input.date,
    payee: input.payee ?? '',
    note: input.note ?? '余额调整',
    tags: input.tags ?? [],
    postings,
  };
}
