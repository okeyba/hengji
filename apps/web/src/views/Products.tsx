import { useEffect, useState } from 'react';
import { fromMinor, toMinor } from '@app/core';
import type { StoredProduct } from '@app/store';
import type { AppData } from '../App';
import { genId } from '../db';
import { fmtMoney } from '../format';

export default function Products({ data }: { data: AppData }) {
  const { repo, book } = data;
  const [list, setList] = useState<StoredProduct[]>([]);
  const [name, setName] = useState('');
  const [cost, setCost] = useState('');
  const [sale, setSale] = useState('');
  const [unit, setUnit] = useState('');
  const [isStock, setIsStock] = useState(false);
  const [dropship, setDropship] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const [editId, setEditId] = useState<string | null>(null);
  const [eName, setEName] = useState('');
  const [eCost, setECost] = useState('');
  const [eSale, setESale] = useState('');
  const [eUnit, setEUnit] = useState('');
  const [eStock, setEStock] = useState(false);
  const [eDropship, setEDropship] = useState(false);

  async function refresh(): Promise<void> {
    setList(await repo.listProducts({ bookId: book.id, includeArchived: true }));
  }
  useEffect(() => {
    void refresh();
  }, [book.id]);

  const rows = list.slice().sort((a, b) => Number(a.archived) - Number(b.archived));

  function parseMoney(s: string): number | null {
    if (s.trim() === '') return 0;
    const n = Number(s);
    return Number.isFinite(n) && n >= 0 ? toMinor(n) : null;
  }

  async function add(): Promise<void> {
    setErr(null);
    const nm = name.trim();
    if (!nm) {
      setErr('请输入商品名称');
      return;
    }
    if (list.some((p) => p.name === nm)) {
      setErr(`已有同名商品「${nm}」`);
      return;
    }
    const c = parseMoney(cost);
    const s = parseMoney(sale);
    if (c === null || s === null) {
      setErr('进价/售价需为非负数');
      return;
    }
    await repo.addProduct({ id: genId(), bookId: book.id, name: nm, costPrice: c, salePrice: s, isStock, dropship, unit: unit.trim(), archived: false });
    setName('');
    setCost('');
    setSale('');
    setUnit('');
    setIsStock(false);
    setDropship(false);
    await refresh();
  }

  function openEdit(p: StoredProduct): void {
    setEditId(p.id);
    setEName(p.name);
    setECost(String(fromMinor(p.costPrice)));
    setESale(String(fromMinor(p.salePrice)));
    setEUnit(p.unit);
    setEStock(p.isStock);
    setEDropship(p.dropship);
    setErr(null);
  }

  async function saveEdit(p: StoredProduct): Promise<void> {
    setErr(null);
    const nm = eName.trim();
    if (!nm) {
      setErr('请输入商品名称');
      return;
    }
    if (list.some((x) => x.id !== p.id && x.name === nm)) {
      setErr(`已有同名商品「${nm}」`);
      return;
    }
    const c = parseMoney(eCost);
    const s = parseMoney(eSale);
    if (c === null || s === null) {
      setErr('进价/售价需为非负数');
      return;
    }
    await repo.updateProduct(p.id, { name: nm, costPrice: c, salePrice: s, isStock: eStock, dropship: eDropship, unit: eUnit.trim() });
    setEditId(null);
    await refresh();
  }

  async function toggleArchive(p: StoredProduct): Promise<void> {
    if (!p.archived && !confirm(`归档「${p.name}」？归档后不在开单选商品中出现，可随时恢复。`)) return;
    await repo.updateProduct(p.id, { archived: !p.archived });
    await refresh();
  }

  return (
    <>
      <div className="main-head">
        <h2>{book.name} · 商品</h2>
      </div>

      <div className="card">
        {rows.length === 0 && <p className="muted">还没有商品，先在下面添加；开单时可直接选商品自动带价。</p>}
        {rows.map((p) => (
          <div className="brow" key={p.id}>
            {editId === p.id ? (
              <>
                <div className="qgrid">
                  <label>
                    名称
                    <input value={eName} onChange={(e) => setEName(e.target.value)} />
                  </label>
                  <label>
                    单位
                    <input placeholder="个 / kg" value={eUnit} onChange={(e) => setEUnit(e.target.value)} />
                  </label>
                  <label>
                    进价（元）
                    <input inputMode="decimal" value={eCost} onChange={(e) => setECost(e.target.value)} />
                  </label>
                  <label>
                    售价（元）
                    <input inputMode="decimal" value={eSale} onChange={(e) => setESale(e.target.value)} />
                  </label>
                </div>
                <label className="chkline">
                  <input type="checkbox" checked={eStock} onChange={(e) => { setEStock(e.target.checked); if (e.target.checked) setEDropship(false); }} /> 库存品（先囤货、移动加权均价）
                </label>
                <label className="chkline">
                  <input type="checkbox" checked={eDropship} onChange={(e) => { setEDropship(e.target.checked); if (e.target.checked) setEStock(false); }} /> 代采品（接单后为此单采购、成本直挂订单）
                </label>
                {err && <p className="form-err" style={{ marginTop: 8 }}>{err}</p>}
                <div className="arow-btns" style={{ marginTop: 8 }}>
                  <button className="lnk" onClick={() => void saveEdit(p)}>
                    保存
                  </button>
                  <button className="lnk" onClick={() => setEditId(null)}>
                    取消
                  </button>
                </div>
              </>
            ) : (
              <div className="bhead">
                <span className={`bname${p.archived ? ' muted' : ''}`}>
                  {p.name}
                  {p.unit && <span className="muted"> / {p.unit}</span>}
                  {p.isStock && <span className="chip"> 库存品</span>}
                  {p.dropship && <span className="chip"> 代采品</span>}
                  {p.archived && <span className="chip"> 已归档</span>}
                </span>
                <span className="bnum">
                  售 {fmtMoney(p.salePrice)} <span className="muted">· 进 {fmtMoney(p.costPrice)}</span>
                </span>
                <div className="arow-btns">
                  <button className="lnk" onClick={() => openEdit(p)}>
                    编辑
                  </button>
                  <button className={`lnk${p.archived ? '' : ' danger'}`} onClick={() => void toggleArchive(p)}>
                    {p.archived ? '恢复' : '归档'}
                  </button>
                </div>
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="card">
        <h3>新增商品</h3>
        <div className="qgrid">
          <label>
            名称
            <input placeholder="商品名称" value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          <label>
            单位（可选）
            <input placeholder="个 / kg" value={unit} onChange={(e) => setUnit(e.target.value)} />
          </label>
          <label>
            进价（元，可选）
            <input inputMode="decimal" placeholder="0.00" value={cost} onChange={(e) => setCost(e.target.value)} />
          </label>
          <label>
            售价（元，可选）
            <input inputMode="decimal" placeholder="0.00" value={sale} onChange={(e) => setSale(e.target.value)} />
          </label>
        </div>
        <label className="chkline">
          <input type="checkbox" checked={isStock} onChange={(e) => { setIsStock(e.target.checked); if (e.target.checked) setDropship(false); }} /> 库存品（先囤货、移动加权均价）
        </label>
        <label className="chkline">
          <input type="checkbox" checked={dropship} onChange={(e) => { setDropship(e.target.checked); if (e.target.checked) setIsStock(false); }} /> 代采品（接单后为此单采购、成本直挂订单）
        </label>
        {!editId && err && <p className="form-err" style={{ marginTop: 8 }}>{err}</p>}
        <button className="btn btn-primary" style={{ marginTop: 8 }} onClick={() => void add()}>
          添加
        </button>
      </div>
    </>
  );
}
