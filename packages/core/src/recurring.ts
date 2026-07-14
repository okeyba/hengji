import type { EntryInput } from './ledger';
import type { RecurringRule } from './types';

/** 一条待确认项：规则 + 累计到期期数（>=1，含当期；>1 即逾期多期，UI 可显示"逾期 N 期"）。 */
export interface PendingOccurrence {
  rule: RecurringRule;
  periodsDue: number;
}

const pad2 = (n: number): string => String(n).padStart(2, '0');

function daysInMonth(year: number, month1: number): number {
  return new Date(Date.UTC(year, month1, 0)).getUTCDate();
}

function clampDay(year: number, month1: number, day: number): number {
  return Math.min(day, daysInMonth(year, month1));
}

/**
 * 推进一期（月末夹断）：总是从 dayOfMonth（规则的固定目标日）算下月同日，短月夹到当月最后一天。
 * 不从"上次实际落地的日期"递推——否则 31 号的规则撞上 2 月后会永久卡在 28/29 号；
 * 这样保证 3 月会跳回 31 号（sticky 到 dayOfMonth，而非累计漂移）。
 */
export function advanceDueDate(rule: Pick<RecurringRule, 'nextDueDate' | 'dayOfMonth'>): { nextDueDate: string } {
  const [y, m] = rule.nextDueDate.split('-').map(Number) as [number, number];
  const ny = m === 12 ? y + 1 : y;
  const nm = m === 12 ? 1 : m + 1;
  return { nextDueDate: `${ny}-${pad2(nm)}-${pad2(clampDay(ny, nm, rule.dayOfMonth))}` };
}

/** 首次到期日：今天当月的 dayOfMonth 若未过（含今天）则用本月，否则下月；同样月末夹断。供"新建规则"表单用。 */
export function firstDueDate(dayOfMonth: number, today: string): string {
  const [y, m, d] = today.split('-').map(Number) as [number, number, number];
  const thisMonthDay = clampDay(y, m, dayOfMonth);
  if (thisMonthDay >= d) return `${y}-${pad2(m)}-${pad2(thisMonthDay)}`;
  const ny = m === 12 ? y + 1 : y;
  const nm = m === 12 ? 1 : m + 1;
  return `${ny}-${pad2(nm)}-${pad2(clampDay(ny, nm, dayOfMonth))}`;
}

function countPeriodsDue(rule: RecurringRule, today: string): number {
  let n = 1;
  let cur = rule.nextDueDate;
  for (;;) {
    const next = advanceDueDate({ nextDueDate: cur, dayOfMonth: rule.dayOfMonth }).nextDueDate;
    if (next > today) break;
    cur = next;
    n++;
  }
  return n;
}

/**
 * 待确认清单：today >= nextDueDate 即"待确认"。periodsDue 只报数、不批量生成——
 * 一次确认＝一期＝一笔交易，逾期多期靠 UI 显示"逾期 N 期"、用户逐期点确认，绝不静默批量生成。
 */
export function pendingRecurring(rules: RecurringRule[], today: string): PendingOccurrence[] {
  return rules
    .filter((r) => r.active && r.nextDueDate <= today)
    .map((r) => ({ rule: r, periodsDue: countPeriodsDue(r, today) }))
    .sort((a, b) => (a.rule.nextDueDate < b.rule.nextDueDate ? -1 : a.rule.nextDueDate > b.rule.nextDueDate ? 1 : 0));
}

/** 模板 + 日期 → EntryInput（不生成 id，expandEntry 负责）。三种 kind 精确镜像 EntryInput 的字段形状。 */
export function templateToEntryInput(rule: RecurringRule, date: string): EntryInput {
  const base = { bookId: rule.bookId, date, amount: rule.amount, currency: rule.currency, payee: rule.payee, note: rule.note, tags: rule.tags };
  switch (rule.kind) {
    case 'expense':
    case 'income':
      if (!rule.assetAccountId || !rule.categoryAccountId) throw new Error('周期记账模板缺少账户/分类');
      return { ...base, kind: rule.kind, accountId: rule.assetAccountId, categoryId: rule.categoryAccountId };
    case 'transfer':
      if (!rule.fromAccountId || !rule.toAccountId) throw new Error('周期记账模板缺少转出/转入账户');
      return { ...base, kind: 'transfer', fromAccountId: rule.fromAccountId, toAccountId: rule.toAccountId };
    default: {
      const _exhaustive: never = rule.kind;
      throw new Error(`未知的周期记账类型：${String(_exhaustive)}`);
    }
  }
}

function addDays(date: string, n: number): string {
  const [y, m, d] = date.split('-').map(Number) as [number, number, number];
  const dt = new Date(Date.UTC(y, m - 1, d + n));
  return `${dt.getUTCFullYear()}-${pad2(dt.getUTCMonth() + 1)}-${pad2(dt.getUTCDate())}`;
}

/** 未来到期清单（日历的 MVP 替代，非真日历网格）：windowDays 天内（含已到期）的启用规则，按 nextDueDate 升序。 */
export function upcomingRecurring(rules: RecurringRule[], today: string, windowDays = 30): RecurringRule[] {
  const cutoff = addDays(today, windowDays);
  return rules.filter((r) => r.active && r.nextDueDate <= cutoff).sort((a, b) => (a.nextDueDate < b.nextDueDate ? -1 : 1));
}
