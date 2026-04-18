#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, Vec,
};

// ─── Storage keys ─────────────────────────────────────────────────────────────
#[contracttype]
pub enum DataKey {
    SplitRules,
    FundedAmount,
    Employer,
    Worker,
    Initialized,
}

// ─── Data structures ──────────────────────────────────────────────────────────

/// One slice of the payroll fan-out.
/// `bps` is basis points (100 bps = 1%, so total must equal 10_000).
#[contracttype]
#[derive(Clone)]
pub struct SplitRule {
    pub wallet: Address,
    pub bps: u32,   // e.g. 5000 = 50%, 3000 = 30%, 2000 = 20%
}

#[contracttype]
#[derive(Clone)]
pub struct SplitRules {
    pub rules: Vec<SplitRule>,
    pub token: Address,
}

// ─── Contract ─────────────────────────────────────────────────────────────────

#[contract]
pub struct SplitSweldo;

#[contractimpl]
impl SplitSweldo {
    // ── Phase 0: one-time init ─────────────────────────────────────────────

    /// Called once by the deployer to bind the employer and worker addresses.
    pub fn initialize(env: Env, employer: Address, worker: Address) {
        employer.require_auth(); // ← AUTH CHECK FIRST (fixed)

        if env.storage().instance().has(&DataKey::Initialized) {
            panic!("already initialized");
        }

        env.storage().instance().set(&DataKey::Employer, &employer);
        env.storage().instance().set(&DataKey::Worker, &worker);
        env.storage().instance().set(&DataKey::Initialized, &true);
    }

    // ── Phase 1a: worker configures the split ─────────────────────────────

    /// Worker defines up to 5 destination wallets and their share in basis points.
    /// The sum of all `bps` values MUST equal exactly 10_000 (= 100%).
    /// Only the registered worker address may call this.
    pub fn set_split_rules(env: Env, token: Address, rules: Vec<SplitRule>) {
        let worker: Address = env
            .storage()
            .instance()
            .get(&DataKey::Worker)
            .expect("not initialized");
        worker.require_auth();

        // Validate: between 1 and 5 destinations
        if rules.is_empty() || rules.len() > 5 {
            panic!("must have 1–5 split rules");
        }

        // Validate: total basis points == 10_000
        let total: u32 = rules.iter().map(|r| r.bps).sum();
        if total != 10_000 {
            panic!("split rules must sum to 10000 bps (= 100%)");
        }

        // Validate: no individual slice is zero
        for rule in rules.iter() {
            if rule.bps == 0 {
                panic!("each split slice must be > 0 bps");
            }
        }

        env.storage()
            .instance()
            .set(&DataKey::SplitRules, &SplitRules { rules, token });
    }

    // ── Phase 1b: employer locks the payroll USDC ─────────────────────────

    /// Employer transfers `amount` of the token into this contract.
    /// Overwrites any previously funded (unreleased) balance.
    /// Only the registered employer address may call this.
    pub fn fund_payroll(env: Env, amount: i128) {
        let employer: Address = env
            .storage()
            .instance()
            .get(&DataKey::Employer)
            .expect("not initialized");
        employer.require_auth();

        if amount <= 0 {
            panic!("amount must be positive");
        }

        let split: SplitRules = env
            .storage()
            .instance()
            .get(&DataKey::SplitRules)
            .expect("split rules not set yet");

        // Pull funds from employer into the contract
        let token_client = token::Client::new(&env, &split.token);
        token_client.transfer(&employer, &env.current_contract_address(), &amount);

        env.storage()
            .instance()
            .set(&DataKey::FundedAmount, &amount);
    }

    // ── Phase 2: employer triggers the atomic fan-out ─────────────────────

    /// Employer calls this on payday.
    /// Soroban reads stored split rules, then fans out the entire funded balance
    /// to all destination wallets in a single atomic transaction.
    /// If any transfer fails, the entire transaction is rolled back.
    pub fn release_payroll(env: Env) {
        let employer: Address = env
            .storage()
            .instance()
            .get(&DataKey::Employer)
            .expect("not initialized");
        employer.require_auth();

        let amount: i128 = env
            .storage()
            .instance()
            .get(&DataKey::FundedAmount)
            .expect("no funded amount — call fund_payroll first");

        if amount <= 0 {
            panic!("nothing to release");
        }

        let split: SplitRules = env
            .storage()
            .instance()
            .get(&DataKey::SplitRules)
            .expect("split rules not set");

        let token_client = token::Client::new(&env, &split.token);
        let contract_addr = env.current_contract_address();

        let mut distributed: i128 = 0;
        let rule_count = split.rules.len() as usize;

        for (i, rule) in split.rules.iter().enumerate() {
            let slice = if i == rule_count - 1 {
                // Last wallet gets any rounding dust
                amount - distributed
            } else {
                // bps / 10_000 * amount  (integer-safe)
                (amount * rule.bps as i128) / 10_000
            };

            if slice > 0 {
                token_client.transfer(&contract_addr, &rule.wallet, &slice);
                distributed += slice;
            }
        }

        // Clear the funded amount — cannot double-release
        env.storage().instance().remove(&DataKey::FundedAmount);
    }

    // ── Read-only helpers ─────────────────────────────────────────────────

    /// Returns the current split rules (wallets + percentages).
    pub fn get_split_rules(env: Env) -> Option<SplitRules> {
        env.storage().instance().get(&DataKey::SplitRules)
    }

    /// Returns the currently funded (unreleased) USDC amount, or 0.
    pub fn get_funded_amount(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::FundedAmount)
            .unwrap_or(0)
    }

    /// Returns the registered employer address.
    pub fn get_employer(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Employer)
    }

    /// Returns the registered worker address.
    pub fn get_worker(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Worker)
    }
}

#[cfg(test)]
mod test;