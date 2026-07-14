import { describe, it, expect } from 'vitest';
import { InMemoryRepository } from '@app/store';
import type { Repository, StoredBook, StoredRecurringRule } from '@app/store';
import { advanceDueDate } from '@app/core';
import { createBookWithChart, genId } from '../src/db';
import { confirmRecurring, skipRecurring } from '../src/recurring';

/**
 * 周期记账编排层（UI 与 store 之间）端到端：用 InMemoryRepository 验证 confirmRecurring/skipRecurring
 * 的接线正确性——core 纯函数、store 契约已各自单测覆盖；本测验"原子推进"与"一次一期"语义。
 */

async function personalBook(repo: Repository): Promise<{ book: StoredBook; bank: string; rent: string }> {
  const { book } = await createBookWithChart(repo, '我的日常', 'personal');
  const accs = await repo.listAccounts({ bookId: book.id, includeArchived: true });
  return {
    book: (await repo.getBook(book.id))!,
    bank: accs.find((a) => a.name === '银行卡')!.id,
    rent: accs.find((a) => a.name === '居住')!.id,
  };
}

async function addRule(repo: Repository, book: StoredBook, bank: string, rent: string, nextDueDate: string): Promise<StoredRecurringRule> {
  return repo.addRecurringRule({
    id: genId(),
    bookId: book.id,
    active: true,
    kind: 'expense',
    categoryAccountId: rent,
    assetAccountId: bank,
    fromAccountId: null,
    toAccountId: null,
    amount: 300000,
    currency: 'CNY',
    payee: '房租',
    note: '',
    tags: [],
    dayOfMonth: 5,
    nextDueDate,
    endDate: null,
  });
}

describe('confirmRecurring', () => {
  it('生成平衡交易 + nextDueDate 恰好推进一期', async () => {
    const repo = new InMemoryRepository();
    const { book, bank, rent } = await personalBook(repo);
    const rule = await addRule(repo, book, bank, rent, '2026-06-05');
    const txn = await confirmRecurring(repo, rule, { date: '2026-06-05' });
    expect(txn.postings.reduce((s, p) => s + p.amount, 0)).toBe(0);
    expect(txn.postings.some((p) => p.accountId === rent && p.amount === 300000)).toBe(true);
    const updated = (await repo.listRecurringRules({ bookId: book.id, includeInactive: true }))[0]!;
    expect(updated.nextDueDate).toBe(advanceDueDate(rule).nextDueDate);
    expect((await repo.listTransactions({ bookId: book.id })).length).toBe(1);
  });

  it('金额覆盖模板默认值：opts.amount 生效', async () => {
    const repo = new InMemoryRepository();
    const { book, bank, rent } = await personalBook(repo);
    const rule = await addRule(repo, book, bank, rent, '2026-06-05');
    const txn = await confirmRecurring(repo, rule, { date: '2026-06-05', amount: 500000 });
    expect(txn.postings.find((p) => p.accountId === rent)!.amount).toBe(500000);
  });

  it('原子性：账户不存在（addTransaction 失败）时 nextDueDate 不被推进', async () => {
    const repo = new InMemoryRepository();
    const { book, bank } = await personalBook(repo);
    const rule = await addRule(repo, book, bank, 'no-such-account', '2026-06-05');
    await expect(confirmRecurring(repo, rule, { date: '2026-06-05' })).rejects.toThrow();
    const cur = (await repo.listRecurringRules({ bookId: book.id, includeInactive: true }))[0]!;
    expect(cur.nextDueDate).toBe('2026-06-05'); // 未推进
  });

  it('连续确认两次（模拟逾期 2 期）：累计推进两期、生成两笔交易，不会一次批量生成', async () => {
    const repo = new InMemoryRepository();
    const { book, bank, rent } = await personalBook(repo);
    const rule = await addRule(repo, book, bank, rent, '2026-04-05');
    const r1 = await confirmRecurring(repo, rule, { date: '2026-06-05' });
    const after1 = (await repo.listRecurringRules({ bookId: book.id, includeInactive: true }))[0]!;
    expect(after1.nextDueDate).toBe('2026-05-05');
    const r2 = await confirmRecurring(repo, after1, { date: '2026-06-05' });
    const after2 = (await repo.listRecurringRules({ bookId: book.id, includeInactive: true }))[0]!;
    expect(after2.nextDueDate).toBe('2026-06-05');
    expect(r1.id).not.toBe(r2.id);
    expect((await repo.listTransactions({ bookId: book.id })).length).toBe(2);
  });
});

describe('skipRecurring', () => {
  it('推进 nextDueDate 但不生成交易', async () => {
    const repo = new InMemoryRepository();
    const { book, bank, rent } = await personalBook(repo);
    const rule = await addRule(repo, book, bank, rent, '2026-06-05');
    const updated = await skipRecurring(repo, rule);
    expect(updated.nextDueDate).toBe(advanceDueDate(rule).nextDueDate);
    expect((await repo.listTransactions({ bookId: book.id })).length).toBe(0);
  });
});
