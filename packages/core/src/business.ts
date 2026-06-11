import type { OrderLine, Transaction } from './types';
import { expandEntry } from './ledger';

/**
 * 生意单据 → 复式分录（纯函数，账户由调用方解析后注入 id）。
 * B 期两笔自动分录都复用 expandEntry：确认收入 = income，收款核销 = transfer。
 */

/** 单行金额（最小单位）：数量 × 单价，四舍五入到整数分以守住「金额=整数分」不变式。 */
export function lineTotal(line: Pick<OrderLine, 'qty' | 'unitPrice'>): number {
  return Math.round(line.qty * line.unitPrice);
}

/** 订单总额（最小单位）= 各行金额之和。 */
export function orderTotal(lines: ReadonlyArray<Pick<OrderLine, 'qty' | 'unitPrice'>>): number {
  return lines.reduce((sum, l) => sum + lineTotal(l), 0);
}

interface EntryOpts {
  bookId: string;
  date: string;
  /** 正数最小单位 */
  amount: number;
  payee?: string;
  note?: string;
  currency?: string;
}

/**
 * 订单完成 → 确认收入（赊销）：借 应收账款/客户（资产+），贷 营业收入（收入−）。
 * income 展开：accountId=应收子科目，categoryId=营业收入科目。
 */
export function orderRevenueEntry(
  opts: EntryOpts & { receivableAccountId: string; revenueAccountId: string },
  genId: () => string,
): Transaction {
  return expandEntry(
    {
      kind: 'income',
      bookId: opts.bookId,
      date: opts.date,
      amount: opts.amount,
      accountId: opts.receivableAccountId,
      categoryId: opts.revenueAccountId,
      payee: opts.payee,
      note: opts.note,
      currency: opts.currency,
    },
    genId,
  );
}

/**
 * 收款核销：钱从 应收账款/客户（资产）转入收款资产账户（微信商户/对公账户…）。
 * transfer 展开：fromAccountId=应收子科目，toAccountId=收款账户。净资产不变，只是应收转为现金。
 */
export function collectionEntry(
  opts: EntryOpts & { receivableAccountId: string; assetAccountId: string },
  genId: () => string,
): Transaction {
  return expandEntry(
    {
      kind: 'transfer',
      bookId: opts.bookId,
      date: opts.date,
      amount: opts.amount,
      fromAccountId: opts.receivableAccountId,
      toAccountId: opts.assetAccountId,
      payee: opts.payee,
      note: opts.note,
      currency: opts.currency,
    },
    genId,
  );
}

/** 单个订单的收款状态。 */
export type OrderPaymentStatus = 'unpaid' | 'partial' | 'paid';

export interface OrderAllocation {
  orderId: string;
  total: number;
  /** FIFO 摊到本单的已收金额（最小单位） */
  collected: number;
  status: OrderPaymentStatus;
}

export interface CustomerLedger {
  /** 各已完成订单的收款分摊（按下单先后） */
  allocations: OrderAllocation[];
  /** 客户欠你（净额，≥0） */
  receivable: number;
  /** 你欠客户 / 预收（净额，≥0）——多付的钱，可抵后续订单 */
  prepaid: number;
}

/**
 * 把客户累计收款按下单先后（FIFO）摊到其已完成订单：
 * 先还最早的单，多付自动滚到后续订单，全部还清后剩余即预收（credit）。
 * @param orders 该客户的已完成订单（id + total 最小单位 + date）
 * @param totalCollected 该客户累计已收（最小单位，≥0）
 */
export function allocateCustomerPayments(
  orders: ReadonlyArray<{ id: string; total: number; date: string }>,
  totalCollected: number,
): CustomerLedger {
  const collected0 = Math.max(0, totalCollected);
  const sorted = [...orders].sort((a, b) =>
    a.date < b.date ? -1 : a.date > b.date ? 1 : a.id < b.id ? -1 : a.id > b.id ? 1 : 0,
  );
  let remaining = collected0;
  const allocations: OrderAllocation[] = sorted.map((o) => {
    const collected = Math.min(remaining, o.total);
    remaining -= collected;
    const status: OrderPaymentStatus = collected <= 0 ? 'unpaid' : collected < o.total ? 'partial' : 'paid';
    return { orderId: o.id, total: o.total, collected, status };
  });
  const totalOrdered = sorted.reduce((s, o) => s + o.total, 0);
  return {
    allocations,
    receivable: Math.max(0, totalOrdered - collected0),
    prepaid: Math.max(0, collected0 - totalOrdered),
  };
}
