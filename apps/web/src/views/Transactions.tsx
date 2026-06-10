import type { AppData } from '../App';
import TxnRow from '../components/TxnRow';

export default function Transactions({ data }: { data: AppData }) {
  const rows = data.txns;
  return (
    <>
      <div className="main-head">
        <h2>{data.book.name} · 流水</h2>
        <span className="muted">共 {rows.length} 笔</span>
      </div>
      <div className="card">
        {rows.length === 0 ? (
          <p className="muted">暂无交易</p>
        ) : (
          rows.map((t) => <TxnRow key={t.id} txn={t} data={data} deletable />)
        )}
      </div>
    </>
  );
}
