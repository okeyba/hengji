/**
 * 周期记账编排层（UI 与 store 之间）：把"模板 + 日期"翻译成 EntryInput → core expandEntry
 * 生成平衡分录 → 落库，并在同一 repo.transaction 内把 nextDueDate 推进一期——避免"交易写成功
 * 但推进失败/半截 → 下次渲染重复出现同一期已确认交易"的问题。core 保持纯，此层只做编排
 * （镜像 biz.ts 的 revertOrderCompletion/recordCollection 用 repo.transaction 组合多个 repo 原语的写法）。
 */
import { advanceDueDate, expandEntry, templateToEntryInput } from '@app/core';
import type { RecurringRule } from '@app/core';
import type { Repository, StoredRecurringRule, StoredTransaction } from '@app/store';
import { genId } from './db';

/** 确认一期：按 opts 覆盖模板默认值（金额可编辑，不锁死）生成交易，原子写入 + 推进 nextDueDate。 */
export async function confirmRecurring(
  repo: Repository,
  rule: RecurringRule,
  opts: { date: string; amount?: number; payee?: string; note?: string },
): Promise<StoredTransaction> {
  return repo.transaction(async () => {
    const input = templateToEntryInput(rule, opts.date);
    if (opts.amount !== undefined) input.amount = opts.amount;
    if (opts.payee !== undefined) input.payee = opts.payee;
    if (opts.note !== undefined) input.note = opts.note;
    const txn = await repo.addTransaction(expandEntry(input, genId));
    await repo.updateRecurringRule(rule.id, { nextDueDate: advanceDueDate(rule).nextDueDate });
    return txn;
  });
}

/** 跳过本次：只推进 nextDueDate，不生成交易（请假/提前还清等场景）。单一写操作，不需要 transaction 包裹。 */
export async function skipRecurring(repo: Repository, rule: RecurringRule): Promise<StoredRecurringRule> {
  return repo.updateRecurringRule(rule.id, { nextDueDate: advanceDueDate(rule).nextDueDate });
}
