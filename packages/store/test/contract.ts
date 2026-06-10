import { describe, it, expect } from 'vitest';
import { budgetUsage, expandEntry, incomeExpense, netWorth } from '@app/core';
import type { Account, EntryInput, Transaction } from '@app/core';
import type { Clock, Repository } from '../src/index';

export function fakeClock(): Clock {
  let n = 0;
  return () => `2026-01-01T00:00:${String(n++).padStart(2, '0')}Z`;
}

export function counter(): () => string {
  let n = 0;
  return () => `id${++n}`;
}

export const accounts: Account[] = [
  { id: 'bank', name: '招行卡', type: 'asset', parentId: null, currency: 'CNY', archived: false },
  { id: 'alipay', name: '支付宝', type: 'asset', parentId: null, currency: 'CNY', archived: false },
  { id: 'invest', name: '投资账户', type: 'asset', parentId: null, currency: 'CNY', archived: false },
  { id: 'card', name: '信用卡', type: 'liability', parentId: null, currency: 'CNY', archived: false },
  { id: 'food', name: '餐饮', type: 'expense', parentId: null, currency: 'CNY', archived: false },
  { id: 'supply', name: '进货成本', type: 'expense', parentId: null, currency: 'CNY', archived: false },
  { id: 'salary', name: '工资', type: 'income', parentId: null, currency: 'CNY', archived: false },
  { id: 'sales', name: '营业收入', type: 'income', parentId: null, currency: 'CNY', archived: false },
];

const SEED: EntryInput[] = [
  { kind: 'income', date: '2026-05-01', amount: 500000, accountId: 'bank', categoryId: 'salary' },
  { kind: 'expense', date: '2026-05-03', amount: 3000, accountId: 'bank', categoryId: 'food' },
  { kind: 'transfer', date: '2026-05-05', amount: 100000, fromAccountId: 'bank', toAccountId: 'alipay' },
  { kind: 'income', date: '2026-06-02', amount: 200000, accountId: 'alipay', categoryId: 'sales', tags: ['business'] },
  { kind: 'expense', date: '2026-06-03', amount: 80000, accountId: 'card', categoryId: 'supply', tags: ['business'] },
  { kind: 'transfer', date: '2026-05-10', amount: 100000, fromAccountId: 'bank', toAccountId: 'invest' },
];

async function seed(repo: Repository): Promise<Repository> {
  for (const a of accounts) await repo.addAccount(a);
  const gen = counter();
  for (const e of SEED) await repo.addTransaction(expandEntry(e, gen));
  return repo;
}

/**
 * 共享 Repository 契约：内存与 SQLite 两实现跑同一套，证明对外行为一致。
 */
export function runRepositoryContract(name: string, makeRepo: (now: Clock) => Repository): void {
  describe(`${name} · 账户`, () => {
    it('addAccount 盖上同步元数据，可取回', async () => {
      const repo = makeRepo(fakeClock());
      const a = await repo.addAccount(accounts[0]!);
      expect(a.createdAt).toBe('2026-01-01T00:00:00Z');
      expect(a.updatedAt).toBe('2026-01-01T00:00:00Z');
      expect(a.deleted).toBe(false);
      expect((await repo.getAccount('bank'))!.name).toBe('招行卡');
    });

    it('重复 id 抛错', async () => {
      const repo = makeRepo(fakeClock());
      await repo.addAccount(accounts[0]!);
      await expect(repo.addAccount(accounts[0]!)).rejects.toThrow();
    });

    it('listAccounts 默认排除归档、可选包含；updateAccount 改值 bump updatedAt 保留 createdAt', async () => {
      const repo = makeRepo(fakeClock());
      await repo.addAccount(accounts[0]!); // bank @00
      await repo.addAccount(accounts[1]!); // alipay @01
      const updated = await repo.updateAccount('alipay', { archived: true }); // @02
      expect(updated.archived).toBe(true);
      expect(updated.createdAt).toBe('2026-01-01T00:00:01Z');
      expect(updated.updatedAt).toBe('2026-01-01T00:00:02Z');
      expect((await repo.listAccounts()).map((a) => a.id)).toEqual(['bank']);
      expect((await repo.listAccounts({ includeArchived: true })).map((a) => a.id).sort()).toEqual(['alipay', 'bank']);
    });

    it('updateAccount 不存在抛错', async () => {
      const repo = makeRepo(fakeClock());
      await expect(repo.updateAccount('nope', { name: 'x' })).rejects.toThrow();
    });
  });

  describe(`${name} · 交易`, () => {
    it('addTransaction 拒绝未平衡分录', async () => {
      const repo = makeRepo(fakeClock());
      const bad: Transaction = {
        id: 't1',
        date: '2026-05-01',
        payee: '',
        note: '',
        tags: [],
        postings: [
          { id: 'p1', txnId: 't1', accountId: 'bank', amount: 100, currency: 'CNY' },
          { id: 'p2', txnId: 't1', accountId: 'food', amount: -50, currency: 'CNY' },
        ],
      };
      await expect(repo.addTransaction(bad)).rejects.toThrow();
    });

    it('listTransactions 过滤 + 倒序，软删除后排除', async () => {
      const repo = makeRepo(fakeClock());
      const gen = counter();
      const t1 = await repo.addTransaction(
        expandEntry({ kind: 'income', date: '2026-05-01', amount: 500000, accountId: 'bank', categoryId: 'salary' }, gen),
      );
      const t2 = await repo.addTransaction(
        expandEntry({ kind: 'expense', date: '2026-06-03', amount: 80000, accountId: 'card', categoryId: 'supply', tags: ['business'] }, gen),
      );
      const t3 = await repo.addTransaction(
        expandEntry({ kind: 'expense', date: '2026-05-03', amount: 3000, accountId: 'bank', categoryId: 'food' }, gen),
      );

      expect((await repo.listTransactions()).map((t) => t.date)).toEqual(['2026-06-03', '2026-05-03', '2026-05-01']);
      expect((await repo.listTransactions({ tag: 'business' })).map((t) => t.id)).toEqual([t2.id]);
      expect((await repo.listTransactions({ accountId: 'card' })).map((t) => t.id)).toEqual([t2.id]);
      expect((await repo.listTransactions({ from: '2026-05-01', to: '2026-05-31' })).map((t) => t.id).sort()).toEqual(
        [t1.id, t3.id].sort(),
      );

      await repo.softDeleteTransaction(t3.id);
      expect(await repo.getTransaction(t3.id)).toBeNull();
      expect((await repo.listTransactions()).map((t) => t.id)).not.toContain(t3.id);
    });

    it('updateTransaction 保留 createdAt、bump updatedAt、保持 id', async () => {
      const repo = makeRepo(fakeClock());
      const t = await repo.addTransaction(
        expandEntry({ kind: 'expense', date: '2026-05-03', amount: 3000, accountId: 'bank', categoryId: 'food' }, counter()),
      );
      const replacement = expandEntry(
        { kind: 'expense', date: '2026-05-03', amount: 5000, accountId: 'bank', categoryId: 'food' },
        counter(),
      );
      const updated = await repo.updateTransaction(t.id, replacement);
      expect(updated.id).toBe(t.id);
      expect(updated.createdAt).toBe('2026-01-01T00:00:00Z');
      expect(updated.updatedAt).toBe('2026-01-01T00:00:01Z');
      expect(updated.postings.find((p) => p.accountId === 'food')!.amount).toBe(5000);
    });
  });

  describe(`${name} · 与 core 报表集成`, () => {
    it('整套链路：repo 取数喂给 core 报表，数值正确', async () => {
      const repo = await seed(makeRepo(fakeClock()));
      const txns = await repo.listTransactions();
      const accts = await repo.listAccounts();
      expect(netWorth(txns, accts)).toBe(617000);
      expect(incomeExpense(txns, accts)).toEqual({ income: 700000, expense: 83000, net: 617000 });
      expect(incomeExpense(txns, accts, { tag: 'business' })).toEqual({ income: 200000, expense: 80000, net: 120000 });
    });

    it('软删除餐饮交易后，支出与净收益随之变化', async () => {
      const repo = await seed(makeRepo(fakeClock()));
      const food = (await repo.listTransactions({ accountId: 'food' }))[0]!;
      await repo.softDeleteTransaction(food.id);
      const ie = incomeExpense(await repo.listTransactions(), await repo.listAccounts());
      expect(ie.expense).toBe(80000); // 83000 - 3000
      expect(ie.net).toBe(620000); // 617000 + 3000
    });
  });

  describe(`${name} · 预算`, () => {
    it('add/list/update/remove + 同步元数据', async () => {
      const repo = makeRepo(fakeClock());
      const b = await repo.addBudget({ id: 'b1', accountId: 'food', monthlyLimit: 50000 }); // @00
      expect(b.createdAt).toBe('2026-01-01T00:00:00Z');
      expect(b.deleted).toBe(false);
      await repo.addBudget({ id: 'b2', accountId: 'shopping', monthlyLimit: 100000 }); // @01
      expect((await repo.listBudgets()).map((x) => x.id).sort()).toEqual(['b1', 'b2']);

      const u = await repo.updateBudget('b1', { monthlyLimit: 60000 }); // @02
      expect(u.monthlyLimit).toBe(60000);
      expect(u.createdAt).toBe('2026-01-01T00:00:00Z');
      expect(u.updatedAt).toBe('2026-01-01T00:00:02Z');

      await repo.removeBudget('b2');
      expect((await repo.listBudgets()).map((x) => x.id)).toEqual(['b1']);
    });

    it('重复/不存在抛错', async () => {
      const repo = makeRepo(fakeClock());
      await repo.addBudget({ id: 'b1', accountId: 'food', monthlyLimit: 50000 });
      await expect(repo.addBudget({ id: 'b1', accountId: 'food', monthlyLimit: 1 })).rejects.toThrow();
      await expect(repo.updateBudget('nope', { monthlyLimit: 1 })).rejects.toThrow();
      await expect(repo.removeBudget('nope')).rejects.toThrow();
    });

    it('集成 budgetUsage：repo 预算 + 交易喂给计算', async () => {
      const repo = makeRepo(fakeClock());
      await repo.addBudget({ id: 'b1', accountId: 'food', monthlyLimit: 50000 });
      const gen = counter();
      await repo.addTransaction(
        expandEntry({ kind: 'expense', date: '2026-06-03', amount: 55000, accountId: 'cash', categoryId: 'food' }, gen),
      );
      const lines = budgetUsage(await repo.listTransactions(), await repo.listBudgets(), '2026-06');
      expect(lines.find((l) => l.accountId === 'food')).toEqual({
        accountId: 'food',
        limit: 50000,
        spent: 55000,
        remaining: -5000,
        over: true,
      });
    });
  });
}
