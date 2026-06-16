import { useEffect, useState } from 'react';
import { isCryptoError, securityStatus, unlock } from '@app/store/crypto';
import type { CryptoError, SecurityStatus } from '@app/store/crypto';

/**
 * 解锁屏（仅桌面已加密时，bootstrap 门在开库前渲染）。
 * 口令由用户在原生输入框输入，解锁成功（DEK 已存 Rust 侧）后回调 onUnlocked 让 App 开库。
 * 失败按分流（§5）：口令错可重试；数据损坏 / 芯片不可用 / 芯片锁定走专门提示、不诱导反复试错。
 * 开了销毁时显「再错 N 次永久销毁、已错 M 次」+ 备份超期大声警示；第 N 次触发销毁则回调 onDestroyed → 终态屏。
 */
export default function UnlockScreen({
  onUnlocked,
  onDestroyed,
}: {
  onUnlocked: () => Promise<void>;
  onDestroyed: () => void;
}) {
  const [pw, setPw] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<CryptoError | null>(null);
  const [status, setStatus] = useState<SecurityStatus | null>(null);

  async function refresh(): Promise<void> {
    try {
      setStatus(await securityStatus());
    } catch {
      /* 忽略 */
    }
  }
  useEffect(() => {
    void refresh();
  }, []);

  async function submit(): Promise<void> {
    if (busy || !pw) return;
    setBusy(true);
    setErr(null);
    try {
      await unlock(pw);
      setPw('');
      await onUnlocked(); // 开库 + 进入主界面（由 App 处理）
    } catch (e) {
      const ce = isCryptoError(e) ? e : { class: 'Internal' as const, code: 0, message: String(e) };
      if (ce.class === 'Destroyed') {
        onDestroyed(); // 第 N 次错口令触发了销毁 → App 切终态屏
        return;
      }
      setErr(ce);
      setBusy(false);
      void refresh(); // 刷新已错次数（销毁倒计时）
    }
  }

  // 开了销毁的倒计时 + 备份超期警示
  const remaining =
    status?.destroy_enabled ? Math.max(0, status.destroy_threshold - status.fail_count) : null;
  const backupDays =
    status?.last_backup_unix != null ? Math.floor(Date.now() / 1000 - status.last_backup_unix) / 86400 : null;

  return (
    <div className="lock-screen">
      <div className="lock-card">
        <div className="lock-brand">
          <span className="mark">衡</span> 衡记
        </div>
        <p className="lock-sub">本账本已加密，请输入密码解锁。</p>
        <input
          type="password"
          className="lock-input"
          placeholder="密码"
          value={pw}
          autoFocus
          disabled={busy}
          onChange={(e) => setPw(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') void submit();
          }}
        />
        <button className="btn btn-primary lock-btn" disabled={busy || !pw} onClick={() => void submit()}>
          {busy ? '解锁中…' : '解锁'}
        </button>
        {err && <LockError err={err} />}
        {remaining !== null && (
          <div className="lock-destroy-warn small">
            ⚠ 已开启「错 {status!.destroy_threshold} 次销毁」：再错 <strong>{remaining}</strong> 次将
            <strong>永久销毁全部账本</strong>、本机无法找回。
            {backupDays !== null && backupDays >= 7 && (
              <div className="lock-stale">上次备份 {Math.floor(backupDays)} 天前——销毁将丢失此后的所有数据。</div>
            )}
          </div>
        )}
        <p className="lock-foot muted small">忘记密码无法找回：钥匙锁在本机安全芯片里，没有后门。请确保你有备份。</p>
      </div>
    </div>
  );
}

function LockError({ err }: { err: CryptoError }) {
  // 显示芯片返回的原始 HRESULT，便于诊断/支持（§5「持 live code 细化」）。
  const code = err.code ? `（错误码 0x${err.code.toString(16)}）` : '';
  if (err.class === 'WrongPassword') {
    return <p className="lock-err">密码错误，请重试。每次输错都会消耗安全芯片的一次防猜测额度，连错过多会被临时锁定。</p>;
  }
  if (err.class === 'Locked') {
    return (
      <div className="lock-err lock-err-block">
        <strong>密码连续输错，安全芯片已临时锁定</strong>
        <p className="small">
          这是芯片的防爆破保护。请隔一段时间后用<strong>正确</strong>密码再试——一次成功即解除；期间别继续试错，重启不一定能解，更<strong>不要清空 TPM</strong>（会永久毁掉钥匙）。{code}
        </p>
      </div>
    );
  }
  if (err.class === 'Corrupt') {
    return (
      <div className="lock-err lock-err-block">
        <strong>数据可能已损坏</strong>
        <p className="small">加密信封或数据库文件读取失败，重复尝试无济于事。如有备份请从备份恢复。{code}</p>
      </div>
    );
  }
  if (err.class === 'ChipUnavailable') {
    return (
      <div className="lock-err lock-err-block">
        <strong>暂时无法访问安全芯片</strong>
        <p className="small">多数情况是临时的——请重启电脑后再试一次。{code}</p>
        <p className="small">
          但如果你曾清空过 TPM、更换主板/CPU，或把数据文件拷到了别的电脑，则封装钥匙已永久失效、本机再也无法解开——请改用备份恢复。
        </p>
      </div>
    );
  }
  return <p className="lock-err">解锁失败：{err.message}{code}</p>;
}
