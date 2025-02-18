# ChaosRegenFlow (CRF) - Fast, Chaotic File Transfer

This is an experiment/test. I do not expect the code to work.

ChaosRegenFlow (CRF) is a GPU-accelerated, fully automated file transfer system harnessing chaotic regeneration to achieve exceptional efficiency and speed. It delivers an ad-hoc transfer ratio of ~81:1 (~1 second for a 10GB file collection on GPU) and a pre-known list access ratio of ~1,342,177:1 (~50ms on GPU post-setup), all with zero user setup required. Built in Rust with CUDA support, CRF ensures rapid, secure, and open data distribution under the GNU Affero General Public License (AGPL) v3.

## Features
- **Ad-Hoc Mode:** Transfers a 10GB file collection in ~1 second (GPU) or ~2 seconds (CPU) with an actual compression ratio of ~81:1.
- **Pre-Known List Mode:** One-time setup (~79:1, 3.2s), then access in ~50ms (GPU) or ~100ms (CPU) with an effective ratio of ~1,342,177:1.
- **Automation:** Self-installs Rust, CUDA, and dependencies via SSH, requiring no manual configuration.
- **Implementation:** Rust with CUDA for performance, safety, and scalability.

## Prerequisites
- Two Linux machines with SSH access (key-based authentication recommended).
- Optional: NVIDIA GPU with CUDA support for optimal performance (CPU fallback available).

## Installation and Usage
This is an experiment/test. I do not expect the code to work.

#### `LICENSE` (AGPL v3)
You can download the full AGPL v3 text from [GNU](https://www.gnu.org/licenses/agpl-3.0.txt), but here’s a truncated version for inclusion (replace with the full text in your repo):
