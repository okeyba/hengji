[← Back to manual index](README.md)

# Monthly reconciliation · Match your in-app balance to your real account

At the end of each month, spend ten minutes confirming that the balance Hengji shows for an account matches the **real money** in your Alipay / bank card / cash box, down to the cent. Once they agree, the month's books are settled.

**For**: small-business users who have enabled "Merchant advanced features" / anyone who wants to verify their books at month-end.

Reconciliation solves a very real problem: as you log entries one by one, it's inevitable that some get missed, mis-entered, or double-counted. Over a month, the balance written in your books may not equal the actual balance in your card. Reconciliation uses the **real balance on your statement** as the yardstick, ticks through every entry, and surfaces the discrepancy so you can square it up.

> Reconciliation is an advanced feature, hidden by default. First go to [Settings](settings.md) → turn on "Merchant advanced features", and the "✓ Reconciliation" entry will appear in the left sidebar.

---

## Where it lives

Reconciliation is a **global entry point**, at the very bottom of the left sidebar (it doesn't belong to any single book). Click "✓ Reconciliation" to enter, and a single page lets you reconcile every account — including real funds accounts shared across books. This is key: your Alipay pays for goods (business book) and also buys groceries (personal book), but it has only one real balance. Reconciliation works **per account**, pulling together the transactions scattered across books. For how to keep business and personal accounts separate, see [Books & splitting accounts](books.md).

---

## Reconciling one account, in five steps

1. **Pick an account.** The "Reconciliation account" dropdown at the top lists all asset / liability accounts. Each one is tagged with which book it belongs to (or "globally-shared"), along with how many entries are still "uncleared". Pick one first, e.g. "Alipay".
2. **Enter the statement balance.** Open the Alipay app to see the current balance, and copy it into "Statement balance (¥)". For liability accounts (e.g. credit card debt), enter a negative number.
3. **Enter the statement cutoff date.** Defaults to today; usually no need to change it.
4. **Tick entries one by one.** Below, "Transaction matching" lists every transaction for this account, sorted by date. Tick them off against the statement one by one — anything that's both in your books and on the statement gets a tick.
5. **Watch the difference, get it to 0, finish.** The bottom of the page shows "Ticked total" and "Difference" in real time. Difference = statement balance − ticked total. When the difference reaches **0** (it turns green), click "Finish reconciliation".

After finishing, the entries you ticked this time get marked "cleared", so you won't have to touch them at the next reconciliation — the list only watches for newly added entries.

---

## What to do when the difference won't balance

If the difference ≠ 0, your books and reality disagree. There are three common cases, all of which you can resolve right on the reconciliation page without navigating away and losing your ticking progress:

### 1. On the statement but missing from your books → add a backfill entry

Click "＋ Add a backfill entry" below the transaction list. Fill in the owning book, type (income / expense), amount, category, and date, then click "Backfill and tick". This entry is immediately logged into your books and automatically ticked, narrowing the difference accordingly.

> When backfilling into a globally-shared account, you can choose which book it lands in (groceries to the personal book, restocking to the business book); a book-specific account can only land in its own book.

### 2. In your books but double-counted or mis-entered → delete it

Every transaction row has a "×" on its right side; click it to delete the whole transaction (along with its counterpart entry). If this entry has already been cleared, you'll get an extra confirmation prompt — deleting it will affect a previously completed reconciliation record. **Deletion cannot be undone**, so look carefully before deleting.

### 3. Can't figure out where the difference is → one-click balancing (the escape hatch)

Sometimes you're just off by a few cents and, search as you might, you can't find it. When the difference ≠ 0, a "Record gain/loss adjustment of ××" button appears at the bottom. Click it: Hengji automatically logs a "gain/loss" adjustment that exactly squares the difference and ticks it automatically. The difference goes to zero, and you can finish the reconciliation.

This is a respectable way to wrap up, not a cop-out — in cash-based businesses small odd amounts are unavoidable, and "gain/loss" is exactly the accounting account meant to absorb such small differences. But if the difference is a large sum, don't rush to balance it; first go back to [Transactions](recording.md) and check whether you missed a big entry.

---

## When one account spans multiple books

If you picked a globally-shared account (e.g. Alipay serving both the business book and the personal book), a row of book filter tags appears above the transaction matching area: "All books", "Daily life", "Corner shop", and so on.

- These filters **only affect which transactions you see**; they don't affect the account's overall reconciliation — the difference is always computed over **all** transactions for this account.
- Click a book tag, and the heading shows "Ticked in this book ××", making it easy to verify separately how much each book contributed.
- Each transaction row is also tagged with which book it belongs to, so you can tell at a glance whether an entry is business or personal.

For why an account can be globally shared and how to set it up, see [Accounts](accounts.md).

---

## Set a reconciliation reminder so you don't forget

Go to [Settings](settings.md) → "Reconciliation reminder" to set a monthly reconciliation day:

| Option | Description |
| --- | --- |
| Reconciliation day | Reminder off / last day of each month / the 1st–28th of each month |
| Advance reminder | On the day / 1, 2, 3, 5, or 7 days early |

Once set, as the reconciliation day approaches a reminder pops up at the top of the book, with a one-click "Go reconcile". A thoughtful touch: **if every account in a book has already been cleared, you won't be bothered** — books that are already reconciled stay quiet.

---

## Under the hood: what happens behind the ticking

You're just ticking boxes, but underneath a rigorous double-entry bookkeeping system is running:

- When you "log an entry", each transaction generates a pair of debit/credit entries (where the money came from, where it went), so the book balances by nature.
- "Cleared" is a flag attached to one of an account's entries, meaning "this one has been matched to the real account".
- **Statement balance − sum of ticked entries = difference.** A difference of 0 proves that the cleared transactions in your books add up to exactly the real account's balance.
- When you finish the reconciliation, ticked entries are marked cleared and unticked ones marked uncleared, and a reconciliation record is saved (account + statement balance + date) as the baseline for the next reconciliation.

You don't need to understand any of this to use it — tick, watch the difference, finish, and that's enough.

---

Related: [Accounts](accounts.md) · [Books & splitting accounts](books.md) · [Transactions](recording.md) · [Settings](settings.md)
