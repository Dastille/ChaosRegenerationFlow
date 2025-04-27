
# ChaosRegen

**Pure lossless universal compression for encrypted and chaotic data.**

ChaosRegen mechanically extracts entropy from random data using 
universal constants, modular mathematics, and reversible transforms, 
achieving real compression without relying on redundancy detection.

Unlike earlier chaotic generation models, ChaosRegen guarantees strict 
bit-for-bit lossless compression and decompression with no residual patching.

Built entirely in Rust.

## Usage

```bash
# Build
cargo build --release

# Compress a file
./target/release/chaosregen compress input_file output_file

# Decompress a file
./target/release/chaosregen decompress compressed_file output_file
```

## License

MIT License.
