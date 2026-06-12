import { useState } from 'react';
import type { AccountingBasis, ConvertCtx } from '@app/core';
import type { Repository, StoredSetting } from '@app/store';
import { CURRENCIES, CURRENCY_LABEL } from '../format';
import {
  APP_SCOPE,
  BASIS_KEY,
  FX_RATES_KEY,
  RECON_DAY_KEY,
  RECON_LEAD_KEY,
  basisOf,
  reconcileDayOf,
  reconcileLeadOf,
} from '../settings';

const OPTIONS: Array<{ value: AccountingBasis; label: string; desc: string }> = [
  { value: 'accrual', label: '权责发生制', desc: '订单完成即确认收入（含未收的赊账）。正规、贴资产负债表。' },
  { value: 'cash', label: '收付实现制', desc: '只把实际到账算收入，赊账等收到钱才计。直观、贴日常现金流。' },
];

/** 全局设置：记账口径 / 对账提醒 / 汇率表，全部应用于所有账本。 */
export default function Settings({
  repo,
  settings,
  convert,
  reload,
}: {
  repo: Repository;
  settings: StoredSetting[];
  convert: ConvertCtx;
  reload: () => Promise<void>;
}) {
  const basis = basisOf(settings);
  const reconDay = reconcileDayOf(settings);
  const reconLead = reconcileLeadOf(settings);
  const [saving, setSaving] = useState(false);

  async function save(key: string, value: string): Promise<void> {
    if (saving) return;
    setSaving(true);
    try {
      await repo.setSetting(APP_SCOPE, key, value);
      await reload();
    } finally {
      setSaving(false);
    }
  }

  async function saveRate(currency: string, raw: string): Promise<void> {
    const n = Number(raw);
    const next: Record<string, number> = {};
    for (const c of CURRENCIES) {
      if (c === 'CNY') continue;
      const r = c === currency ? n : convert.rates[c];
      if (typeof r === 'number' && Number.isFinite(r) && r > 0) next[c] = r;
    }
    setSaving(true);
    try {
      await repo.setSetting(APP_SCOPE, FX_RATES_KEY, JSON.stringify(next));
      await reload();
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <div className="main-head">
        <h2>设置</h2>
        <span className="muted">全局 · 应用于所有账本</span>
      </div>

      <div className="card">
        <h3>记账口径</h3>
        <p className="muted small">
          切换「本月收入 / 利润」的计算口径。底层分录始终按权责发生制记账，这里只改报表呈现，不改动任何已记交易。
          仅对有赊账的生意账本有差异，个人 / 投资账本两种口径结果一致。
        </p>
        <div className="opt-list">
          {OPTIONS.map((o) => (
            <button
              key={o.value}
              className={`opt-card${basis === o.value ? ' on' : ''}`}
              onClick={() => void save(BASIS_KEY, o.value)}
              disabled={saving}
            >
              <span className="opt-radio" aria-hidden />
              <span className="opt-body">
                <span className="opt-label">{o.label}</span>
                <span className="muted small">{o.desc}</span>
              </span>
            </button>
          ))}
        </div>
      </div>

      <div className="card">
        <h3>对账提醒</h3>
        <p className="muted small">设定每月对账日后，临近时在账本顶部提醒去「对账」页核对；某账本已全部核销则不打扰。</p>
        <div className="rec-setup">
          <label>
            对账日
            <select value={reconDay} onChange={(e) => void save(RECON_DAY_KEY, e.target.value)} disabled={saving}>
              <option value="">关闭提醒</option>
              <option value="last">每月最后一天</option>
              {Array.from({ length: 28 }, (_, i) => String(i + 1)).map((d) => (
                <option key={d} value={d}>
                  每月 {d} 日
                </option>
              ))}
            </select>
          </label>
          {reconDay !== '' && (
            <label>
              提前提醒
              <select value={String(reconLead)} onChange={(e) => void save(RECON_LEAD_KEY, e.target.value)} disabled={saving}>
                {[0, 1, 2, 3, 5, 7].map((n) => (
                  <option key={n} value={String(n)}>
                    {n === 0 ? '当天' : `提前 ${n} 天`}
                  </option>
                ))}
              </select>
            </label>
          )}
        </div>
      </div>

      <div className="card">
        <h3>汇率表</h3>
        <p className="muted small">
          多币种账户在财务总表里按币种分组，并用这里的汇率折合成人民币总值。折合仅用于展示，不改原币余额。
        </p>
        <div className="fx-grid">
          {CURRENCIES.filter((c) => c !== 'CNY').map((c) => (
            <label key={c} className="fx-row">
              <span>1 {CURRENCY_LABEL[c]} =</span>
              <span className="fx-input">
                <input
                  inputMode="decimal"
                  defaultValue={String(convert.rates[c] ?? '')}
                  placeholder="如 7.10"
                  onBlur={(e) => void saveRate(c, e.target.value)}
                  disabled={saving}
                />
                <span className="muted">元</span>
              </span>
            </label>
          ))}
        </div>
      </div>
    </>
  );
}
