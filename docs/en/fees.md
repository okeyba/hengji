[← Back to manual index](README.md)

# Extra Fees and the Formula Engine

Commissions, shipping, packaging — the kind of money that "rides along with each order and shifts brackets based on order size." Let Hengji do the math for you: tick a box per line when you create an order, the amount pops out automatically, and it all counts as revenue.

**Good for**: online shops, community group-buys, platform drop-shipping — any small business that frequently adds or deducts assorted fees on top of the order amount.

> This is an advanced merchant feature. Go to "Settings → Enable advanced merchant features" first, and the sidebar will then show "Extra Fees" and "Orders." Plain bookkeeping doesn't need this — feel free to skip this page.

---

## In one sentence

You first **define** a fee in the Book (what it's called, how it's calculated), then **tick it on each product line when creating an order**. Hengji computes the amount in real time according to the rules you defined and counts it into that order's revenue.

Fee definitions are **Book-level** and reusable: build a "Platform Commission" once in the "Business" Book, and every order in that Book can tick it from then on. Switch to the "Personal" Book and it won't show up — fees travel with the Book, the same logic as in [Books and split accounting](books.md).

---

## Three calculation methods

When you add a fee, pick one "calculation method." It decides how the fee is computed from the product lines:

| Method | How it's calculated | Typical use |
|------|--------|----------|
| **Percentage** | Amount of ticked lines × percentage | Platform commission (4% of turnover), card-processing fee |
| **Fixed amount** | A one-time charge, regardless of how many lines are ticked | Packaging fee, base shipping |
| **By quantity** | Item count of ticked lines × per-unit amount | Per-item packing fee (¥1.5 each) |

Note that "Fixed amount" is a **one-time charge for the whole fee**, not collected once per line — no matter how many lines this fee is ticked on within one order, the amount stays the same.

---

## Tiered brackets: the bigger the order, the rate auto-shifts

Every fee supports **declarative tiers**: you list a few "threshold → value" brackets, and Hengji checks **which bracket the combined total of the lines that ticked this fee falls into**, then applies that one bracket to the whole group.

Percentage / fixed amount set the bracket by **amount total**; by-quantity sets it by **item-count total**.

For example, build a "Platform Commission," choose the percentage method, and list three brackets:

| Amount threshold | Percentage |
|----------|--------|
| ¥0 | 5% |
| ¥600 | 4% |
| ¥2000 | 3% |

Meaning: the combined total of the lines that ticked the commission < ¥600 is charged 5%, ¥600–¥2000 is charged 4%, and ≥¥2000 is charged 3%. An order that ticked ¥800 of goods is computed at 4% for the whole group.

Key rules:

1. At least one bracket must have **threshold set to 0** (the floor bracket, covering the smallest total). If you only want a single fixed rate, just fill in this one row and leave the threshold at 0.
2. Thresholds must be **non-negative numbers**.
3. **Values may be negative** — this is how spend-and-save discounts work. For example, "spend ¥300, save ¥20": choose the fixed-amount method, list two brackets — threshold ¥0 value 0, threshold ¥300 value −20 — and once the total reaches 300 it automatically deducts 20.

---

## Step 1: Define a fee

Open "Extra Fees" from the sidebar:

1. In "Add fee," fill in the **name** (e.g. "Commission," "Shipping").
2. Pick the **calculation method** (percentage / fixed amount / by quantity).
3. Fill in the tier brackets. Each row is one "threshold + value" bracket; click "+ Add a bracket" to add more, and remove extras with "×". For a single bracket, just leave the threshold-0 row.
4. Click "Add."

Each fee in the list shows all its brackets, for example `≥¥0 → 5% · ≥¥600 → 4%`.

**Edit / Archive**: click "Edit" to change the name, method, or brackets. Click "Archive" to stop it appearing when creating orders — fees already recorded in past orders are unaffected and compute as before. An archived fee can be "Restored" at any time.

---

## Step 2: Tick it when creating an order

Go to "Orders → New order." Below each product line is a row of "Extra Fees" buttons (the fees you've defined that aren't archived). Click one to highlight it, meaning this line applies that fee; click again to cancel.

- The same fee can be ticked on **multiple lines** — they merge into one group to compute a combined total and set the bracket together.
- Different lines can tick different fees with no interference.
- Once ticked, the breakdown shows in real time below: `Goods ¥800 + extra fee (commission ¥32)`, and the "Total" row is directly the fee-inclusive total.

After you save the order, the fee is bound to that order. Each order in the order list is tagged "Includes extra fee: commission ¥32," and the order's amount is the fee-inclusive total.

---

## How it works: fees are all treated as revenue

When you complete an order (confirm revenue), Hengji records the **goods amount + all extra fees** together into "Operating revenue," and the customer's payable and gross-margin calculations also use this fee-inclusive total. The double-entry behind it is generated automatically — you don't worry about debits and credits: tick fees when creating the order, complete the order, and the books are right.

This means commissions, shipping, and the like are seen by Hengji as part of the money you **take in** on this order, not as a separate expense account. If you actually have to pay the commission to the platform, that's a separate [transaction](recording.md).

---

## It's the first building block of the formula engine

"Extra Fees" is the first primitive to land from Hengji's planned **formula engine / plugin foundation**: a name + a set of declarative tier brackets that deterministically compute an amount from order lines. It's pure, recomputable, and auditable — the same inputs always yield the same result.

More complex fee rules down the road — even "describe a fee rule in plain language and have it auto-translated into brackets" — will all build on this primitive. What's already usable in the current version (v0.2.0) is exactly the set above: percentage / fixed / by-quantity × tier brackets.

---

Related: [Books and split accounting](books.md) · [Accounts](accounts.md) · [Transactions](recording.md)
