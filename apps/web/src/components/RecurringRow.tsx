import { useState } from 'react';
import { fromMinor, toMinor } from '@app/core';
import type { PendingOccurrence } from '@app/core';
import type { AppData } from '../App';
import { confirmAsk } from '../confirm';
import { confirmRecurring, skipRecurring } from '../recurring';
import { currencyDef, todayISO } from '../format';

const KIND_LABEL = { expense: '支出', income: '收入', transfer: '转账' } as const;

export default function RecurringRow({ item, data }: { item: PendingOccurrence; data: AppData }) {
  const { rule } = item;
  const { repo, reload } = data;
  const decimals = currencyDef(rule.currency).decimals;
  const [amount, setAmount] = useState(String(fromMinor(rule.amount, decimals)));
  const [err, setErr] = useState<string | null>(null);

  async function confirm(): Promise<void> {
    setErr(null);
    const major = Number(amount);
    if (!Number.isFinite(major) || major <= 0) {
      setErr('请输入有效金额');
      return;
    }
    try {
      await confirmRecurring(repo, rule, { date: todayISO(), amount: toMinor(major, decimals) });
      await reload();
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }

  async function skip(): Promise<void> {
    if (await confirmAsk(`跳过本次「${rule.payee || KIND_LABEL[rule.kind]}」？不生成交易，下次到期日照常推进。`)) {
      await skipRecurring(repo, rule);
      await reload();
    }
  }

  async function deactivate(): Promise<void> {
    if (await confirmAsk(`停用周期记账「${rule.payee || KIND_LABEL[rule.kind]}」？可在"周期记账"页重新启用。`)) {
      await repo.updateRecurringRule(rule.id, { active: false });
      await reload();
    }
  }

  return (
    <div className="brow rr-row">
      <div className="bhead">
        <span className="bname">
          {rule.payee || KIND_LABEL[rule.kind]}
          {item.periodsDue > 1 && <span className="chip"> 逾期 {item.periodsDue} 期</span>}
        </span>
        <span className="muted small">
          {KIND_LABEL[rule.kind]} · 每月 {rule.dayOfMonth} 日 · 应到期 {rule.nextDueDate}
        </span>
      </div>
      <div className="rr-actions">
        <input inputMode="decimal" value={amount} onChange={(e) => setAmount(e.target.value)} />
        <button className="btn btn-primary" onClick={() => void confirm()}>
          确认记账
        </button>
        <button className="btn" onClick={() => void skip()}>
          跳过本次
        </button>
        <button className="del" title="停用" onClick={() => void deactivate()}>
          停用
        </button>
      </div>
      {err && <p className="form-err">{err}</p>}
    </div>
  );
}
