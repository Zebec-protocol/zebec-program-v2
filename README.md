### Configure CLI
If you're on Windows, it is recommended to use WSL to run these commands

### Set CLI config url to devnet cluster
```bash
solana config set --url devnet
```
Create CLI Keypair
If this is your first time using the Solana CLI, you will need to generate a new keypair:
```bash
solana-keygen new
```
Start local Solana cluster
This example connects to a local Solana cluster by default.

### Start a local Solana cluster:

```bash
$ solana-test-validator
```
Note: You may need to do some system tuning (and restart your computer) to get the validator to run

Listen to transaction logs:
```bash
solana logs
```

### Deploy the on-chain program
```bash
solana program deploy $(pwd)/target/deploy/TheStream.so
```
