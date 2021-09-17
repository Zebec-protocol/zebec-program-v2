cargo build-bpf   
solana program deploy $(pwd)/target/deploy/TheStream.so
node index.js init
node index.js withdraw
