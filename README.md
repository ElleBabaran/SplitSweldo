# SplitSweldo 💸

> **On-chain payroll splitter built on Stellar/Soroban**  
> Atomically fan out USDC to multiple wallets in a single transaction — trustless, transparent, and instant.

---

## Contract on Stellar Expert

![SplitSweldo contract on Stellar Expert testnet](./Screenshot_2026-04-18_161710.png)

> Contract `CBC57CDUPQU6EP5IOLCTTIZEKCRLYVEIKG2SIRGM3WM35QP7XH7DOCHB` deployed and live on Stellar testnet.

---

## What is SplitSweldo?

SplitSweldo is a decentralized payroll splitting dApp built on the Stellar blockchain using Soroban smart contracts. An employer locks USDC into a contract, and on payday, the funds are atomically distributed to multiple recipient wallets according to pre-defined percentage splits — all in one transaction. If any transfer fails, the entire release is rolled back.

**Built for:** Freelancers, remote teams, DAOs, and anyone who needs trustless payroll splitting on-chain.

---

## Features

- ⚡ **Atomic fan-out** — all splits happen in a single Soroban transaction
- 🔒 **Non-custodial** — funds are locked in the smart contract, not held by any third party
- 📐 **Basis points (BPS) precision** — splits defined in BPS (1 BPS = 0.01%), total must equal 10,000
- 🛡️ **Double-release protection** — funded amount is cleared after release, preventing duplicate payouts
- 🌐 **Web UI** — clean dashboard for managing the full payroll lifecycle
- 🔄 **Multi-period support** — fund and release multiple pay periods with the same contract

---

## Tech Stack

| Layer | Technology |
|---|---|
| Smart Contract | Rust + Soroban SDK |
| Blockchain | Stellar Testnet |
| Backend | Node.js + Express |
| Frontend | HTML + Vanilla JS |
| Token | USDC (Circle, Stellar testnet) |

---

## Project Structure

```
SplitSweldo/
├── contract/
│   ├── src/
│   │   ├── lib.rs        # Soroban smart contract
│   │   └── test.rs       # Contract unit tests
│   ├── Cargo.toml
│   └── Cargo.lock
└── frontend/
    ├── index.html        # Web dashboard UI
    ├── server.js         # Express API server
    ├── .env              # Environment config
    └── package.json
```

---

## Smart Contract

**Contract ID (Testnet):** `CBC57CDUPQU6EP5IOLCTTIZEKCRLYVEIKG2SIRGM3WM35QP7XH7DOCHB`

### Functions

| Function | Caller | Description |
|---|---|---|
| `initialize(employer, worker)` | Employer | One-time setup binding employer and worker addresses |
| `set_split_rules(token, rules)` | Worker | Define recipient wallets and their BPS allocations |
| `fund_payroll(amount)` | Employer | Lock USDC into the contract |
| `release_payroll()` | Employer | Atomically distribute funds to all recipients |
| `get_split_rules()` | Anyone | Read current split configuration |
| `get_funded_amount()` | Anyone | Read currently locked USDC amount |
| `get_employer()` | Anyone | Read registered employer address |
| `get_worker()` | Anyone | Read registered worker address |

### Split Rules

Rules are defined in **basis points (BPS)**:
- 5000 BPS = 50%
- 3000 BPS = 30%
- 2000 BPS = 20%
- Total MUST equal exactly **10,000 BPS**
- Minimum 1 rule, maximum 5 rules
- Each rule must be > 0 BPS

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) with `wasm32-unknown-unknown` target
- [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/stellar-cli)
- [Node.js](https://nodejs.org/) v18+
- A funded Stellar testnet account (employer key)

### 1. Build the Contract

```bash
cd contract
cargo build --target wasm32-unknown-unknown --release
```

### 2. Deploy the Contract

```bash
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/split_sweldo.wasm \
  --source employer \
  --network testnet
```

Copy the output Contract ID.

### 3. Configure Environment

Create `frontend/.env`:

```env
CONTRACT_ID=<your_contract_id>
NETWORK=testnet
PORT=3001
```

### 4. Start the Backend

```bash
cd frontend
npm install
npm start
```

Server runs at `http://localhost:3001`.

### 5. Open the Dashboard

Open `frontend/index.html` in your browser.

---

## Usage Flow

```
Phase 1A  →  Initialize Contract
              (employer + worker addresses)
                        ↓
Phase 1B  →  Set Split Rules
              (recipient wallets + BPS allocations)
                        ↓
Phase 2   →  Fund Payroll
              (employer locks USDC into contract)
                        ↓
Phase 3   →  Release Payroll ⚡
              (atomic fan-out to all wallets)
```

---

## API Endpoints

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/status` | Contract status and funded amount |
| `POST` | `/api/initialize` | Initialize contract |
| `POST` | `/api/set-split-rules` | Save split rules on-chain |
| `POST` | `/api/fund-payroll` | Lock USDC into contract |
| `POST` | `/api/release-payroll` | Release funds to all wallets |
| `GET` | `/api/balance/:address` | Check USDC balance of an address |
| `GET` | `/api/split-rules` | Read current on-chain split rules |

---

## Running Tests

```bash
cd contract
cargo test
```

Test coverage includes:
- ✅ Successful initialization
- ✅ Double-initialize prevention
- ✅ Valid 50/30/20 split rules
- ✅ Invalid BPS total rejection
- ✅ Payroll funding and locking
- ✅ Correct atomic distribution
- ✅ Rounding dust handling (goes to last wallet)
- ✅ Double-release prevention
- ✅ Multiple pay periods

---

## Testnet Token

USDC token address on Stellar testnet:
```
CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA
```

To mint testnet USDC to your employer wallet:
```bash
stellar contract invoke \
  --id CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA \
  --source employer \
  --network testnet \
  -- mint \
  --to <employer_address> \
  --amount 1000000000
```
*(1,000,000,000 stroops = 100 USDC at 7 decimal places)*

---

## Security Notes

- The backend uses Node.js `exec()` for Stellar CLI calls — not recommended for production. A production version should use the [Stellar SDK](https://stellar.github.io/js-stellar-sdk/) directly.
- Only the registered `employer` and `worker` addresses can call their respective functions — enforced on-chain via `require_auth()`.
- This project is deployed on **testnet only** and is intended for demonstration purposes.

---

## Built With ❤️ on Stellar

Powered by [Soroban](https://soroban.stellar.org/) — Stellar's smart contract platform.

> *"Sweldo" is Filipino for salary/wages.*
