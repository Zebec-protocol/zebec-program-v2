import { PublicKey,Keypair } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { SlowBuffer } from 'buffer';
const crypto = require('crypto');

const SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID: PublicKey = new PublicKey(
  'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL',
);

async function findAssociatedTokenAddress(
    walletAddress: PublicKey,
    tokenMintAddress: PublicKey
): Promise<PublicKey> {
    return (await PublicKey.findProgramAddress(
        [
            walletAddress.toBuffer(),
            TOKEN_PROGRAM_ID.toBuffer(),
            tokenMintAddress.toBuffer(),
        ],
        SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID
    ))[0];
}


async function main() {
    const wallet: PublicKey = new PublicKey(
        '6Yn4TJryWFtwhKSEqqVKDEpTpjJ5fKFRPCpS5SMeQhh9') // sender/recipient address
    const wallet2: PublicKey = new PublicKey(
            '2ibSirDWk5P68ZKmQQSxUMtiWQFRuanpPfMfaYzxgSRv'); //token address
    console.log( await (await findAssociatedTokenAddress(wallet,wallet2)).toBase58()) // 
}


async function pda_seed() {

    let address = new PublicKey("CwQnKJtXXNTtSMgygMMBWWqFocVUzGTXG68MtSE3Uy8k"); // sender address
    let base58publicKey = new PublicKey('9Ayh2hS3k5fTn6V9Ks7NishUp5Jz19iosK3tYPAcNhsp'); // program address
    let validProgramAddress = await PublicKey.findProgramAddress([address.toBuffer()], base58publicKey);
    console.log(`Valid Program Address: `+validProgramAddress);
}
main()
pda_seed()

const i = {
    "type": "Buffer",
    "data": [
        186,
        70,
        53,
        44,
        175,
        66,
        208,
        121,
        208,
        139,
        237,
        230,
        73,
        62,
        63,
        202,
        246,
        98,
        0,
        90,
        195,
        196,
        85,
        106,
        206,
        18,
        65,
        231,
        238,
        230,
        189,
        68,
        173,
        47,
        165,
        21,
        247,
        240,
        172,
        184,
        112,
        177,
        124,
        188,
        216,
        251,
        137,
        78,
        252,
        238,
        131,
        155,
        78,
        172,
        142,
        90,
        77,
        55,
        202,
        162,
        73,
        67,
        164,
        12
    ]
}