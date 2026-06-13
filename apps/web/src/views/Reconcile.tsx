import { useEffect, useMemo, useState } from 'react';
import { accountBalance, adjustBalanceEntry, expandEntry, toMinor, unclearedCount } from '@app/core';
import type { Repository, StoredAccount, StoredBook, StoredReconciliation, StoredTransaction } from '@app/store';
import { genId } from '../db';
import { currencyDef, fmtMoney, todayISO } from '../format';

const GAIN_LOSS = '盘盈盘亏';
type AddKind = 'income' | 'expense';

/**
 * 勾对式对账（账户全局化 Phase 4：全局入口、按账户跨账本）：
 * 选账户 → 填对账单余额 → 逐笔勾选（每条流水标所属账本、可按账本筛选核对各自小计）→ 差额对到 0 → 完成。
 * 全局共享账户的流水散落多账本，故用全量交易聚合该账户分录。补录/调整：全局账户由用户手动选账本，
 * 账本专属账户固定其所属账本。
 */
export default function Reconcile({
  repo,
  accounts,
  allTxns,
  books,
  reload,
}: {
  repo: Repository;
  accounts: StoredAccount[];
  allTxns: StoredTransaction[];
  books: StoredBook[];
  reload: () => Promise<void>;
}) {
  // books 为全量（含归档），故全局账户跨归档账本的历史流水也能查到名字（标「已归档」），不再显示「未知账本」。
  const bookName = (id: string): string => {
    const b = books.find((x) => x.id === id);
    return b ? (b.archived ? `${b.name}（已归档）` : b.name) : '（未知账本）';
  };
  // 补录归属账本只能选未归档账本（不把交易写回已归档账本）。
  const liveBooks = books.filter((b) => !b.archived);

  // 全部资产/负债账户（含全局共享）跨账本对账；按账本名+账户名排序，全局置顶
  const recAccounts = useMemo(
    () =>
      accounts
        .filter((a) => (a.type === 'asset' || a.type === 'liability') && !a.archived)
        .sort((x, y) => Number(!!y.global) - Number(!!x.global) || x.bookId.localeCompare(y.bookId) || x.name.localeCompare(y.name)),
    [accounts],
  );

  const clearedIdsOf = (accountId: string): Set<string> =>
    new Set(allTxns.flatMap((t) => t.postings).filter((p) => p.accountId === accountId && p.cleared).map((p) => p.id));

  const [accountId, setAccountId] = useState(() => recAccounts[0]?.id ?? '');
  const [checked, setChecked] = useState<Set<string>>(() => clearedIdsOf(recAccounts[0]?.id ?? ''));
  const [bookFilter, setBookFilter] = useState<'all' | string>('all'); // 流水按账本筛选（仅核对用，不影响整账户对账）
  const [stmt, setStmt] = useState('');
  const [stmtDate, setStmtDate] = useState(todayISO());
  const [lastRec, setLastRec] = useState<StoredReconciliation | null>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // 内联补录一笔（对账时发现漏记的款项，免去跳「记一笔」丢失勾选进度）
  const [addOpen, setAddOpen] = useState(false);
  const [aKind, setAKind] = useState<AddKind>('expense');
  const [aAmount, setAAmount] = useState('');
  const [aCatId, setACatId] = useState('');
  const [aBook, setABook] = useState('');
  const [aDate, setADate] = useState(todayISO());

  const selAccount = recAccounts.find((a) => a.id === accountId);
  const homeBookId = selAccount?.bookId ?? '';
  const curCode = selAccount?.currency ?? 'CNY';
  const dec = currencyDef(curCode).decimals;

  // 上次对账记录：随账户与数据变化刷新（不碰 checked，避免抹掉勾选中途状态）
  useEffect(() => {
    let alive = true;
    void repo.listReconciliations({ accountId }).then((rs) => {
      if (alive) setLastRec(rs[0] ?? null);
    });
    return () => {
      alive = false;
    };
  }, [accountId, allTxns, repo]);

  function selectAccount(id: string): void {
    setAccountId(id);
    setChecked(clearedIdsOf(id));
    setBookFilter('all');
    setStmt('');
    setMsg(null);
    setErr(null);
  }

  // 该账户的分录（含所在交易的账本），按日期升序——贴对账单阅读顺序
  const allRows = useMemo(() => {
    const out: Array<{ pid: string; txnId: string; bookId: string; date: string; title: string; amount: number; cleared: boolean }> = [];
    for (const t of allTxns) {
      for (const p of t.postings) {
        if (p.accountId !== accountId) continue;
        out.push({ pid: p.id, txnId: t.id, bookId: t.bookId, date: t.date, title: t.payee || t.note || '交易', amount: p.amount, cleared: !!p.cleared });
      }
    }
    return out.sort((a, b) => (a.date !== b.date ? (a.date < b.date ? -1 : 1) : 0));
  }, [allTxns, accountId]);

  // 该账户涉及的账本（用于筛选 chips）
  const rowBooks = useMemo(() => [...new Set(allRows.map((r) => r.bookId))], [allRows]);
  const rows = bookFilter === 'all' ? allRows : allRows.filter((r) => r.bookId === bookFilter);

  const addCats = accounts.filter((a) => a.type === aKind && a.bookId === aBook && !a.global && !a.archived);
  const effACat = addCats.some((c) => c.id === aCatId) ? aCatId : (addCats[0]?.id ?? '');
  // 补录归属账本：全局账户可选任意账本；账本专属账户只能落其所属账本（铁律）
  const addBookOptions = selAccount?.global ? liveBooks : liveBooks.filter((b) => b.id === homeBookId);

  const currentBalance = useMemo(() => accountBalance(allTxns, accountId), [allTxns, accountId]);
  // 已勾选合计 / 差额：始终按整账户（全部 rows），筛选只影响展示
  const checkedSum = useMemo(() => allRows.reduce((s, r) => (checked.has(r.pid) ? s + r.amount : s), 0), [allRows, checked]);
  // 当前筛选（某账本）下的小计——对完整账户后核对各账本贡献
  const filteredCheckedSum = bookFilter === 'all' ? null : rows.reduce((s, r) => (checked.has(r.pid) ? s + r.amount : s), 0);

  const stmtTrim = stmt.trim();
  const stmtNum = stmtTrim === '' ? null : Number(stmtTrim);
  const stmtValid = stmtNum !== null && Number.isFinite(stmtNum);
  const stmtMinor = stmtValid ? toMinor(stmtNum, dec) : null;
  const diff = stmtMinor === null ? null : stmtMinor - checkedSum;

  function toggle(pid: string): void {
    setChecked((prev) => {
      const next = new Set(prev);
      if (next.has(pid)) next.delete(pid);
      else next.add(pid);
      return next;
    });
    setMsg(null);
  }

  async function complete(): Promise<void> {
    if (diff !== 0 || busy) return;
    setBusy(true);
    setErr(null);
    try {
      const checkedIds = allRows.filter((r) => checked.has(r.pid)).map((r) => r.pid);
      const uncheckedIds = allRows.filter((r) => !checked.has(r.pid)).map((r) => r.pid);
      await repo.setPostingsCleared(checkedIds, true);
      if (uncheckedIds.length) await repo.setPostingsCleared(uncheckedIds, false);
      await repo.addReconciliation({
        id: genId(),
        bookId: homeBookId, // 对账记录挂账户 home 账本（仅元数据；查询按 accountId）
        accountId,
        statementBalance: stmtMinor!,
        statementDate: stmtDate,
        completedAt: new Date().toISOString(),
      });
      await reload();
      setMsg(`已完成对账：「${selAccount?.name ?? '账户'}」余额与对账单 ${fmtMoney(stmtMinor!, curCode)} 相符，${checkedIds.length} 笔已核销。`);
    } finally {
      setBusy(false);
    }
  }

  /** 逃生口：差额查不出错时，记一笔盘盈盘亏调整把差额对平，并自动勾选这笔调整（落账户 home 账本）。 */
  async function adjust(): Promise<void> {
    if (diff === null || diff === 0 || busy) return;
    setBusy(true);
    setErr(null);
    try {
      let gl = accounts.find((a) => a.type === 'income' && a.name === GAIN_LOSS && a.bookId === homeBookId);
      if (!gl) {
        gl = await repo.addAccount({ id: genId(), bookId: homeBookId, name: GAIN_LOSS, type: 'income', parentId: null, currency: 'CNY', archived: false });
      }
      const entry = adjustBalanceEntry(
        { bookId: homeBookId, date: stmtDate, accountId, currentBalance: checkedSum, targetValue: stmtMinor!, counterAccountId: gl.id, currency: curCode, note: '对账盘盈盘亏调整' },
        genId,
      );
      await repo.addTransaction(entry);
      const adjPosting = entry.postings.find((p) => p.accountId === accountId)!;
      setChecked((prev) => new Set(prev).add(adjPosting.id));
      await reload();
      setMsg('已记盘盈盘亏调整并勾选，差额已对平，可完成对账。');
    } catch (e) {
      setErr(e instanceof Error ? e.message : '调整失败');
    } finally {
      setBusy(false);
    }
  }

  /** 内联补录一笔漏记的收支（落对账账户 + 用户所选账本的收支分类），并自动勾选。 */
  async function addMissing(): Promise<void> {
    if (busy) return;
    setErr(null);
    const major = Number(aAmount);
    if (!Number.isFinite(major) || major <= 0) {
      setErr('请输入有效的补录金额');
      return;
    }
    if (!aBook) {
      setErr('请选择补录归属账本');
      return;
    }
    if (!effACat) {
      setErr(`「${bookName(aBook)}」没有${aKind === 'income' ? '收入' : '支出'}分类，先去该账本「账户」页加一个`);
      return;
    }
    setBusy(true);
    try {
      const entry = expandEntry(
        { kind: aKind, bookId: aBook, date: aDate, amount: toMinor(major, dec), currency: curCode, payee: '对账补录', accountId, categoryId: effACat },
        genId,
      );
      await repo.addTransaction(entry);
      const newPosting = entry.postings.find((p) => p.accountId === accountId)!;
      setChecked((prev) => new Set(prev).add(newPosting.id));
      setAAmount('');
      setAddOpen(false);
      await reload();
      setMsg('已补录一笔并自动勾选。');
    } catch (e) {
      setErr(e instanceof Error ? e.message : '补录失败');
    } finally {
      setBusy(false);
    }
  }

  /** 内联删除一笔（重复 / 错记）——整笔交易软删，保留其余勾选进度。 */
  async function removeRow(row: { txnId: string; cleared: boolean }): Promise<void> {
    if (busy) return;
    const warn = row.cleared
      ? '这笔已核销，删除会影响已完成的对账记录。确定删除整笔交易？'
      : '删除整笔交易（含其对方分录）？此操作不可撤销。';
    if (!confirm(warn)) return;
    setBusy(true);
    setErr(null);
    try {
      await repo.softDeleteTransaction(row.txnId);
      await reload();
      setMsg('已删除一笔。');
    } catch (e) {
      setErr(e instanceof Error ? e.message : '删除失败');
    } finally {
      setBusy(false);
    }
  }

  if (recAccounts.length === 0) {
    return (
      <>
        <div className="main-head">
          <h2>对账</h2>
        </div>
        <div className="card">
          <p className="muted">还没有资产/负债账户可对账。先到某账本「账户」页添加。</p>
        </div>
      </>
    );
  }

  return (
    <>
      <div className="main-head">
        <h2>对账</h2>
        <span className="muted">勾对式 · 按账户跨账本</span>
      </div>

      <div className="card">
        <div className="rec-setup">
          <label>
            对账账户
            <select value={accountId} onChange={(e) => selectAccount(e.target.value)}>
              {recAccounts.map((a) => {
                const n = unclearedCount(allTxns, a.id);
                return (
                  <option key={a.id} value={a.id}>
                    {a.name} · {a.global ? '全局共享' : bookName(a.bookId)}
                    {n > 0 ? ` · ${n} 待核销` : ' · 已全核销'}
                  </option>
                );
              })}
            </select>
          </label>
          <label>
            对账单余额（元）
            <input
              inputMode="decimal"
              placeholder="如 4970.00；负债欠款记负"
              value={stmt}
              onChange={(e) => {
                setStmt(e.target.value);
                setMsg(null);
              }}
            />
          </label>
          <label>
            对账截止日
            <input type="date" value={stmtDate} onChange={(e) => setStmtDate(e.target.value)} />
          </label>
        </div>
        <div className="rec-hint muted small">
          账户当前余额 {fmtMoney(currentBalance, curCode)}
          {(() => {
            const n = unclearedCount(allTxns, accountId);
            return n > 0 ? <> · {n} 笔待核销</> : <> · 已全部核销 ✓</>;
          })()}
          {selAccount?.global && <> · 全局共享（流水来自各账本）</>}
          {lastRec && <> · 上次对账 {lastRec.statementDate}（{fmtMoney(lastRec.statementBalance, curCode)}）</>}
        </div>
      </div>

      <div className="card">
        <h3>
          流水勾对 <span className="mini">{rows.length} 笔{bookFilter !== 'all' ? ` · 本账本已勾选 ${fmtMoney(filteredCheckedSum ?? 0, curCode)}` : ''}</span>
        </h3>
        {rowBooks.length > 1 && (
          <div className="rec-bookfilter">
            <button className={`chip${bookFilter === 'all' ? ' on' : ''}`} onClick={() => setBookFilter('all')}>
              全部账本
            </button>
            {rowBooks.map((bid) => (
              <button key={bid} className={`chip${bookFilter === bid ? ' on' : ''}`} onClick={() => setBookFilter(bid)}>
                {bookName(bid)}
              </button>
            ))}
          </div>
        )}
        {rows.length === 0 ? (
          <p className="muted">该账户暂无流水。</p>
        ) : (
          <div className="rec-list">
            {rows.map((r) => (
              <label className={`rec-row${checked.has(r.pid) ? ' on' : ''}`} key={r.pid}>
                <input type="checkbox" checked={checked.has(r.pid)} onChange={() => toggle(r.pid)} />
                <span className="rec-date">{r.date}</span>
                <span className="rec-title">
                  {r.title}
                  {rowBooks.length > 1 && <span className="chip"> {bookName(r.bookId)}</span>}
                </span>
                <span className={`rec-amt ${r.amount < 0 ? 'neg' : 'pos'}`}>{fmtMoney(r.amount, curCode)}</span>
                <button
                  className="del rec-del"
                  title="删除这笔（错记/重复）"
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    void removeRow(r);
                  }}
                  disabled={busy}
                >
                  ×
                </button>
              </label>
            ))}
          </div>
        )}
        <div className="rec-add">
          {!addOpen ? (
            <button
              className="lnk"
              onClick={() => {
                setAddOpen(true);
                setADate(stmtDate);
                setABook(selAccount?.global ? (liveBooks[0]?.id ?? '') : homeBookId);
                setErr(null);
              }}
            >
              ＋ 补录一笔（对账单上有、账里漏记的款项）
            </button>
          ) : (
            <div className="rec-add-form">
              <div className="qgrid">
                <label>
                  归属账本
                  <select value={aBook} onChange={(e) => setABook(e.target.value)} disabled={addBookOptions.length <= 1}>
                    {addBookOptions.map((b) => (
                      <option key={b.id} value={b.id}>
                        {b.name}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  类型
                  <select value={aKind} onChange={(e) => setAKind(e.target.value as AddKind)}>
                    <option value="expense">支出</option>
                    <option value="income">收入</option>
                  </select>
                </label>
                <label>
                  金额（{currencyDef(curCode).symbol}）
                  <input inputMode="decimal" value={aAmount} onChange={(e) => setAAmount(e.target.value)} placeholder="0.00" />
                </label>
                <label>
                  分类
                  <select value={effACat} onChange={(e) => setACatId(e.target.value)}>
                    {addCats.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.name}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  日期
                  <input type="date" value={aDate} onChange={(e) => setADate(e.target.value)} />
                </label>
              </div>
              <div className="arow-btns" style={{ marginTop: 8 }}>
                <button className="btn btn-primary" onClick={() => void addMissing()} disabled={busy}>
                  补录并勾选
                </button>
                <button className="lnk" onClick={() => setAddOpen(false)}>
                  取消
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="card rec-foot">
        <div className="rec-tally">
          <span>已勾选合计 <b>{fmtMoney(checkedSum, curCode)}</b></span>
          <span className={`rec-diff${diff === 0 ? ' ok' : ''}`}>
            差额 <b>{diff === null ? '—' : fmtMoney(diff, curCode)}</b>
          </span>
        </div>
        <div className="rec-actions">
          {diff !== null && diff !== 0 && (
            <button className="btn" onClick={() => void adjust()} disabled={busy}>
              记盘盈盘亏调整 {fmtMoney(diff, curCode)}
            </button>
          )}
          <button className="btn btn-primary" onClick={() => void complete()} disabled={busy || diff !== 0}>
            完成对账
          </button>
        </div>
        {!stmtValid && stmtTrim !== '' && <p className="form-err">对账单余额需为数字</p>}
        {err && <p className="form-err">{err}</p>}
        {msg && <p className="rec-ok">{msg}</p>}
      </div>
    </>
  );
}
