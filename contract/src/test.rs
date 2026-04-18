#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    token, Address, Env, IntoVal, Vec,
};

// ─── Test helpers ─────────────────────────────────────────────────────────────

struct TestSetup {
    env: Env,
    contract_id: Address,
    token_id: Address,
    employer: Address,
    worker: Address,
    wallet_a: Address,
    wallet_b: Address,
    wallet_c: Address,
}

fn setup() -> TestSetup {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SplitSweldo);
    let employer = Address::generate(&env);
    let worker = Address::generate(&env);
    let wallet_a = Address::generate(&env);
    let wallet_b = Address::generate(&env);
    let wallet_c = Address::generate(&env);

    // Deploy a native USDC mock token
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    // Mint 10_000 USDC (7 decimals → 10_000_0000000) to employer
    token_admin_client.mint(&employer, &1_000_000_0000000);

    TestSetup {
        env,
        contract_id,
        token_id,
        employer,
        worker,
        wallet_a,
        wallet_b,
        wallet_c,
    }
}

fn make_rules(s: &TestSetup, bps: (u32, u32, u32)) -> Vec<SplitRule> {
    let mut rules = Vec::new(&s.env);
    rules.push_back(SplitRule { wallet: s.wallet_a.clone(), bps: bps.0 });
    rules.push_back(SplitRule { wallet: s.wallet_b.clone(), bps: bps.1 });
    rules.push_back(SplitRule { wallet: s.wallet_c.clone(), bps: bps.2 });
    rules
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[test]
fn test_initialize_ok() {
    let s = setup();
    let client = SplitSweldoClient::new(&s.env, &s.contract_id);
    client.initialize(&s.employer, &s.worker);

    assert_eq!(client.get_employer(), Some(s.employer.clone()));
    assert_eq!(client.get_worker(), Some(s.worker.clone()));
}

#[test]
#[should_panic(expected = "already initialized")]
fn test_double_initialize_panics() {
    let s = setup();
    let client = SplitSweldoClient::new(&s.env, &s.contract_id);
    client.initialize(&s.employer, &s.worker);
    client.initialize(&s.employer, &s.worker); // must panic
}

#[test]
fn test_set_split_rules_valid_50_30_20() {
    let s = setup();
    let client = SplitSweldoClient::new(&s.env, &s.contract_id);
    client.initialize(&s.employer, &s.worker);

    let rules = make_rules(&s, (5000, 3000, 2000));
    client.set_split_rules(&s.token_id, &rules);

    let stored = client.get_split_rules().expect("rules missing");
    assert_eq!(stored.rules.len(), 3);
}

#[test]
#[should_panic(expected = "sum to 10000 bps")]
fn test_set_split_rules_bad_total() {
    let s = setup();
    let client = SplitSweldoClient::new(&s.env, &s.contract_id);
    client.initialize(&s.employer, &s.worker);

    let rules = make_rules(&s, (5000, 3000, 1000)); // only 90% → panic
    client.set_split_rules(&s.token_id, &rules);
}

#[test]
fn test_fund_payroll_locks_amount() {
    let s = setup();
    let client = SplitSweldoClient::new(&s.env, &s.contract_id);
    client.initialize(&s.employer, &s.worker);
    client.set_split_rules(&s.token_id, &make_rules(&s, (5000, 3000, 2000)));

    let payroll: i128 = 1_000_0000000; // 1,000 USDC
    client.fund_payroll(&payroll);

    assert_eq!(client.get_funded_amount(), payroll);

    // Contract should now hold the tokens
    let token_client = token::Client::new(&s.env, &s.token_id);
    assert_eq!(token_client.balance(&s.contract_id), payroll);
}

#[test]
fn test_release_payroll_splits_correctly_50_30_20() {
    let s = setup();
    let client = SplitSweldoClient::new(&s.env, &s.contract_id);
    client.initialize(&s.employer, &s.worker);
    client.set_split_rules(&s.token_id, &make_rules(&s, (5000, 3000, 2000)));

    let payroll: i128 = 1_000_0000000; // 1,000 USDC (7 decimals)
    client.fund_payroll(&payroll);
    client.release_payroll();

    let token_client = token::Client::new(&s.env, &s.token_id);

    // Check each wallet received correct amount
    assert_eq!(token_client.balance(&s.wallet_a), 500_0000000);  // 50%
    assert_eq!(token_client.balance(&s.wallet_b), 300_0000000);  // 30%
    assert_eq!(token_client.balance(&s.wallet_c), 200_0000000);  // 20%

    // Contract should be empty
    assert_eq!(token_client.balance(&s.contract_id), 0);

    // Funded amount cleared
    assert_eq!(client.get_funded_amount(), 0);
}

#[test]
fn test_rounding_dust_goes_to_last_wallet() {
    let s = setup();
    let client = SplitSweldoClient::new(&s.env, &s.contract_id);
    client.initialize(&s.employer, &s.worker);

    // 3333 + 3333 + 3334 = 10000 bps
    let mut rules = Vec::new(&s.env);
    rules.push_back(SplitRule { wallet: s.wallet_a.clone(), bps: 3333 });
    rules.push_back(SplitRule { wallet: s.wallet_b.clone(), bps: 3333 });
    rules.push_back(SplitRule { wallet: s.wallet_c.clone(), bps: 3334 });
    client.set_split_rules(&s.token_id, &rules);

    // 10 USDC — doesn't divide evenly at 3333 bps
    let payroll: i128 = 10_0000000;
    client.fund_payroll(&payroll);
    client.release_payroll();

    let token_client = token::Client::new(&s.env, &s.token_id);
    let a = token_client.balance(&s.wallet_a);
    let b = token_client.balance(&s.wallet_b);
    let c = token_client.balance(&s.wallet_c);

    // Total must always equal payroll — no dust lost
    assert_eq!(a + b + c, payroll);
    assert_eq!(token_client.balance(&s.contract_id), 0);
}

#[test]
#[should_panic]
fn test_double_release_panics() {
    let s = setup();
    let client = SplitSweldoClient::new(&s.env, &s.contract_id);
    client.initialize(&s.employer, &s.worker);
    client.set_split_rules(&s.token_id, &make_rules(&s, (5000, 3000, 2000)));
    client.fund_payroll(&1_000_0000000);
    client.release_payroll();
    client.release_payroll(); // must panic — already released
}

#[test]
fn test_multiple_pay_periods() {
    let s = setup();
    let client = SplitSweldoClient::new(&s.env, &s.contract_id);
    client.initialize(&s.employer, &s.worker);
    client.set_split_rules(&s.token_id, &make_rules(&s, (5000, 3000, 2000)));

    let payroll: i128 = 500_0000000;

    // Pay period 1
    client.fund_payroll(&payroll);
    client.release_payroll();

    // Pay period 2
    client.fund_payroll(&payroll);
    client.release_payroll();

    let token_client = token::Client::new(&s.env, &s.token_id);
    // Wallet A should have received 50% × 2 periods = 500 USDC
    assert_eq!(token_client.balance(&s.wallet_a), 500_0000000);
    assert_eq!(token_client.balance(&s.wallet_b), 300_0000000);
    assert_eq!(token_client.balance(&s.wallet_c), 200_0000000);
}