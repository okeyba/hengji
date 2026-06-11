import type { StoredTransaction } from '@app/store';
import type { AppData } from '../App';
import { describeTxn } from '../format';

export default function TxnRow({
  txn,
  data,
  deletable,
}: {
  txn: StoredTransaction;
  data: AppData;
  deletable?: boolean;
}) {
  const accountMap = new Map(data.accounts.map((a) => [a.id, a]));
  const v = describeTxn(txn, accountMap);
  return (
    <div className="tx">
      <div className="avatar">{v.emoji}</div>
      <div className="meta">
        <div className="t1">
          {v.title} {v.tags.includes('business') && <span className="chip">生意</span>}
        </div>
        <div className="t2">{v.sub}</div>
      </div>
      <div className={`amt ${v.tone}`}>{v.amountText}</div>
      {deletable && (
        <button
          className="del"
          title="删除"
          onClick={async () => {
            const reconciled = txn.postings.some((p) => p.cleared);
            const prompt = reconciled
              ? '这笔交易含已核销分录，删除会打散已完成的对账（届时需重新对账）。确定删除？'
              : '删除这笔交易？';
            if (confirm(prompt)) {
              await data.repo.softDeleteTransaction(txn.id);
              await data.reload();
            }
          }}
        >
          ×
        </button>
      )}
    </div>
  );
}
