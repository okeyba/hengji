import { invoke } from '@tauri-apps/api/core';

/**
 * 自写 rusqlite + SQLCipher 桥的 JS adapter（替代 @tauri-apps/plugin-sql 的 Database）。
 * 形状与原 Database 对齐（select/execute/close + 新增 batch），让 TauriSqlRepository 的 SQL 全不动。
 * 经 IPC 调 Rust 的 db_* 命令。占位符 `$N` 仍由 Rust 侧翻成 `?N`。
 */
export class TauriDb {
  /** 打开本地库。key 给定则开 SQLCipher 加密库（阶段 2 用）；不给＝明文。 */
  static async open(path: string, key?: string): Promise<TauriDb> {
    await invoke('db_open', { path, key: key ?? null });
    return new TauriDb();
  }

  async select<T>(sql: string, params: unknown[] = []): Promise<T> {
    return invoke<T>('db_select', { sql, params });
  }

  async execute(sql: string, params: unknown[] = []): Promise<void> {
    await invoke('db_execute', { sql, params });
  }

  /** 多条写在一把事务里（原子）。 */
  async batch(stmts: Array<{ sql: string; params?: unknown[] }>): Promise<void> {
    await invoke('db_batch', { stmts: stmts.map((s) => ({ sql: s.sql, params: s.params ?? [] })) });
  }

  async close(): Promise<void> {
    await invoke('db_close');
  }
}
