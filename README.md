Event Tracker is a basic tool to search for withdrawal events emitted by Starkway and compile relevant data pertaining to such events.

# How to use the tool

1. `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` - If you need to install rust
2. `git clone https://github.com/zkxteam/event-tracker-v2.git`
3. `cd event-tracker-v2`
4. `cargo build --release`
5. `cargo run --release -- -f <from block-number> -t <to block-number> -n <network mainnet/sepolia>`

A csv output file is created **output.csv** with the following header

> **Tx Hash, Timestamp, From L2 Addr, To L1 Recipient, L1 Token, Amount, Is completed on L1 (zero = yes), Message Hash**
