import type { Minor } from './types';

/** 金额必须是整数最小单位（分），拒绝浮点。 */
export function assertMinor(amount: number, label = 'amount'): asserts amount is Minor {
  if (!Number.isInteger(amount)) {
    throw new Error(`${label} 必须是整数最小单位（分），got ${amount}`);
  }
}

/** 主单位小数（如 30.5 元）→ 最小单位（3050 分）。scale 默认 2。 */
export function toMinor(major: number, scale = 2): Minor {
  return Math.round(major * 10 ** scale);
}

/** 最小单位 → 主单位小数。 */
export function fromMinor(minor: Minor, scale = 2): number {
  return minor / 10 ** scale;
}
