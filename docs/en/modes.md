[← Back to the manual index](README.md)

# Simple and Pro: two interfaces, switch by the size of your business

When you first open Hengji, you see "Simple mode" — just four sections: Overview, Transactions, Budget, and Accounts. Logging a business entry is as effortless as jotting down your grocery spending. Once your business grows and you need to manage purchasing and credit, head into Settings and turn on "Merchant advanced features." Inventory, reconciliation, multi-currency, and the other professional tools then appear all at once.

**Best for**: all users / anyone unsure whether to turn on that pile of pro features.

---

## Simple mode is the default

Hengji works out of the box and **shows no pro features by default**. Whether you create a personal Book or a [business Book](books.md), the tabs on the left are only these four:

| Tab | What it does |
|------|--------|
| Overview | See this Book's income, expenses, and profit at a glance |
| Transactions | Log and review entries one by one ([how to log an entry](recording.md)) |
| Budget | Set a ceiling for a category of spending and get warned when you exceed it |
| Accounts | Manage your Alipay, WeChat, cash, and bank cards ([Accounts](accounts.md)) |

> **Why is this the default for sole proprietors?**
> For people running street stalls, small shops, or community group-buys, the most common pain point is that **business money and personal money are mixed in the same Alipay** — the same QR code takes in sales revenue and pays for groceries. Hengji's core purpose is to help you separate these two sets of books with [multiple Books](books.md), not to force you to learn a whole inventory-management suite. So by default it keeps only the four sections you most need for "logging an entry," and leaves the rest until you actually need them.

In Simple mode, the interface for a business Book and a personal Book looks **nearly identical**. The only difference lies in the Book type itself (which affects Accounts and how things are grouped in the Global overview), not in how many extra buttons sit in front of you.

---

## When should you turn on "Merchant advanced features"

If you start running into the situations below, it's time to turn it on:

- You need to manage **purchasing, stock, and sales**, and know how much of each product is left and how much you make on it;
- Customers **take goods first and pay later** (on credit), and you want to track who still owes you and when it's due;
- You also **owe suppliers** for purchases on credit, and need to record payment terms and avoid missed payments;
- An order carries **shipping, packaging, or platform commission** and other extra fees on top of the goods;
- You want to **reconcile once a month**, checking whether the balance in the software matches your actual Alipay balance;
- Your income and expenses include foreign currencies like **US dollars, Japanese yen, or even Bitcoin**;
- You also want a separate **investment Book** for stocks and funds.

If you only log small day-to-day entries and never touch these, **keep it off for a cleaner interface** — that's also Hengji's default recommendation for sole proprietors.

---

## How to turn it on (one step)

1. Click "⚙ Settings" at the bottom left.
2. The very first item at the top is **"Enable merchant advanced features"** — check it.
3. It takes effect immediately, no restart needed. A row of settings cards appears below, and your business Books on the left grow new tabs.

To switch back to Simple mode, just uncheck it — **not a single piece of the data you've already recorded is lost**; those pro tabs are simply tucked away for now.

---

## What gets unlocked once it's on

After you check "Enable merchant advanced features," all of these appear at once:

### 1. Inventory (business Books gain 6 more tabs)

A business Book's tabs expand from 4 into a full set, arranged by business workflow:

| New tab | What it does |
|----------|------|
| Orders | Record what each order sold, whether payment was received, and how much the customer owes |
| Customers | Manage customers and their payment terms (A/R) |
| Suppliers | Manage suppliers and purchasing payment terms (A/P) |
| Products | The list of things you sell |
| Stock | How much of each product is left |
| Purchasing | Record incoming goods |

(The original Overview / Transactions / Budget / Accounts are still there, just moved further back.)

### 2. Extra fees (the "Fees" tab)

Attach extra fees like shipping, packaging, and platform commission to an order. You can preset reusable tiers (amounts can be negative too, to record full-reduction discounts).

### 3. Monthly reconciliation

A "✓ Reconciliation" entry appears on the left so you can regularly check whether the account balances recorded in the software match the real balances. See [Reconciliation](reconciliation.md) for details.

### 4. Multi-currency and accounting basis (in Settings)

The Settings page unfolds a few new cards:

- **Accounting basis**: switch whether "this month's income / profit" is presented on an **accrual basis** (income counts when the order is completed, including credit) or a **cash basis** (it counts only when the money lands). Note that this only changes how the reports read — it **changes none of the entries you've already recorded**; under the hood, bookkeeping is always on the accrual basis. The two bases differ only for business Books that involve credit; for personal / investment Books the results are the same.
- **Multi-currency**: once on, you can add currencies like US dollars / Japanese yen / Bitcoin, choose a currency when creating an account, and designate a "display currency" to convert and show the Global overview. (As soon as you've created a foreign-currency account, multi-currency locks itself to on; you must archive all foreign-currency accounts before you can switch back to RMB-only.)

### 5. Investment Book type

When creating a new Book, the Book-type dropdown gains an "Investment" option, used to separately track stocks, funds, and the like.

### 6. Reminder banners

In advanced mode, a reminder bar pops up at the top of a Book at the appropriate times:

- **Reconciliation reminder**: when you're near the monthly reconciliation date you set, and the Book still has unreconciled transactions;
- **A/R due reminder**: when a customer's credit is about to come due or is already overdue, prompting you to collect;
- **A/P due reminder**: when the money you owe a supplier is about to come due, prompting you to pay.

These reminders **never appear at all in Simple mode** — they don't intrude, and in any case Simple mode has no reconciliation or order pages to go to. The reconciliation date and lead-time days for reminders are all adjustable in Settings.

---

## The principle: the interface changes, the books don't

Simple and Pro are just **two interface shells**. No matter how you flip the switch:

- Every entry you record is backed by a standard double-entry journal that Hengji generates automatically — the rigor is never compromised;
- Switching modes only shows / hides tabs and settings items; it **never deletes or alters data**;
- Turn advanced features off and then back on, and the orders, stock, and foreign-currency accounts you recorded before are all still there, exactly as they were.

So you can confidently start out in Simple mode and turn on the professional tools whenever your business grows — Hengji has been keeping your books correct underneath the whole time.

---

Related reading: [Accounts](accounts.md) · [Books and separating accounts](books.md) · [Log an entry](recording.md) · [Reconciliation](reconciliation.md)
