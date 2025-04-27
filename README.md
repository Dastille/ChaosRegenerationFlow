# ChaosRegen

**Pure lossless universal compression for encrypted and chaotic data.**

ChaosRegen mechanically extracts entropy from random data using 
universal constants, modular mathematics, and reversible transforms, 
achieving real compression without relying on redundancy detection.

Built entirely in Rust.

## License

ChaosRegen is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).

See the LICENSE file for details.

## Usage

```bash
cargo build --release

# Compress a file
./target/release/chaosregen compress input_file output_file

# Decompress a file
./target/release/chaosregen decompress compressed_file output_file
```

## Philosophy

ChaosRegen is committed to ensuring that all users retain the right to study, modify, and share improvements,
even when software is used over a network. This project upholds the spirit of free software in all deployments.
