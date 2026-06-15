[← Back to manual index](README.md)

# Orders · Customers · A/R

Record a piece of business: who you sold to, what you sold, whether you got paid, and how much you made. Hengji ties customers, orders, receivables, and gross margin into a single thread, generating double-entry records in the background, so all you have to do is write the order.

**For**: small business owners who are starting to take orders, sell on credit, and want to know "who still owes me money, and does this order even make a profit" (online shops, community group-buying, wholesale, dropshipping) / advanced scenarios.

> "Orders / Customers" is an advanced merchant feature and is hidden in the default minimal mode. Go to **Settings → Enable advanced merchant features** first, and these two entries will appear in the left sidebar. Worried about mixing business and personal accounts? Before you write any orders, read [Books & splitting accounts](books.md) and put this business in its own separate book.

---

## 1. Set up customer profiles first

Before you can write an order, you need a customer. Go to the "Customers" page → "Add customer", and fill in three things:

| Field | Description |
| --- | --- |
| Name | Required, cannot duplicate an existing customer's name |
| Phone | Optional, handy for reaching someone when chasing payment |
| Default payment term (days) | Optional, enter `0` or leave blank = cash on delivery; enter `30` and "this customer is allowed to hold the balance for 30 days" |

The payment term decides **how many days until an unpaid credit order is overdue**: due-date reminders and A/R aging are both calculated from it. In the customer list, each person shows their current **A/R** in real time (how much they still owe you) or "Settled".

- **Rename**: just change it; the underlying receivables account is renamed along with it, and the outstanding balance is not lost.
- **Archive**: if you don't want to see this customer when writing orders anymore, archive them. Historical orders are preserved as usual after archiving; if they still owe money, you'll be warned before archiving.

---

## 2. Write an order

Go to the "Orders" page → "New order":

1. **Pick a customer** and a **order date** (defaults to today). If you've enabled [multi-currency](multi-currency.md), you can also pick a **settlement currency**, defaulting to CNY.
2. **Add line items**. For each line you can:
   - Pick a [product](inventory.md) from the dropdown — this auto-fills the selling price (converted into the settlement currency and filled in, still manually editable);
   - Or choose "Free input" and type the name, quantity, and unit price by hand (for one-off items you haven't set up as products).
   - Once you manually edit the name, that line becomes free text and is disconnected from the product (editing the price does not disconnect it).
3. **Check fees per line** (optional). If you've configured [extra fees](fees.md) in the book (shipping, packaging, platform commission, threshold discounts...), they're listed below each line — click one to attach that fee to the line. **All fees count as revenue**, and the order total = item amount + each fee.
4. Use "+ Add a line" to add more items, fill in any notes, and click **Save order**.

The bottom shows the total in real time, and if you've checked fees it breaks it down for you as "items X + extra fees (shipping Y, packaging Z)".

### Out of stock? A draft purchase order is generated automatically

On save, Hengji calculates the shortfall against **current inventory**:

- Enough stock (or all free-text lines / inquiry-only products) → the order goes straight to **Awaiting shipment**.
- Any product short on hand → the order goes to **Awaiting purchase**, and **a draft purchase order is generated for each out-of-stock product** (different products can come from different suppliers, each confirmed independently). Draft orders are just placeholders, are not booked, and are safe.

---

## 3. The five order statuses

The status label on the order card:

| Status | Meaning | What you can do |
| --- | --- | --- |
| Awaiting purchase | Some products are out of stock, waiting for you to source them | Confirm each one via "Purchase: ..."; or cancel |
| Awaiting shipment | Goods are ready, waiting to confirm revenue | Click "Complete (confirm revenue)"; or cancel |
| Shipped | (Intermediate transitional state) | — |
| Completed | Revenue confirmed, cost transferred | "Receive payment" to record money the customer sent |
| Cancelled | Voided | — |

**Cancel**: for an order whose purchase has never been confirmed, cancelling has no accounting impact (the draft purchase orders are voided along with it). Once a purchase order has been confirmed (and produced accounting entries), this order can no longer be cancelled directly — it needs to be reversed manually.

### Source goods for an out-of-stock order (Awaiting purchase → Awaiting shipment)

Click "Purchase: product × quantity" on the order to expand the draft order:

1. Pick a **supplier** (add one on the [Suppliers](purchases.md) page first if you have none), a **payment method** (on credit → records A/P / pay now → pick a CNY payment account), and a **date**.
2. Fill in the **purchase price** for each line (defaults to the product's cost price, editable), and click **Confirm purchase**. The cost is first parked under "Sourcing-in-transit cost" and is transferred when the order is completed.
3. If by now stock is sufficient again, the form will prompt you, and you can "Stock now sufficient · void" this draft; the order moves straight to Awaiting shipment and ships from inventory.

> Purchase prices are entered in CNY (sourcing is denominated in CNY). For an order with multiple out-of-stock products, you confirm each of their draft orders separately, and the order only moves to Awaiting shipment once all of them are handled.

---

## 4. Completing an order = confirm revenue + transfer cost + compute gross margin

Click **Complete (confirm revenue)** on an "Awaiting shipment" order, and Hengji does three things at once:

1. **Confirm revenue**: the customer's payable (including extra fees) is recorded as your A/R from them.
2. **Stock out / transfer cost**: inventory products are stocked out at the moving weighted-average price; sourced items are transferred from "Sourcing-in-transit" — the two together make up this order's cost.
3. **Compute gross margin**: once completed, the order card directly shows **gross margin = fee-inclusive revenue (converted to CNY) − cost**.

> When stock is insufficient and no purchase covers this order, completion is blocked and tells you which product is short by how much — go purchase or replenish stock first, then come back to complete. This guarantees you never get a "sold but nothing deducted" record.

### How it works (the double-entry behind the scenes)

You only clicked "Complete" once, but here's what was actually recorded behind the scenes (skip this if it doesn't make sense — it won't affect usage):

- Confirm revenue: debit `Accounts receivable / customer`, credit `Operating revenue` — net assets increase.
- Cost transfer: debit `Cost of sales`, credit `Inventory goods` (or `Sourcing-in-transit cost`).
- Later receipt: money moves from `Accounts receivable / customer` into your receiving account (WeChat, corporate account...), A/R turns into cash, net assets unchanged.

A/R is booked to separate accounts by "customer × currency", so the same customer's CNY debt and USD debt are never mixed together.

---

## 5. Credit sales and installment receipts

On a completed order, click **Receive payment** to record money the customer sent: enter an **amount** (defaults to the portion this order itself still owes), a **date**, and a **receiving account** (you can only pick an asset account in the same currency as the order).

Hengji spreads each receipt across the customer's completed orders by "customer × currency", on a **FIFO (earliest orders cleared first)** basis:

- A receipt you record on a specific order is **applied to that order first** — it won't accidentally pull money from another order.
- If a particular order is **overpaid**, or you record an amount without specifying an order, the excess **carries over** to still-owing orders in order date sequence.
- If everything is filled and there's still money left, it becomes this customer's **prepayment** (money they paid in advance, applicable to future orders).

Each completed order shows a receipt badge: **Unpaid / Partial X/Y / Fully paid**.

---

## 6. A/R overview, aging, and due-date reminders

As long as there are still unsettled orders, an **A/R overview** appears at the top of the "Orders" page:

- Total **A/R** (what others owe you) and total **prepayments** (what you collected in advance), both converted to the display currency.
- **Aging buckets**: each debt is sorted by aging (days since the order date) into four tiers —

  | 0–30 days | 31–60 days | 61–90 days (yellow) | Over 90 days (red) |
  | --- | --- | --- | --- |

  Mixed currencies are first converted to the display currency and then bucketed, for easy side-by-side comparison.
- **Per-order detail**: lists each unsettled order, showing the customer, order date, how many days owed, and how much owed; anything past this customer's **payment term** is flagged with a red **Overdue** tag — that's your due-date reminder.

> Aging is measured "from the order date", not "from the due date". Customers with no payment term set (cash on delivery) are not tracked for due dates and are never flagged overdue.

---

## 7. Gross margin summary

Once you have completed orders, the "Orders" page shows a **Gross margin summary** card, aggregated on a **CNY basis** (revenue converted to CNY − cost):

- **By customer**: which customer earns you the most / is losing money.
- **By product**: which item is the profit driver and which is losing money.

Each row shows the gross margin (positive for profit, negative for loss) and a "rev X cost Y" breakdown, sorted by gross margin from high to low. If you want to know "does this business actually make money", look here.

---

## Related

- [Accounts](accounts.md) — receiving accounts and A/R / A/P accounts are managed here
- [Books & splitting accounts](books.md) — put this business in its own separate book, away from personal accounts
- [Products](inventory.md) · [Inventory & restocking](inventory.md) · [Suppliers](purchases.md) — the upstream for picking items when writing orders and sourcing out-of-stock goods
- [Extra fees](fees.md) — how to configure shipping / commission / threshold discounts
- [Multi-currency](multi-currency.md) — cross-currency settlement and receipts
