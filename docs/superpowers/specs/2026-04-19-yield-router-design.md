# Yield Router Design Specification

## Overview
A "Smart Router" Solana smart contract (using Anchor) designed for remote workers, freelancers, and small businesses. It acts as a primary payment receiving address that automatically splits incoming funds (USDC) into three streams: direct liquid wallet balance, a yield-bearing savings vault (Kamino/Drift), and a time-locked release vault.

## Architecture & Accounts

### State: `UserProfile` (PDA)
The core configuration account per user (remote worker/business).
- `owner`: Pubkey (The business or worker's main wallet).
- `split_main_pct`: u8 (Percentage to liquid wallet, e.g., 50%).
- `split_yield_pct`: u8 (Percentage to yield protocol, e.g., 30%).
- `split_lock_pct`: u8 (Percentage to time-locked vault, e.g., 20%).
- `weekly_release_amount`: u64 (USDC amount allowed per week from lock).
- `last_claim_timestamp`: i64 (Unix timestamp of the last weekly claim).
- `yield_protocol`: Enum `YieldProtocol { Kamino, Drift }` (Preferred yield route).
- `bump`: u8 (PDA bump for security).

### Token Accounts (PDAs)
- **Router Vault**: The public-facing USDC address. Senders (employers/clients) deposit here.
- **Lock Vault**: USDC vault holding the `split_lock_pct` funds for weekly release.
- **Yield Vault**: Token account holding the yield-bearing receipt tokens (e.g., Kamino kTokens).

## Core Instructions

### 1. `initialize_profile`
- **Purpose**: Creates the `UserProfile` PDA and initializes the three vault Token Accounts.
- **Parameters**: `split_main_pct`, `split_yield_pct`, `split_lock_pct` (must sum to 100), `weekly_release_amount`, `yield_protocol`.

### 2. `update_profile`
- **Purpose**: Allows the `owner` to modify their split percentages, weekly release amount, or yield preference.

### 3. `process_payment`
- **Purpose**: The automation crank. Anyone (a backend bot or the owner) can call this.
- **Action**:
  1. Reads the total USDC balance in the **Router Vault**.
  2. Calculates the split amounts based on the `UserProfile` percentages.
  3. Transfers the `split_main_pct` amount directly to the `owner` wallet.
  4. Transfers the `split_lock_pct` amount to the **Lock Vault**.
  5. Performs a CPI (Cross-Program Invocation) to the selected `yield_protocol` (e.g., Kamino Finance) to deposit the `split_yield_pct` amount and receive yield tokens into the **Yield Vault**.

### 4. `claim_locked_funds`
- **Purpose**: Allows the `owner` to withdraw their time-locked funds.
- **Action**: Checks if `current_timestamp >= last_claim_timestamp + 7_days`. If true, transfers `weekly_release_amount` from the **Lock Vault** to the `owner` and updates `last_claim_timestamp`.

### 5. `withdraw_yield`
- **Purpose**: Allows the `owner` to pull funds out of the yield protocol.
- **Action**: Performs a CPI to the `yield_protocol` to burn the receipt tokens (from the **Yield Vault**) and withdraw the underlying USDC + earned yield back to the `owner`.

## Testing Strategy
- **Framework**: Anchor with TypeScript (`ts-mocha`).
- **Environment**: Mainnet Forking (`--skip-local-validator`).
- **Setup**: The `Anchor.toml` will be configured to clone the Kamino Finance program (`KLend...`), its lending market state, and the USDC mint from mainnet to ensure CPI tests are realistic.
