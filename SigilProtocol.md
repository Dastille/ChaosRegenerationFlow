# The Sigil Protocol (.sg1): Regenerative Data Representation

## Abstract

Sigil is a regenerative protocol for secure data representation, combining chaotic transformations, cryptographic seeding, and embedded verification. Its `.sg1` file format enables post-cloud workflows, offline use, and deterministic reconstitution of data. The design is directly derived from the ChaosRegen algorithm.

## Format Overview

```
[ Magic Header | Original Length | CRC32 | Transformed Data ]
```

## Features

- **Lossless** transformation  
- **Deterministic** reproducibility  
- **Resilient** to corruption (self-verifying)  
- **No cloud needed** — works offline  
- **Post-quantum aligned** with chaos theory roots  

## Compression Process

1. XOR with chaos masks (π, φ, logistic)  
2. Modular transformation (mod 257)  
3. Matrix shuffle  
4. Header + CRC added  
5. Output written as `.sg1`  

## Decompression

1. Header validated  
2. CRC32 checked  
3. Reversal of chaos transforms  
4. Original file reconstructed perfectly  

## Use Cases

- Secure file transfer  
- Archival of encrypted blobs  
- Air-gapped restoration  
- Lightweight replication  
- Stateless container regeneration  

## Forensic and Legal Use Case: Zero-Knowledge Verification

Sigil's `.sg1` format includes self-verifying structures that make it suitable for adversarial and legal environments. Its design allows **proof of integrity, authenticity, and non-tampering** — without requiring decryption or revealing file contents. This is essential in legal, forensic, and compliance workflows.

### How It Works

1. **Magic Header (`CRGN`)**
   - Identifies the file format
   - Prevents filetype spoofing or trick execution
   - Retained from ChaosRegen for compatibility (may evolve to `SGIL`)

2. **Original File Length**  
   - Encoded in the header  
   - Detects truncation or hidden injection attempts  

3. **CRC32 Checksum**  
   - Fingerprint of the compressed/transformed data  
   - Used for zero-knowledge verification (without decompression)  

4. **Deterministic Transformation**  
   - Guarantees that the same input + same seed = same output  
   - Enables reproducible verification across systems and time  

### Why It Matters

- ✅ **Court-admissible validation**  
  - A file can be proven authentic without decrypting it  
  - Chain of custody doesn't require access to the contents  

- ✅ **Whistleblower-safe**  
  - Files can be shared and validated without ever opening them  

- ✅ **Evidence-grade audit logging**  
  - Systems can verify log archives without reading them  

- ✅ **Air-gapped verification**  
  - Integrity checks are possible without cloud or internet access  

### Example Scenario: Digital Evidence Chain

1. A journalist receives a compressed `.sg1` file from a whistleblower.  
2. They don't need to open it — they validate it using the built-in checksum and header.  
3. Months later, a court requests validation. The journalist re-runs the checksum against a new copy.  
4. Because the CRC32 and length match exactly, the file is proven unaltered since its original creation.  

This enables **legally sound, cryptographically verifiable compression** of any type of sensitive data.

## License

AGPL-3.0

## Author

Ashlynn  
