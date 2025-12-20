# DSponsor Contract

This is a consolidated Soroban smart contract for the DSponsor platform, which manages sponsorship NFTs and related functionality.

## Overview

The DSponsor contract combines the functionality of multiple contracts into a single, cohesive contract:

1. **Admin Contract**: Manages administrative functions, including setting fees, updating addresses, and managing sponsorship properties.
2. **NFT Contract**: Handles the creation and management of sponsorship NFTs.
3. **Factory Contract**: Creates and manages NFT instances for different sponsees.

## Features

- **Admin Functions**: Initialize the contract, update admin and treasury addresses, set minting fees and referral rewards.
- **Sponsorship Properties**: Add and retrieve properties for sponsees.
- **NFT Management**: Create, transfer, and manage sponsorship NFTs.
- **Factory Pattern**: Create and manage NFT instances for different sponsees.

## Contract Structure

The contract is organized into the following modules:

- `lib.rs`: Main contract implementation with admin functions and high-level NFT operations.
- `error.rs`: Error definitions for the contract.
- `nft.rs`: NFT-specific functionality.
- `factory.rs`: Factory pattern implementation for creating NFT instances.
- `test.rs`: Test suite for the contract.

## Usage

### Initialization

```rust
// Initialize the contract with admin and treasury addresses
client.initialize(&admin, &treasury);
```

### Admin Functions

```rust
// Update admin address
client.update_admin(&current_admin, &new_admin);

// Update treasury address
client.update_treasury(&admin, &new_treasury);

// Update minting fee
client.update_minting_fee(&admin, &new_fee);

// Update referral reward
client.update_referral_reward(&admin, &new_reward);
```

### Sponsorship Properties

```rust
// Add sponsorship properties
client.add_sponsorship_properties(&sponsee, &properties);

// Get sponsorship properties
let properties = client.get_sponsorship_properties(&sponsee);
```

### NFT Operations

```rust
// Create a new sponsorship NFT
let nft_contract = client.create_sponsorship_nft(&sponsee, &metadata);

// Get the sponsee for an NFT
let sponsee = client.get_nft_sponsee(&nft_contract);

// Get NFT metadata
let metadata = client.get_nft_metadata(&sponsee);

// Get all NFT contracts
let all_contracts = client.get_all_nft_contracts();

// Get NFT contracts by sponsee
let sponsee_contracts = client.get_nft_contracts_by_sponsee(&sponsee);
```

## Testing

Run the tests with:

```bash
cargo test
```

## License

This project is licensed under the MIT License - see the LICENSE file for details. 