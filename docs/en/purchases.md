[← Back to manual index](README.md)

# Purchases · Suppliers · A/P

Restocking inventory, paying for goods, logging a shipping fee — Hengji gathers all of this onto two pages: "Purchases" and "Suppliers." Pay on the spot or buy on credit, either works. Who you owe and when it's due — it keeps track for you.

**For**: small owners who need to buy stock or buy on credit from suppliers (small online shops, community group-buying, small restaurants stocking ingredients). Scenario = you need both to replenish inventory and to log odds-and-ends purchases like shipping fees and office supplies.

> Purchases and Suppliers are "merchant advanced features." Regular users don't see them by default — only after you turn them on at **Settings → Enable merchant advanced features** do "Purchases" and "Suppliers" appear in the menu. Minimal bookkeeping doesn't need this chapter.

---

## 1. The Purchases page: where does one purchase go?

Open "Purchases." The top half of the screen is the list of all purchase orders; the bottom half is "New purchase order." Every purchase order carries two tags: **destination** (what the money bought counts as) and **payment method** (paid on the spot or on credit).

There are only two destinations, corresponding to two completely different sets of books:

| Destination | Meaning | Example | Where the money goes |
| --- | --- | --- | --- |
| **Into inventory** | Restocking, posted to "Inventory goods" as quantity × cost | Buy 100 T-shirts to keep on hand for sale | Into the moving weighted-average pool; cost is only recognized when sold |
| **Expense** | Posted directly to an expense account, never into inventory | Shipping fees, office supplies, printer paper | An expense on the spot, with nothing hanging in inventory |

> There's also a third kind called "**purchase-to-order**" — buying on the fly for a specific customer order. You don't create it on this page; instead you generate it from the order in the **[Orders](orders.md)** page by clicking "Purchase for this order" on that order. Section 4 below covers it specifically.

### Create an "into inventory" purchase

1. Set the destination to **Into inventory**.
2. Choose the payment method (on the spot / on credit, see the next section).
3. Choose the **product** (if there isn't one, add it on the [Products](inventory.md) page first).
4. Fill in **quantity** and **cost (per unit)**. Leave the cost blank and it auto-uses the cost price recorded on the product.
5. Pick the date and click "New purchase."

This batch of goods now enters inventory, and the quantity on hand and average cost update accordingly. When you sell it, the system carries the cost out at the average price — this step is just about "how much came in and how much it cost."

### Create an "expense" purchase

1. Set the destination to **Expense**.
2. Choose the **expense account** (a spending account like shipping or office supplies; if there isn't one, add it on the [Accounts](accounts.md) page first).
3. Fill in the **amount** and a **description** (e.g. "shipping fee," "office supplies").
4. Pick the date and payment method, then click "New purchase."

This goes straight into that expense account and doesn't touch inventory. It's right for all the "bought it, spent it" odds-and-ends.

---

## 2. On the spot vs. on credit: pay in full now, or owe it for now

Every purchase order needs a payment method:

- **On the spot**: paid in full right away from a **payment account** (WeChat, Alipay, corporate account…). Pick a CNY account and the money goes out directly.
- **On credit (record A/P)**: take the goods now, pay later; the amount owed is recorded as accounts payable under the corresponding **supplier**. This is the source of "A/P."

> Before choosing on credit, you need a supplier. If you don't have one, the payment-method field prompts you to add one on the "Suppliers" page first.
>
> Purchases are **CNY-based**: the payment account for an on-the-spot purchase, and the repayment after an on-credit purchase, must both be CNY accounts.

In the list, every order is tagged "on the spot / on credit" with its amount on the right, so you can tell at a glance which ones are paid in full and which are still owed.

---

## 3. Supplier profiles and payment terms

On the "Suppliers" page, "Add supplier" at the bottom lets you create a profile:

1. **Name** (required, can't duplicate an existing supplier's name).
2. **Phone** (optional).
3. **Default payment term (days)**: enter `0` for cash on delivery; enter `30` to mean this supplier gives you 30 days. Optional — leave it blank and it's treated as 0.

The payment term is used to compute "when it's due" — the due date is the purchase date plus the term in days. It's only a reminder line; it never deducts money automatically.

Each supplier gets a row showing phone, payment term, and how much you currently still owe them ("A/P ¥…" or "settled"). On the right of each row:

- **Repay** (only appears while money is still owed) — see section 5 below.
- **Rename** — rename it and the accounts payable under it follows the change, without splitting the debt into two.
- **Archive** — tuck away a supplier you no longer work with. You can archive even with money still owed (it'll warn you); the historical books are kept and you can "restore" anytime.

---

## 4. Purchase-to-order: buying for a specific order on the fly

If you run on a "buy stock only once an order comes in" model (common in community group-buying and personal shopping/proxy buying), the flow isn't on the Purchases page but on the Orders page:

1. When you open an order on the [Orders](orders.md) page, if a product is short on stock, the order automatically becomes "awaiting purchase" and generates a draft purchase order.
2. On that order, click **Purchase for this order**, choose the supplier, fill in the purchase price for each item being bought, pick on the spot / on credit, and click **Confirm purchase**.
3. Once confirmed, the order moves to "awaiting shipment." This purchase cost is tied directly to this order and is carried out together when the order completes — it doesn't enter the inventory average-price pool and doesn't mix with the stock you keep on hand normally.

> If you find after opening the order that stock was actually enough, you can click "Void" on that draft purchase order, and the order can still ship from inventory.

Purchase-to-order orders show up in the Purchases list tagged "purchase-to-order." But you can't **create** a purchase-to-order on the Purchases page — it can only be generated from an order, because it needs to know which order it's buying for.

---

## 5. Accounts payable: how much you owe suppliers

As soon as there's a single on-credit purchase, A/P is created. Hengji records it **separately per supplier** (one A/P sub-account per supplier), forming an exact mirror of [accounts receivable](orders.md) (recorded separately per customer): A/R is what others owe you, A/P is what you owe others.

### A/P overview and aging

At the top of the "Suppliers" page, whenever there's an outstanding balance an **A/P overview** appears:

- **A/P** total: how much you still owe suppliers in all.
- **Prepaid** (if any): what you overpaid and suppliers still owe you back.
- **Aging buckets**: each on-credit balance is sorted by "how long it's been owed" (counted from the purchase date) into four tiers —

| Aging | Cue color |
| --- | --- |
| 0–30 days | Normal |
| 31–60 days | Normal |
| 61–90 days | Yellowish (worth watching) |
| Over 90 days | Red-flagged (dragged on too long) |

The further out, the more it stands out, making it easy to prioritize paying off the longest-owed first.

### Repayment

1. On the "Suppliers" page, find the supplier you want to repay and click **Repay** on its row.
2. In the small form that expands: the **payment amount** is pre-filled with "the full amount currently owed" (you can also change it to a partial repayment), pick the date, and choose a **payment account** (a CNY account).
3. Click **Confirm payment**.

After repayment, this supplier's A/P drops accordingly, and once settled it shows "settled." Repayment applies on a "oldest-owed-first" (FIFO) basis — you don't pick which one to repay; the system automatically offsets starting from the earliest debt.

### Due reminders

Once you've set a payment term (>0) for a supplier, when some A/P is nearing due or already overdue, you'll be reminded at the top of the book to schedule payment. If no term is set or everything is settled, you won't be bothered. This reminder shares one spot with the collection reminders for [A/R](orders.md), so you can see at a glance "money to collect this week / money to pay this week."

---

## How it works: the two entries behind the scenes (good to know, no action needed)

You only "log a purchase" — Hengji balances the double-entry bookkeeping automatically, and you don't normally need to deal with it. A peek gives you more confidence:

- **On-the-spot purchase**: Inventory goods (increase) ← Payment account (decrease). Money became goods.
- **On-credit purchase**: Inventory goods (increase) ← Accounts payable / a supplier (debt increase). Goods taken now, debt recorded.
- **Expense purchase**: An expense account (increase) ← Payment account or Accounts payable (decrease / debt increase).
- **Repayment**: Accounts payable / a supplier (debt decrease) ← Payment account (decrease). Settles what's owed.

Every entry has equal debits and credits and balances automatically — the "how much is still owed, how many days aged" you see is computed in real time from these entries, not a dead number you typed in.

---

Related: [Accounts](accounts.md) · [Books & ledger separation](books.md) · [Products](inventory.md) · [Orders](orders.md) · [Accounts receivable / Customers](orders.md)
