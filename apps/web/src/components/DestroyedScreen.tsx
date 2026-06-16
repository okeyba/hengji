import { useState } from 'react';
import { restartAfterDestroy } from '@app/store/crypto';

/**
 * 销毁终态屏（gate='destroyed'，由 sentinel 触发）。诚实告知：数据已按安全设置永久销毁、密文墓碑不可解；
 * 可恢复的数据只在之前导出的备份里（显路径）。给一个「清空并从零开始」出路（否则永久卡此屏要手动改文件）。
 * 口令早已忘/被用尽，无解锁入口——这是与「数据损坏(可重试)」「芯片不可用(请重启)」彻底分开的第四态。
 */
export default function DestroyedScreen({
  backupPath,
  onRestarted,
}: {
  backupPath: string | null;
  onRestarted: () => Promise<void>;
}) {
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function restart(): Promise<void> {
    if (busy) return;
    setBusy(true);
    setErr(null);
    try {
      await restartAfterDestroy();
      await onRestarted(); // App 复位 → 开全新空明文库 → 进主界面
    } catch (e) {
      setErr(String(e));
      setBusy(false);
    }
  }

  return (
    <div className="lock-screen">
      <div className="lock-card destroyed-card">
        <div className="destroyed-mark">🔥</div>
        <h2 className="destroyed-title">数据已按你的安全设置销毁</h2>
        <p className="lock-sub">连续输错密码达到上限，封装密钥已从安全芯片删除。</p>
        <div className="destroyed-body small">
          <p>本机这份数据已<strong>永久且不可恢复</strong>地销毁——留下的密文副本没有密钥、无法再解开。</p>
          {backupPath ? (
            <p>
              可恢复的数据只在你此前导出的<strong>未加密备份</strong>里：
              <br />
              <code className="destroyed-path">{backupPath}</code>
            </p>
          ) : (
            <p>没有找到备份记录——若你曾导出过备份，请用那份文件恢复。</p>
          )}
        </div>

        {!confirming ? (
          <button className="btn destroyed-restart" onClick={() => setConfirming(true)}>
            清空并从零开始
          </button>
        ) : (
          <div className="destroyed-confirm">
            <p className="small">这会清掉销毁记录与密文墓碑，建一个全新的空账本。确定？</p>
            <div className="sec-form-btns">
              <button className="btn danger-btn" disabled={busy} onClick={() => void restart()}>
                {busy ? '处理中…' : '确定，从零开始'}
              </button>
              <button className="nb-cancel" disabled={busy} onClick={() => setConfirming(false)}>
                取消
              </button>
            </div>
          </div>
        )}
        {err && <p className="lock-err">{err}</p>}
      </div>
    </div>
  );
}
