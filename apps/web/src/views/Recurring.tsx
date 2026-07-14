import { useState } from 'react';
import { firstDueDate, toMinor, upcomingRecurring } from '@app/core';
import type { AppData } from '../App';
import { confirmAsk } from '../confirm';
import { genId } from '../db';
import { currencyDef, todayISO } from '../format';

type Kind = 'expense' | 'income' | 'transfer';

const KIND_LABEL: Record<Kind, string> = { expense: '支出', income: '收入', transfer: '转账' };

export default function Recurring({ data }: { data: AppData }) {
  const { accounts, recurringRules, repo, reload, book } = data;
  const [kind, setKind] = useState<Kind>('expense');
  const [amount, setAmount] = useState('');
  const [payee, setPayee] = useState('');
  const [catId, setCatId] = useState('');
  const [accId, setAccId] = useState('');
  const [toId, setToId] = useState('');
  const [dayOfMonth, setDayOfMonth] = useState('5');
  const [err, setErr] = useState<string | null>(null);

  const cats = accounts.filter((a) => a.type === (kind === 'expense' ? 'expense' : 'income'));
  const reals = accounts.filter((a) => a.type === 'asset' || a.type === 'liability');
  const effCat = cats.some((c) => c.id === catId) ? catId : (cats[0]?.id ?? '');
  const effAcc = reals.some((c) => c.id === accId) ? accId : (reals[0]?.id ?? '');
  const effTo = reals.some((c) => c.id === toId) ? toId : (reals[1]?.id ?? reals[0]?.id ?? '');
  const currency = reals.find((a) => a.id === effAcc)?.currency ?? 'CNY';
  const decimals = currencyDef(currency).decimals;

  const nameOf = (id: string | null): string => accounts.find((a) => a.id === id)?.name ?? '—';
  const upcoming = upcomingRecurring(recurringRules, todayISO(), 30);
  const active = recurringRules.filter((r) => r.active);
  const inactive = recurringRules.filter((r) => !r.active);

  async function add(): Promise<void> {
    setErr(null);
    const major = Number(amount);
    const day = Number(dayOfMonth);
    if (!Number.isFinite(major) || major <= 0) {
      setErr('请输入有效的正数金额');
      return;
    }
    if (!Number.isInteger(day) || day < 1 || day > 31) {
      setErr('每月第几日须为 1-31 的整数');
      return;
    }
    if (kind === 'transfer' && effAcc === effTo) {
      setErr('转出与转入账户不能相同');
      return;
    }
    try {
      await repo.addRecurringRule({
        id: genId(),
        bookId: book.id,
        active: true,
        kind,
        categoryAccountId: kind !== 'transfer' ? effCat : null,
        assetAccountId: kind !== 'transfer' ? effAcc : null,
        fromAccountId: kind === 'transfer' ? effAcc : null,
        toAccountId: kind === 'transfer' ? effTo : null,
        amount: toMinor(major, decimals),
        currency,
        payee,
        note: '',
        tags: [],
        dayOfMonth: day,
        nextDueDate: firstDueDate(day, todayISO()),
        endDate: null,
      });
      setAmount('');
      setPayee('');
      await reload();
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }

  async function toggle(id: string, next: boolean): Promise<void> {
    await repo.updateRecurringRule(id, { active: next });
    await reload();
  }

  async function remove(id: string): Promise<void> {
    if (await confirmAsk('删除该周期记账规则？')) {
      await repo.removeRecurringRule(id);
      await reload();
    }
  }

  return (
    <>
      <div className="main-head">
        <h2>{book.name} · 周期记账</h2>
        <span className="muted">工资、房租、分期等每月固定的记账动作</span>
      </div>

      {upcoming.length > 0 && (
        <div className="card">
          <h3>未来 30 天</h3>
          {upcoming.map((r) => (
            <div className="brow" key={r.id}>
              <div className="bhead">
                <span className="bname">{r.payee || KIND_LABEL[r.kind]}</span>
                <span className="muted small">{r.nextDueDate}</span>
              </div>
            </div>
          ))}
        </div>
      )}

      <div className="card">
        <h3>新增周期记账</h3>
        <div className="qtabs">
          {(Object.keys(KIND_LABEL) as Kind[]).map((k) => (
            <button key={k} className={`qtab${kind === k ? ` on k-${k}` : ''}`} onClick={() => setKind(k)}>
              {KIND_LABEL[k]}
            </button>
          ))}
        </div>
        <div className="qgrid">
          <label>
            金额（{currencyDef(currency).symbol}）
            <input inputMode="decimal" placeholder="0.00" value={amount} onChange={(e) => setAmount(e.target.value)} />
          </label>
          <label>
            每月第几日
            <input type="number" min={1} max={31} value={dayOfMonth} onChange={(e) => setDayOfMonth(e.target.value)} />
          </label>
          {kind !== 'transfer' ? (
            <>
              <label>
                {kind === 'expense' ? '分类' : '来源'}
                <select value={effCat} onChange={(e) => setCatId(e.target.value)}>
                  {cats.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                账户
                <select value={effAcc} onChange={(e) => setAccId(e.target.value)}>
                  {reals.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
              </label>
            </>
          ) : (
            <>
              <label>
                从
                <select value={effAcc} onChange={(e) => setAccId(e.target.value)}>
                  {reals.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                到
                <select value={effTo} onChange={(e) => setToId(e.target.value)}>
                  {reals.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
              </label>
            </>
          )}
          <label className="full">
            备注
            <input placeholder="工资 / 房租 / 分期（可选）" value={payee} onChange={(e) => setPayee(e.target.value)} />
          </label>
        </div>
        {err && <p className="form-err">{err}</p>}
        <button className="btn btn-primary wfull" onClick={() => void add()}>
          添加
        </button>
      </div>

      <div className="card">
        <h3>已启用</h3>
        {active.length === 0 && <p className="muted">还没有周期记账规则，在上方添加一个。</p>}
        {active.map((r) => (
          <div className="brow" key={r.id}>
            <div className="bhead">
              <span className="bname">{r.payee || KIND_LABEL[r.kind]}</span>
              <span className="muted small">
                {KIND_LABEL[r.kind]} ·{' '}
                {r.kind === 'transfer' ? `${nameOf(r.fromAccountId)} → ${nameOf(r.toAccountId)}` : `${nameOf(r.categoryAccountId)} / ${nameOf(r.assetAccountId)}`}{' '}
                · 每月 {r.dayOfMonth} 日 · 下次 {r.nextDueDate}
              </span>
              <button className="del" title="停用" onClick={() => void toggle(r.id, false)}>
                停用
              </button>
              <button className="del" title="删除" onClick={() => void remove(r.id)}>
                ×
              </button>
            </div>
          </div>
        ))}
      </div>

      {inactive.length > 0 && (
        <div className="card">
          <h3>已停用</h3>
          {inactive.map((r) => (
            <div className="brow" key={r.id}>
              <div className="bhead">
                <span className="bname muted">{r.payee || KIND_LABEL[r.kind]}</span>
                <button className="btn" onClick={() => void toggle(r.id, true)}>
                  重新启用
                </button>
                <button className="del" title="删除" onClick={() => void remove(r.id)}>
                  ×
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
