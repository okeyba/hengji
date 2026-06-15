[← Back to manual index](README.md)

# Settings (accounting basis · reminders · advanced)

The Settings page holds the "global switches"—change one once, and it applies to all your Books, not just the one you're currently viewing.

**Who it's for**: every user does the basic setup; once you turn on "Advanced features for merchants," you'll also use the advanced items like accounting basis, reconciliation / receivables-and-payables reminders, and multi-currency.

---

## Global: change once, every Book updates together

Open "Settings" and you'll see "Global · Applies to all Books" at the very top. Every item on this page takes effect uniformly, no matter which Book you're currently looking at—for example, if you set "Display currency" to USD, both the Global overview and each Book's net worth are shown converted to USD. There's no need to set it Book by Book.

## Minimal mode vs. advanced features for merchants

Hengji defaults to **minimal mode**: a regular user opening it sees only four things—Overview / Transactions / Budget / Accounts—enough to log everyday small entries. The professional features stay hidden, keeping the interface clean.

The very first card on the Settings page is that master switch:

> ☐ **Enable advanced features for merchants**

Only after turning it on do these unlock: inventory & sales (products / stock / purchasing / suppliers / receivables & payables), extra fees, monthly reconciliation, multi-currency, accounting-basis switching, and investment Books.

**So**: the four cards covered below—"Accounting basis," "Reconciliation reminder," "Receivables/payables due reminder," and "Multi-currency"—only appear once you've ticked this switch. If you only log everyday small entries, just leave it off; you can come back and turn it on anytime.

---

## Accounting basis: accrual basis / cash basis

> Only appears when "Advanced features for merchants" is enabled.

This switch only changes how report figures like "this month's income / profit" are calculated—it **doesn't touch any entry you've already logged**. Underneath, everything is always recorded on an accrual basis; what you switch here is purely how reports are presented.

| Basis | In a sentence | Best for |
| --- | --- | --- |
| **Accrual basis** (default) | An order counts as income once it's completed, even if the money hasn't come in yet (credit sales are counted too) | Wanting a formal view, aligned with the balance sheet |
| **Cash basis** | Income only counts once the money actually arrives; credit sales count only when collected | Wanting a true, intuitive view of cash flow |

### When do the two differ?

**The two bases only diverge for "business Books with credit sales."** If you run a cash business where money and goods change hands at the same time, or you keep a personal Book or an investment Book, the two bases compute exactly the same result—no need to agonize over it, just leave the default.

### How it works (briefly)

The difference comes solely from the change in accounts receivable (A/R):

```
Cash-basis income = Accrual-basis income − net increase in A/R for the period (ΔAR)
```

- You sell goods on credit: the accrual basis counts the income in the current period, while the cash basis waits until payment is collected;
- The customer later pays: the cash basis only then counts it as income.

The two underlying journal entries don't change one bit; Hengji simply uses a different way of totaling when producing reports. For concepts like credit sales and payment terms, see [Books & splitting accounts](books.md) and [Receivables & payables](orders.md).

---

## Reconciliation reminder

> Only appears when "Advanced features for merchants" is enabled.

Set a "monthly reconciliation day," and as it approaches Hengji reminds you at the top of the Book to go to the "Reconciliation" page and check whether your book balance matches the actual balance.

1. **Reconciliation day**: pick "last day of each month," or "the Nth of each month" (any of 1–28), or "turn off the reminder."
2. **Advance reminder**: appears only after you've chosen a reconciliation day; you can pick "same day" or 1 / 2 / 3 / 5 / 7 days ahead (default 3 days ahead).

A thoughtful touch: **once a Book is fully reconciled and cleared, the reconciliation reminder won't bother you again.** For how to clear items and how to use the reconciliation page, see [Reconciliation](reconciliation.md).

---

## Receivables/payables due reminder

> Only appears when "Advanced features for merchants" is enabled.

When someone owes you money (A/R) or you owe a supplier money (A/P) in a business Book, Hengji reminds you at the top of the Book before the due date / after it's overdue to follow up on collecting / arranging payment.

- **A/R due date** = customer's order date + payment terms;
- **A/P due date** = purchase date + supplier's payment terms.

There's just one option, "Advance reminder":

| Option | Meaning |
| --- | --- |
| Turn off reminder | No reminder |
| On the due date | Remind only on the day |
| 3 / 7 / 14 / 30 days ahead | Start reminding this many days early |

Default is 7 days ahead (overdue receivables are a real pain point, so it's on by default). **No receivables/payables, or everything settled, means no interruptions.** For how to set payment terms and how to log receivables and payables, see [Receivables & payables](orders.md).

---

## Multi-currency

> Only appears when "Advanced features for merchants" is enabled. For the full how-to, see [Multi-currency](multi-currency.md).

If you only use RMB, leave this switch off—no currency options appear anywhere in accounts or reports, nice and clean.

If you need to record USD / JPY / Bitcoin and the like, tick "Enable multi-currency," and from then on you can choose a currency when creating an account. Once it's on, you can also do the following here:

- **Display currency**: net worth and income/expenses for the Global overview and each Book are shown converted to the currency you choose (each account's balance and Transactions still display in their own original currency, unchanged).
- **Custom currencies**: fill in the code / symbol / name / decimal places / exchange rate against RMB. RMB is the base currency, fixed at ¥, rate 1, and cannot be changed.

Two safety reminders:

1. **When a foreign-currency account is already in use, the multi-currency switch is locked on** (turning it off while foreign currency is still displayed would be self-contradictory). To return to RMB-only, first go to the "Accounts" page and archive all foreign-currency accounts.
2. **When an account is already using a given currency, its decimal places can no longer be changed** (changing them would misread already-recorded amounts), nor can the currency be deleted (archive first / switch to an account in a different currency).

How exchange-rate conversion works and how the Global overview is grouped by currency are explained at length in [Multi-currency](multi-currency.md).

---

## Where is the data stored?

All settings and accounts are stored in a local SQLite file on this very computer — no network, no upload. Before switching computers or reinstalling, remember to copy that data file yourself as a backup.

---

Related: [Accounts](accounts.md) · [Books & splitting accounts](books.md) · [Multi-currency](multi-currency.md) · [Reconciliation](reconciliation.md) · [Receivables & payables](orders.md)
