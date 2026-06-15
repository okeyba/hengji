# Advanced · Multi-Currency

[← Back to manual index](README.md)

Got paid in foreign currency, stashed some Bitcoin, or occasionally need to swap currencies? Hengji can manage all of it together, without getting in the way of people who only use CNY.

**Who it's for**: small business owners with foreign-currency income/expenses, those holding foreign or crypto assets, and advanced users. If you only ever move CNY in and out, you can skip this page.

---

## Off by default, turn it on when you need it

Out of the box, Hengji is pure CNY: there's no "currency" column anywhere in accounts or reports, which is one less layer of distraction. Multi-currency is an advanced merchant feature, and you turn it on in two steps:

1. Go to "Settings" → check "**Enable advanced merchant features**".
2. Scroll down to the "**Multi-currency**" card and check "Enable multi-currency".

Only after this is on do you get the currency registry, the currency picker when creating an account, and the display-currency switcher in reports. If you only use CNY, leave it off — the interface stays cleaner.

> For more on minimal mode and the advanced-features toggle, see [Getting started & settings](modes.md).

### Holding a foreign-currency account "locks it on"

If you've already created a foreign-currency account (even if you later want to turn this off), the multi-currency toggle becomes **locked on and can't be unchecked** — because "turning off multi-currency while still showing a foreign-currency account" is self-contradictory. In that case the card spells out which currencies are in use, e.g. "Foreign-currency account already in use (USD); multi-currency is locked on".

To get back to pure CNY, first go to the [Accounts](accounts.md) page and archive all foreign-currency accounts; only then does the toggle become switchable again.

---

## A self-managed currency registry

Hengji doesn't ship a long list of currencies, and it doesn't pull exchange rates from the internet — you register only the few you actually use. This registry is **globally shared**: all [Books](books.md) share the same set of currencies.

CNY (¥) is the **base currency** — it's always the first row in the list, fixed and unchangeable, with an exchange rate permanently set to 1. You add the rest yourself, filling in five fields for each:

| Field | Meaning | Example |
| --- | --- | --- |
| Code | Currency code, auto-uppercased | `JPY`, `USD`, `BTC` |
| Symbol | Display symbol; falls back to the code if left blank | `¥`, `$`, `₿` |
| Name | Display name; falls back to the code if left blank | Japanese Yen, US Dollar, Bitcoin |
| Decimals | Number of decimal places of the smallest unit (0–8) | `0` for JPY, `8` for BTC |
| Rate | **1 unit of this currency = how many CNY** | USD ≈ `7.1`, BTC ≈ `400000` |

Steps to add one:

1. In the row of input boxes at the bottom of the "Multi-currency" card, fill in the code, symbol, name, decimals, and rate in order.
2. Click "**Add**".

A few things to note:

- **The code must be unique**, and it can't be CNY (that's the base currency).
- **The rate must be greater than 0**, otherwise you'll see "Please enter a valid exchange rate against CNY".
- For a currency you've already added, just click the relevant cell to edit its symbol / name / rate — **it saves automatically when the input loses focus (you click outside it)**, no extra button needed.
- Changing a rate only affects the **converted display in future reports**; it never rewrites any recorded amount (the original-currency amount is always stored exactly as entered).

### Variable precision: JPY at 0 places, BTC at 8

Different currencies have different decimal places, and Hengji records and displays amounts according to the decimals you registered:

- **Japanese Yen** uses `0` decimals — 1 yen is the smallest unit, there are no "cents".
- **Bitcoin** uses `8` decimals — precise all the way to 0.00000001 BTC (a.k.a. 1 satoshi).
- CNY and USD are both 2 places.

> ⚠️ **Once any account is using a currency, its decimals are locked and can't be changed.** Changing the decimals would make the program "misread" amounts that have already been recorded. To switch precision, first archive the accounts using the old setting. Symbol, name, and rate are not subject to this restriction and can be changed at any time.

### Deleting a currency

Only a currency that **no account is using** can be deleted. If a currency still has accounts attached to it, deletion is blocked with the message "Accounts are using this currency; can't delete (archive them first, or switch them to another currency)".

---

## Picking a currency when creating an account

Once multi-currency is on, creating a new real cash account adds a "currency" field, where you pick from the currencies you've registered. For example:

- "USD PayPal" → choose USD
- "Binance Wallet" → choose BTC
- "Storefront Cash" → keep CNY

An account's currency **determines the original currency of every entry inside it** — money logged to a USD account is counted in USD, money logged to a CNY account is counted in CNY. For how accounts are created and why real accounts are globally shared, see [Accounts](accounts.md).

---

## Display currency: overview conversion + original-currency chips

Each account's balance, and every entry in Transactions, is **always shown in its original currency** ($ for USD, ¥ with no decimals for JPY). But the Global overview has to show different currencies side by side, so it needs a single "**display currency**" to convert into.

In the "Multi-currency" card there's a "display currency" dropdown — pick one of your registered currencies (defaults to CNY). Once selected:

- **The Global overview, plus each book's net worth and income/expense**, are shown converted into the chosen display currency using the exchange rates.
- **Each account's balance and every transaction still use the original currency**, untouched.
- Multi-currency assets in the overview are first **subtotaled by currency**, with each group labeled in its original currency (an original-currency chip), then totaled into the display currency. This way you can see both "how much USD and how much CNY I have" and a single converted grand total.

Conversion is purely a **display-layer translation** — not a single original-currency amount underneath is changed. Switch the display currency or change a rate and the converted figures you see will change, but the money you actually recorded doesn't move a cent.

> How it works (just enough): each currency's rate is registered "relative to CNY". To convert, the program first translates the original currency into the CNY baseline, then divides by the display currency's rate to move it across. So you only have to maintain that one "rate against CNY" column, and the display currency can be switched freely.

---

## Currency exchange: one entry, two original-currency legs

Swapping CNY into USD, or selling Bitcoin back into CNY — these "one transaction, two currencies" operations are recorded by Hengji as **transfers between accounts**, except the outgoing and incoming sides are different currencies.

The way to do it is a single transfer from a foreign-currency account → an account in another currency (or the reverse): you enter how much the source account actually paid out (in the source account's original currency), then how much the destination account actually received (in the destination account's original currency). Each leg is recorded at its own real original-currency amount, with no need for you to compute an intermediate rate by hand.

Once recorded, this entry shows up in [Transactions](recording.md) as a "**currency exchange**" (with a 💱 icon), and the subtitle reads "From [account] [outgoing amount] → [account]", so you can see at a glance that this was a cross-currency swap rather than a plain same-currency transfer (🔁).

> Tip: a currency exchange is recorded by each leg's real settled amount, which effectively "pins" the actual deal rate for that transaction into the entry itself — independent of the reference rate in the currency registry used for report conversion.

---

## Related

- [Accounts](accounts.md) — how real cash accounts are created and why they're globally shared
- [Books & splitting accounts](books.md) — keeping business and personal money separate
- [Transactions](recording.md) — logging an entry, transfers, and currency exchanges
- [Getting started & settings](modes.md) — minimal mode and the advanced-features toggle
