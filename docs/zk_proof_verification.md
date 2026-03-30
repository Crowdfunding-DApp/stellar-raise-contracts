# Zero-Knowledge Proof Verification

## Overview

This module implements zero-knowledge proof (ZKP) verification for the Stellar Raise crowdfunding contract. ZKPs allow proving certain properties about data without revealing the underlying data, improving both gas efficiency and privacy.

## Features

- **Gas Efficiency**: Reduces computational overhead by verifying proofs instead of performing expensive computations
- **Privacy**: Contributors can prove properties about their contributions without revealing sensitive information
- **Security**: Cryptographically secure proof verification

## Usage

### Verifying a Proof

```rust
use crate::zk_proof_verification::{verify_zkp, ZkProof, generate_challenge};

let proof = ZkProof { ... };
let public_key = ...;
let statement = 1000; // e.g., minimum contribution

if verify_zkp(&env, &proof, &public_key, statement) {
    // Proof is valid
}
```

### Generating a Challenge

```rust
let challenge = generate_challenge(&env, &commitment, &public_key);
```

## Security Considerations

- The current implementation is a simplified version for demonstration
- In production, use a full cryptographic ZKP library
- Ensure proofs are generated correctly on the client side
- Validate all inputs to prevent malleability attacks

## Testing

Run the tests with:

```bash
cargo test zk_proof_verification
```

## Integration

The ZKP verification is integrated into the `contribute` function to verify contribution validity efficiently.

## Future Improvements

- Implement full zk-SNARK verification
- Add support for range proofs
- Integrate with external ZKP services for enhanced privacy