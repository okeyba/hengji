[← Back to manual index](README.md)

# Logging an entry & Transactions

Writing down every bit of money that comes in and goes out is the thing you'll do most often in Hengji. A few taps, type an amount, and you're done.

**Who it's for**: everyone who uses Hengji / any time you collect money, pay money, or shuffle money between your own accounts.

---

## Logging an entry: three types

Open **Log an entry**, and you'll see three tabs at the top: **Expense**, **Income**, **Transfer**. Pick the right type first, then fill in the rest.

- **Expense**—money went out. Buying stock, groceries, paying rent, paying employee wages.
- **Income**—money came in. Sales receipts, a platform payout, someone paying you back.
- **Transfer**—the money neither grew nor shrank; it just moved from one of your accounts to another. For example, withdrawing from Alipay to a bank card, or topping up your WeChat wallet. A transfer is neither earning nor spending, so don't record it as income or an expense.

### How to fill in Expense / Income

1. Pick **Expense** or **Income**.
2. Enter the **amount** (positive numbers only, e.g. `38.50`).
3. Pick a **date** (defaults to today; tap to change it).
4. Pick a **category** (for expenses) or a **source** (for income)—e.g. dining, transport, cost of goods for expenses; operating revenue, wages, etc. for income.
5. Pick an **account**—which real account this money went out of / came into, e.g. "WeChat" or "CMB card".
6. **Note** (optional)—type a merchant name or a quick phrase so you can recognize later what this entry was.
7. Tap **Save**. When you see "Entry logged ✓", you're done.

> The symbol in the amount field's label (e.g. `¥`) changes with the currency of the account you pick. See "Foreign-exchange" below for details.

### How to fill in a Transfer

1. Pick **Transfer**.
2. Enter the **amount**.
3. Pick **From** (which account the money leaves) and **To** (which account it enters)—the two can't be the same, or you'll get a "Source and destination accounts can't be the same" warning.
4. Note is optional; tap **Save**.

---

## Where do categories and accounts come from?

The categories and accounts you can pick in **Log an entry** are ones you've set up ahead of time:

- Where your real money sits, and who you owe money to—managed under [Accounts](accounts.md), shared across all of Hengji.
- Which book this entry lands in (your business book or your personal book)—it depends on which book you currently have open; logging an entry automatically files it under that book. For how to split things across books, see [Books & splitting](books.md).

So if you can't find the account or category you want in a dropdown, go to the corresponding page first and create it.

---

## Multiple currencies: how to record a foreign-exchange

If you only use RMB, skip this section.

When you make a **Transfer** and the "From" and "To" accounts have different currencies, Hengji automatically recognizes this as a **foreign-exchange**, and the form gains an extra field:

- **Sent (original currency)**—how much you actually paid out (in the source account's currency).
- **Received (original currency)**—how much the other side actually received (in the destination account's currency).

Hengji only records the two real numbers you enter; it **won't** multiply by an exchange rate on its own, nor will it compute exchange gains/losses for you. Whatever small gap there is between the two amounts simply lives inside each account's own original-currency balance, and only surfaces when you convert all your assets to RMB to see the total. Both fields must be positive numbers; leaving one out triggers a "Please enter a valid received amount" warning.

---

## How it works: a double-entry record is quietly created behind the scenes

What you see is "Log an entry," but under the hood Hengji actually splits it into **two offsetting entries** so the books always balance—this is double-entry bookkeeping, but you can use it perfectly well without understanding any of it.

| What you log | What happens underneath |
| --- | --- |
| Expense 100 | "Category" account +100, "Account" −100 |
| Income 100 | "Account" +100, "Source" account −100 |
| Transfer 100 | "To" account +100, "From" account −100 |

For an ordinary (single-currency) transaction, the two entries must sum to 0, or Hengji won't let you save—this is the floor it holds for you: money can't appear from nowhere or vanish. Foreign-exchange is the exception: the two legs are in different currencies and aren't equal in value to begin with, so this "sum to 0" rule is waived.

You don't see or deal with this layer day to day; knowing that "logging an entry is a balanced record behind the scenes" is enough.

---

## The Transactions page: review every entry you've logged

**Transactions** in the left-hand menu lists all the transactions you've logged, newest first. Each row looks like this:

- An **icon** on the left (auto-assigned by category, e.g. dining 🍜, transfer 🔁, foreign-exchange 💱).
- A **title plus a line of small text** in the middle: the title shows your note first, and the small text is "category · account · date".
- An **amount** on the right: expenses show as negative, income as positive, transfers neutral.
- If this entry belongs to a business book, a gray **"Business"** tag hangs next to the title.

The top of the page shows the current book's name and "N entries total".

### Transactions only shows the current book

The Transactions page **only shows transactions from the book you currently have open**. To look at another book (say, switching from "Personal" to "Business"), switch books at the top first, and Transactions follows along. In other words, your business book and personal book each show their own; they never get mixed into one list.

### Deleting an entry

There's an **×** at the far right of each row; tap it to delete. A confirmation box pops up:

- Ordinary transaction—it asks "Delete this transaction?"; confirm to delete.
- If this entry contains a **reconciled** record, the prompt changes to "This transaction contains a reconciled record; deleting it will break a completed reconciliation (you'll need to reconcile again)." This means you previously reconciled it, matching it against your bank statement, and deleting it means you'll have to match again. See [Reconciliation](reconciliation.md) to learn how reconciling works.

Deletion is a soft delete, so don't panic if you make a mistake—but for now, once deleted in the UI it disappears from Transactions, so log carefully.

---

Related: [Accounts](accounts.md) · [Books & splitting](books.md) · [Reconciliation](reconciliation.md)
