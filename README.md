# DSponsor Soroban Contracts

This repository contains the Soroban (Stellar) implementation of the DSponsor protocol, originally developed for EVM chains. DSponsor is a decentralized sponsorship protocol that enables funding through visibility shares, aligning sponsors' and sponsees' interests.

## Protocol Overview

DSponsor allows the creation of tokens representing advertising spaces. These tokens can be purchased to acquire exclusive rights to place advertisements in specific spaces (websites, apps, etc.) for defined time periods.

## Contract Architecture

### dsponsor-admin
The main contract serving as the protocol's gateway. Key features:
- Creation and management of ad spaces
- Handling ad proposals and validations
- Protocol fee collection system
- Integration with native XLM and other Stellar tokens

### dsponsor
NFT contract implementation with features:
- Token minting with multiple currency support (XLM and other Stellar tokens)
- Metadata management
- Price settings for both native and custom tokens
- Token allowlist functionality

### dsponsor-factory
Factory contract for deploying new DSponsor NFT contracts with:
- Standardized initialization
- Contract deployment tracking
- Admin rights management

## Key Differences from Ethereum Version

1. **Token Standards**
   - Replaced ERC721/ERC4907 with Soroban-specific token implementations
   - Adapted token metadata handling for Stellar ecosystem

2. **Currency Handling**
   - Integration with native XLM instead of ETH
   - Modified token approval mechanisms for Stellar's token interface

3. **Event System**
   - Implemented Soroban-specific event emission
   - Adapted indexing mechanisms for Stellar blockchain

[Previous sections remain unchanged...]

## Development

### Prerequisites
- Rust toolchain
- Stellar CLI
- Soroban CLI

### Setup and Testing

The project uses a Makefile to simplify development operations. Here are the main commands:

```bash
# Build all contracts
make build

# Run all tests
make test

# Format code
make fmt

# Clean build artifacts and rebuild
make clean

# Build and run tests and deploy in one command
make all
```

### Deployment

The Makefile includes commands for deploying contracts to the Stellar network:
Before deploying the contracts, you need to generate a Stellar keypair using the Stellar CLI. After generating your keypair, you'll need to update the Makefile by setting your key name in the `SOURCE_ACCOUNT` and `FEE_RECIPIENT` variables.


```bash
# Deploy factory contract
make deploy-factory

# Deploy admin contract
make deploy

# Initialize admin contract with configuration
make initialize
```

Deployment variables can be configured in the Makefile:
- `SOURCE_ACCOUNT`: The deployer account
- `FEE_RECIPIENT`: Account receiving protocol fees
- `NETWORK`: Target network (default: testnet)
- `NATIVE_XLM`: Native token contract address
- Contract addresses are automatically set after deployment

### Environment Variables

Important contract addresses (testnet):

## Testing Networks

The contracts are deployed on:
- Stellar Testnet 
- Stellar Mainnet (pending)

## Security Considerations

- All contracts include comprehensive test coverage
- Implemented Stellar-specific security best practices
- Adapted access control mechanisms for Soroban

## Contributing

1. Fork the repository
2. Create a feature branch
3. Submit a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details