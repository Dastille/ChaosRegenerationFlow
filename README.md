# ChaosRegen

**Pure lossless universal compression for encrypted and chaotic data with full network-grade safety.**

ChaosRegen mechanically extracts entropy from random data using
universal constants, modular mathematics, and reversible transforms.

Now includes:
- Magic file headers
- Original file size tracking
- CRC32 checksum verification
- Recursive microzone passes (up to three) with entropy sensing
- Modular prime rotation across {251, 241, 239}

Sigil extends ChaosRegen with a self-verifying `.sg1` format. See
[SigilProtocol.md](SigilProtocol.md) for details on the protocol and how
it derives from the ChaosRegen engine.

## License

ChaosRegen is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).

## Usage

```bash
cargo build --release

# Compress a file
./target/release/chaosregen compress <input_file> <output_file>

# Decompress a file
./target/release/chaosregen decompress <compressed_file> <output_file>

# Verify an `.sg1` file without decompressing
./target/release/chaosregen verify <compressed_file>

# Attempt to repair a corrupted `.sg1` file
./target/release/chaosregen repair <compressed_file> <output_file>
Maintained by **Ashlynn**
Contact: dastille@protonmail.com
