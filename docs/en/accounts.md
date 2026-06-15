[← Back to manual index](README.md)

# Accounts & Global Sharing

The first step toward good bookkeeping is getting your "money pouches" and "where the money goes" straight. This chapter covers how to create and edit accounts, and how to let one real-world wallet serve both your business book and your personal book at once.

**Applies to**: everyone using Hengji / anyone just getting their accounts organized, or any sole proprietor who wants to keep business and personal money clearly separated.

---

## Accounts actually come in five kinds

In Hengji, "Account" is a big bucket that holds two kinds of things: real cash wallets, and the comings and goings of money. There are five kinds in total, all visible on the **Accounts** page:

| Kind | Name in Hengji | Examples | Has a balance? |
| --- | --- | --- | --- |
| Asset account | Asset account | Cash, China Merchants Bank card, Alipay, WeChat Wallet | Yes |
| Liability account | Liability account | Credit card, Huabei, money borrowed from others | Yes |
| Income category | Income category | Salary, operating revenue | Not shown |
| Expense category | Expense category | Dining, transport, cost of goods purchased | Not shown |
| Equity | Equity | Opening balance, etc. | Not shown |

You only need to remember one sentence: **assets/liabilities are "where the money is," income/expense are "where the money comes from and where it goes."** When you log an entry, you pick one wallet (asset/liability) and one category (income/expense), and Hengji computes the rest automatically.

> **Under the hood (skip if it doesn't click)**: Hengji is built on double-entry bookkeeping, where every entry is balanced between debit and credit. But day to day you just "log an entry," and the underlying journal lines are generated automatically. These five kinds on the Accounts page are exactly the five top-level account classes of double-entry bookkeeping.

---

## Creating a new account

1. Go to the **Accounts** page on the left, and scroll all the way down to the "Add account / category" card.
2. Fill in the **name** (e.g. "China Merchants Bank," "Dining").
3. Pick the **kind** (one of the five above); a gray hint below tells you what goes in that kind.
4. Click "Add."

A few common situations:

- You can't have two accounts with the same name under the same kind; a duplicate name prompts you to pick a different one.
- Once created, asset/liability accounts immediately show their current balance on the **Accounts** page (0 at first).
- Want to give an account a starting balance? Don't fill it in here — after creating the account, go to **Transactions** and log an entry, using something like "opening balance" to put the money in (see [Transactions & bookkeeping](recording.md)).

---

## Rename, archive, restore

Every account has buttons on its right:

- **Rename**: click "Rename," the name turns into an editable input box; press Enter or click "Save" when done; press Esc or click "Cancel" to discard.
- **Archive**: click "Archive." Archiving is not deletion — all historical transactions are kept, the account simply **no longer appears in the bookkeeping dropdowns** from then on, keeping the list tidy. Archiving an account that already has transaction records will give you a heads-up.
- **Restore**: an archived account carries a gray "Archived" tag and sorts to the back of the list; click "Restore" to make it usable again.

> Hengji has no "delete account." Once an account has had any transactions, deleting it would make the historical books fail to reconcile, so only archiving is offered.

---

## Letting one real wallet be shared across multiple books (global sharing)

This is Hengji's most practical trick, designed to cure a chronic sole-proprietor ailment: **one Alipay account that pays both supplier bills and grocery runs.**

Hengji recommends keeping two books — one for business, one for personal life (see [Books & splitting accounts](books.md)). But your Alipay and bank card are **the same real wallet**, with the money all mixed together. This is where "global sharing" comes in:

1. On the **Accounts** page, find that real-wallet account (e.g. "Alipay"); it must be an **asset or liability** kind.
2. Click "**Set as shared**" on its right.
3. A gray "Globally shared" tag appears next to its name.

Once set as shared:

- This wallet can be selected when logging entries in **every book**, with the balance being the same one, synced across books in real time — pay a supplier bill from the business book, and the Alipay balance you see in the personal book drops accordingly.
- During reconciliation, this account is checked across all books together by "account," so nothing gets missed because of how the books are split (see [Reconciliation](reconciliation.md)).
- Don't want it shared anymore? Click "**Unshare**," and it reverts to belonging only to the current book.

**What should and shouldn't be shared?**

| Account | Recommendation | Why |
| --- | --- | --- |
| Alipay, WeChat, bank card, cash (mixed business/personal use) | Set as shared | Same real wallet; both books should see the same balance |
| A corporate account used only for business | Don't share | It serves only the business book; keeping it exclusive is clearer |
| Income/expense categories | Can't be shared | The share button only appears for asset/liability accounts |

> Tip: when creating a new account, if you pick the asset or liability kind, a "**Share with all books**" checkbox appears right below. You can tick it as you create the wallet, instead of coming back to set it afterward.

---

## Some accounts are "auto-managed" and can't be edited

If you've turned on the merchant advanced features (enabled in Settings) and used things like customer credit sales, supplier credit purchases, or inventory, Hengji automatically creates a batch of accounts for you, such as:

- **Accounts Receivable** (A/R — money customers owe you), and the per-customer "Accounts Receivable / So-and-so" split out automatically
- **Accounts Payable** (A/P — money you owe suppliers), and the per-supplier "Accounts Payable / So-and-so" split out
- **Inventory goods**, **Goods-in-transit purchasing cost**
- Operating revenue, platform commission, logistics fees, etc. (when using e-commerce documents)

These accounts show no rename/archive buttons on their right, only a gray "**Auto-managed**" label. The reasons are:

- They are claimed automatically by business processes by name. If you rename or archive one, the next time Hengji opens a document it can't find the original account and rebuilds a new one, splitting the balance in two and throwing the books into disarray.
- They are all "virtual" operating accounts, not the real money in your hand, so they **always belong only to the current book and can't be set as shared.**

You don't need to manage them — just keep opening documents, collecting payments, and purchasing stock as usual, and these accounts update themselves.

---

## Account currency (only with multi-currency on)

By default all accounts are in CNY, and you won't see a currency option.

If you've turned on **multi-currency** in Settings, and the account you're creating is an **asset or liability account**, the "Add account" card gains an extra "**Currency**" dropdown, letting you pick a currency for this wallet (e.g. a USD account). Note:

- Only asset/liability accounts can pick a currency; income/expense categories don't need one.
- An account in a non-CNY currency shows a small currency tag next to its name (e.g. `USD`).
- Currency is set when the account is created; with multi-currency off, everything runs in CNY.

For how multi-currency handles exchange and converting total value, see [Multi-currency](multi-currency.md).

---

## Related chapters

- [Books & splitting accounts](books.md) — why you'd separate the business book from the personal book
- [Transactions & bookkeeping](recording.md) — how to log each entry
- [Reconciliation](reconciliation.md) — verifying real balances across books by account
- [Multi-currency](multi-currency.md) — foreign-currency accounts and conversion
