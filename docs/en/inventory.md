[← Back to manual index](README.md)

# Products & Inventory

Keep a clear picture of how much stock you bought, how much is left on hand, and what each unit costs. When you sell, the cost is deducted from inventory automatically.

**Good for**: small owners trading physical goods (small online shops, community group-buys, street stalls, mini-marts) / anyone who wants to know "how much money is tied up in this pile of stock" and "how much do I actually make per sale".

> This is an advanced feature. By default, regular users don't see the "Products" and "Inventory" entries — go to **Settings → Enable advanced merchant features** first, and they will appear in the left sidebar. If you only log transactions and never touch physical goods, skip this page.

---

## The one-minute overview

Everything about "goods" in Hengji lives on two pages:

- **Products**: register what you sell — name, purchase price, sale price, unit. This is reference data; you fill it in once.
- **Inventory**: register the movement of goods — how much came in, how much is left, what it's worth, and reconciliation against a stock count.

One rule runs through this whole page: **every product is inventory-tracked by default**. You don't have to tick a "should this product be inventory-managed?" box — as long as it's a physical item, Hengji tracks its on-hand quantity and cost for you. The only exception is "quote-only / service" items (see below).

---

## Products: register what you sell

Go to the "Products" page, find the "Add product" card at the bottom, and fill in:

| Field | Description | Required |
|---|---|---|
| Name | Product name; cannot duplicate an existing product | Yes |
| Unit | piece / kg / box… shown next to the quantity | No |
| Purchase price | Your cost when buying in, in CNY | No (blank = 0) |
| Sale price | The price you sell at, in CNY | No (blank = 0) |
| Opening stock quantity | See "Stock already on hand when you open the books" below | No |
| Opening unit cost | Same as above; blank uses the purchase price | No |

Both the purchase price and sale price are only **defaults** — you can still adjust them on the fly when logging an entry or buying in stock. Once filled, selecting this product in an [order](orders.md) auto-fills the sale price, and buying in auto-fills the purchase price, saving you a few keystrokes.

In the product list, click **Edit** on any row to change its details, or click **Archive** to put away products you no longer sell (archived products can't be selected in an order, but all history stays and you can **Restore** them anytime).

### Quote-only / service: the kind that doesn't enter inventory

If what you sell is a design fee, a sampling fee, a delivery fee — something with **no physical item and no stock to occupy** — tick **"Quote-only / service"** when adding the product. Once ticked:

- This product is not inventory-tracked, and won't appear on the "Inventory" page;
- Selling it doesn't carry over any cost (it had no purchase cost to begin with);
- The two "Opening stock" boxes collapse automatically — a quote-only item has no inventory to speak of.

For all other physical products, **leave this box alone** — they are inventory-tracked by default.

### Stock already on hand when you open the books (opening inventory)

When you first start using Hengji, your storeroom is probably already stacked with goods. When adding a product, fill in the **Opening stock quantity** (and the optional **Opening unit cost**), and Hengji counts this existing stock straight into inventory without touching your cash accounts — because this stock wasn't paid for today; it's what you already had before opening the books.

> When the opening unit cost is left blank, the purchase price is used. When both are 0 (you don't know what it's worth), only the quantity is recorded, not the amount.

---

## Inventory: movement of goods and stock counts

Go to the "Inventory" page. The very top row is the **Total inventory value** (the worth of all goods on hand, valued at cost, in CNY).

Below, each product gets one row showing three key numbers:

- **On hand N**: how much is left right now (auto-decreased when you sell, auto-increased when you buy in);
- **Average cost**: the average cost per unit (see "How it works" below);
- **Value**: this product's total inventory value = on hand × average cost.

If the on-hand quantity is 0 or negative (oversold), the number is flagged in red to warn you.

### Buying in stock (cash / on credit)

On the "Buy in / restock" card, pick a product, fill in the quantity and purchase price, then choose a **payment method**:

1. **Cash** — pay on the spot. Pick a **payment account** (your cash, Alipay, bank card, or other CNY account); the money is deducted from that account, and the goods enter "Inventory goods".
2. **On credit (record A/P)** — owe it for now. Pick a **supplier** (add it first on the [Suppliers](purchases.md) page); the goods enter inventory, and the debt is recorded under that supplier. Pay it off later on the [Suppliers](purchases.md) page.

Fill it in and click **Buy in**. The on-hand quantity and inventory value update immediately.

> Inventory is denominated in CNY, so the buy-in payment account can only be a **CNY asset account** — foreign-currency accounts, A/R, and the like won't appear in the dropdown.

### What if there isn't enough to sell: auto-generated draft purchase orders

This is where the unified inventory model saves you the most worry. When you [log an entry](orders.md), if a product's **on-hand quantity is short of what this order needs**, Hengji won't block you — instead it:

1. Marks the order as **"Pending purchase"**;
2. Automatically generates a **draft purchase order** for each short product (quantity exactly equal to the shortfall, unit price pre-filled with that product's purchase price).

You just go to the [Purchases](purchases.md) page to confirm these drafts (filling in the supplier and purchase price). Once the goods are replenished, the order can move to "Pending shipment". In other words: **being short of stock doesn't stop you from logging the order first**, and Hengji starts the restock process for you.

### Stock count / inventory adjustment (surplus & shortage)

When the actual count doesn't match the books — shrinkage, scrapping, miscounts, lost goods — use a stock count to bring them in line. Click **Stock count / Adjust** on a product's row:

1. **Actual quantity**: fill in the true on-hand count from your stock count;
2. **Reason** (required): reconciliation / scrapped / shrinkage… recorded into Transactions for the record;
3. **Date**;
4. If the actual quantity is **greater than** on hand (surplus), an extra **Surplus posting unit cost** box appears — leave it blank to post at the current average cost.

Click **Confirm adjustment**:

- **Shortage** (actual < on hand): the missing goods are counted as a loss at the current average cost, posted to **"Inventory gains/losses"**;
- **Surplus** (actual > on hand): the extra goods are posted at the unit cost you entered (or the current average cost), also into **"Inventory gains/losses"**.

A stock count doesn't touch your cash accounts — it only brings the book inventory in line with reality, recording the difference as a gain or loss.

---

## How it works (just enough)

You only ever "log an entry" — the double-entry postings below are generated **automatically** by Hengji, and you can skip this if it doesn't make sense:

- **Buying in** (cash): money in the payment account → transferred into "Inventory goods" (asset for asset, total money unchanged — it just turns from cash into goods); on credit, an "A/P" debt to the supplier is recorded instead.
- **Selling**: when an [order](orders.md) is completed, the cost of the goods sold is carried over from "Inventory goods" to "Cost of sales" using **moving weighted-average cost** — this is the key to how much you really made on that sale.
- **Moving weighted-average cost**: each time a new batch comes in at a different price, Hengji divides the combined cost of old and new goods by the combined quantity to get a new "average cost per unit". When you sell, cost is deducted at this average. When on hand hits zero, the average cost resets to 0, so rounding remnants aren't carried into the next batch.
- **Surplus/shortage / opening inventory**: these use the "Inventory gains/losses" and "Opening balance" accounts respectively as the counterparty, keeping the books always in balance.

These accounts (Inventory goods, Cost of sales, Inventory gains/losses, Opening balance, A/P…) are all **created automatically the first time they're used** — you don't have to create them by hand on the [Accounts](accounts.md) page.

---

## Related

- [Orders](orders.md) — selling goods; cost is carried over automatically when an order completes
- [Purchases](purchases.md) — confirm the draft purchase orders auto-generated for shortages
- [Suppliers](purchases.md) — buying in on credit, and paying off supplier debts
- [Accounts](accounts.md) — payment accounts, and the auto-created inventory-type accounts
- [Books & split accounting](books.md) — products and inventory are each managed per book
