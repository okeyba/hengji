import { confirmNative } from '@app/store/dialog';
import { isDesktop } from './db';

/**
 * 外发/破坏性操作确认：桌面走 tauri 原生 `ask()`（`window.confirm` 在 WebView2 是 no-op、会静默放行——
 * 真机实测：AI 认列/语音上云、清除 Key 等在无弹窗下直接执行），浏览器 demo 退回 `window.confirm`。
 * 返回 true=用户确认继续。
 *
 * 覆盖全 app 的确认点（AI 认列/语音上云、安全清空/移除密码、删交易/取消订单/作废/归档/删币种等）——
 * 桌面统一走原生框，杜绝 window.confirm no-op 静默放行破坏性/外发操作。
 */
export const confirmAsk = (message: string): Promise<boolean> =>
  isDesktop ? confirmNative(message) : Promise.resolve(window.confirm(message));
