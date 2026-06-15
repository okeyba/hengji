[← Back to handbook index](README.md)

# Overview · Global overview · Budgets

Understand your own money: every Book gets a one-page "Overview," all Books roll up into a single "Global overview," and a "Budget" lets you draw a red line on your spending.

**For**: everyone / any time you want to know at a glance "did I make money this month, how much is left, and have I overspent."

Hengji keeps your business books and personal books in separate [Books](books.md), yet they often share the same real-world account (one Alipay pays both for goods and for groceries). The three pieces covered here — a Book's overview, the cross-Book global overview, and budgets — are what let you switch freely between "viewing them separately" and "viewing them together."

---

## 1. Book overview: how is this Book doing right now

Open any Book and the left side defaults to "Overview." From top to bottom it has three sections.

### A row of number cards at the top

The largest card is this Book's **net amount**:

- Personal Books call it "Book net amount" — the assets minus liabilities of the accounts that belong exclusively to this Book.
- Business Books call it "Operating net amount" — it counts only what the business itself owns (inventory, equipment, debts, etc.) and leaves your personal wallet out of it.

Right next to it there may be an "**Available funds · globally shared**" card. This is the total of the real-world accounts you've marked as "global" (Alipay, bank cards — money used on both sides). It's listed separately and is *not* folded into the net amount above, so you can see both "what this business itself is worth" and "how much cash you actually have on hand." If you've never set up a global account, this card doesn't appear.

> Why keep them apart? Because a shared Alipay balance belongs to *you as a person*, not to any single business. Listing it on its own keeps the business's profit and loss from getting muddied by personal spending. For how to mark an account as global, see [Accounts](accounts.md).

The next three are this month's flows:

| Card | Meaning |
| --- | --- |
| Income this month | Total of all income this month |
| Expenses this month | Total of all expenses this month |
| Balance this month / Profit this month | Income minus expenses; personal Books call it "balance," business Books call it "profit" |

The top-right corner of the page also shows the current month. A business Book on cash basis carries a small "Cash basis" tag; if reconciliation has been done, it shows "Reconciled this period ✓" or "N accounts pending reconciliation."

### Middle: asset distribution pie chart + log an entry

On the left is the **asset distribution** pie chart, which draws each of this Book's own asset accounts into a ring sized by amount, lists each slice's name and balance beside it, and gives the total in the heading. Note: globally-shared accounts (that "available funds" figure) are *not* drawn into this pie — the pie contains only this Book's own assets.

On the right is the quick entry point for "log an entry," where day-to-day bookkeeping begins.

### Bottom: recent transactions

Lists this Book's 6 most recent transactions. When nothing has been recorded yet, it prompts "No transactions yet — start with 'Log an entry.'" To see them all, go to [Transactions](recording.md).

---

## 2. Global overview: see all Books together

At the very top of the left-hand Book list there's a "🧮 Global overview." Click it to step out of any single Book and see the sum of **all the Books in your name**.

The largest card is "**Total net worth**," with "(rollup of N Books)" in the heading. Its formula is straightforward:

```
Total net worth = Global funds (shared) + sum of each Book's own net amount
```

Below it, again, is "**Global funds · shared across all Books**" listed once on its own (those shared Alipay and bank-card accounts), along with this month's total income / total expenses / total balance.

> Global funds are counted only once. They don't belong to any single Book, so no matter how many Books share the same Alipay, the funds land in the single "Global funds" cell and are never double-counted. This is exactly the key to how Hengji "keeps mixed accounts straight."

Further down, each Book gets a **card** showing the Book's name, type, and net amount, along with its balance/profit this month and its transaction count. **Click any card to jump into that Book's overview** — it doubles as a quick shortcut.

> Archived Books are not counted in the global overview; but accounts you've marked as global always take part in the totals, unaffected even if the Book they were originally attached to has been archived.

---

## 3. Budgets: set a monthly red line for each spending category

In personal Books (or business Books before advanced features are turned on), the left side has "Budget." It helps you set a **monthly** ceiling per **expense category**, then uses a progress bar to show how far along you are.

### Add a budget

1. Go to the Book's "Budget" page and look at the "Add budget" section in the lower half.
2. In the "Category" dropdown, pick an expense category (e.g. "Dining," "Rent"). The dropdown only lists **expense categories that don't yet have a budget**; ones already set won't appear again.
3. Enter the amount in "Monthly limit (¥)," for example `1000`.
4. Click "Add." If no category is selected or the amount is invalid, it prompts "Please select a category and enter a valid limit."

### Watch your usage

Each budget is one row, showing "spent / limit" and a progress bar:

- When you're under, the progress bar fills proportionally (capped at 100%).
- When you've overspent, the number turns red with "· Over budget" appended, and the progress bar changes color too.

To delete a budget, click the "×" on the right of that row and confirm.

### How it works (just the gist)

- Budgets target **expense categories**, not real accounts — they govern "how much of this kind of money was spent," not "how much is left in a particular wallet."
- "Spent" is the total of everything recorded under that category this month; it accumulates automatically as you [log an entry](recording.md) normally, with no manual updating.
- Budgets are always reckoned in **Chinese yuan (¥)**. If you record an expense in a foreign currency, it's converted at the current exchange rate before being counted (even if the display currency you picked isn't yuan, the budget is still measured in yuan).

---

## Related

- [Books and account separation](books.md) — how to split business vs. personal books, create them, and archive them
- [Accounts](accounts.md) — real-world fund accounts, and which to mark as "globally shared"
- [Transactions](recording.md) — log an entry, view all transactions
