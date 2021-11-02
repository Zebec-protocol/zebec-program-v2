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
        '9fvHiQXZHVqMftRBCUxrXKLqVKT7kUty2uaobUvnCkPu') // sender/recipient address
    const wallet2: PublicKey = new PublicKey(
            '2ibSirDWk5P68ZKmQQSxUMtiWQFRuanpPfMfaYzxgSRv'); //token address
    console.log( await (await findAssociatedTokenAddress(wallet,wallet2)).toBase58()) // 
}


async function pda_seed() {

    let address = new PublicKey("BjhQP3c6tazpaiNjvbyV9pvPuA1DQQw58USHSfEjdaeU"); // sender address
    console.log(address)
    let pda_associated = "pda_associated";
    let recipient = new PublicKey("9fvHiQXZHVqMftRBCUxrXKLqVKT7kUty2uaobUvnCkPu"); // sender address
    let base58publicKey = new PublicKey('9Ayh2hS3k5fTn6V9Ks7NishUp5Jz19iosK3tYPAcNhsp'); // program address
    let validProgramAddress = await PublicKey.findProgramAddress([address.toBuffer()], base58publicKey);
    let sender_recipient = await PublicKey.findProgramAddress([address.toBuffer(),recipient.toBuffer()], base58publicKey);
    console.log(`Master PDA: `+validProgramAddress);
    console.log(`Storage PDA `+sender_recipient);

}
async function pda_seed_token() {

    let address = new PublicKey("BjhQP3c6tazpaiNjvbyV9pvPuA1DQQw58USHSfEjdaeU"); // sender address
    console.log(address)
    let recipient = new PublicKey("9fvHiQXZHVqMftRBCUxrXKLqVKT7kUty2uaobUvnCkPu"); // sender address
    let base58publicKey = new PublicKey('9Ayh2hS3k5fTn6V9Ks7NishUp5Jz19iosK3tYPAcNhsp'); // program address
    let test = "token";
    let pda_associated = "pda_associated";

    let validProgramAddress = await PublicKey.findProgramAddress([Buffer.from(test, 'utf8'),address.toBuffer()], base58publicKey);
    let sender_recipient = await PublicKey.findProgramAddress([Buffer.from(test, 'utf8'),address.toBuffer(),recipient.toBuffer()], base58publicKey);
    let pda_associate = await PublicKey.findProgramAddress([Buffer.from(pda_associated, 'utf8'),address.toBuffer(),recipient.toBuffer()], base58publicKey);

    console.log(`Master PDA: `+validProgramAddress);
    console.log(`Storage PDA `+sender_recipient);
    console.log(`pda associate `+pda_associate);

}
// pda_seed()
pda_seed_token()
main()
