# ChaosRegen

**Pure lossless universal compression for encrypted and chaotic data with full network-grade safety.**

ChaosRegen mechanically extracts entropy from random data using 
universal constants, modular mathematics, and reversible transforms.

Now includes:
- Magic file headers
- Original file size tracking
- CRC32 checksum verification

## License

ChaosRegen is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).

## Usage

```bash
cargo build --release

# Compress a file
./target/release/chaosregen compress input_file output_file

# Decompress a file
./target/release/chaosregen decompress compressed_file output_file
```

## Maintainer

Maintained by **Ashlynn**  
Contact: dastille@protonmail.com