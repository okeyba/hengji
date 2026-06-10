# 贡献指南

## 开发
前置：Node ≥ 24、pnpm ≥ 9
```sh
pnpm install
pnpm -r test
pnpm -r typecheck
```
- `core` 改动 **test-first**；新增 `Repository` 行为请加进共享契约（`packages/store/test/contract.ts`），内存与 SQLite 两实现都要过。
- TypeScript strict；金额一律整数最小单位（分）。
- 改动尽量小而聚焦，匹配现有风格。

## DCO 签名
本项目采用 Developer Certificate of Origin（DCO 1.1，全文见 https://developercertificate.org ）。每个提交需签名：
```sh
git commit -s -m "..."
```
`-s` 会在提交信息追加 `Signed-off-by: 你的名字 <邮箱>`，表示你有权提交这些代码、并同意以本项目许可证发布。

## 许可证
贡献即同意以 **Apache-2.0** 发布。
