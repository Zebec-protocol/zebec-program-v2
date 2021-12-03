import { PublicKey,Keypair } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { SlowBuffer } from 'buffer';
import { fromPairs } from 'lodash';
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
        'ErAykG8kXqpAjGGWbZ9BuQMq7j5SEk9fhrXV8JRpysx4') // sender/recipient address
    const wallet2: PublicKey = new PublicKey(
            '2ibSirDWk5P68ZKmQQSxUMtiWQFRuanpPfMfaYzxgSRv'); //token address
    console.log( await (await findAssociatedTokenAddress(wallet,wallet2)).toBase58()) //

    
}


async function pda_seed() {
    const wallet: PublicKey = new PublicKey(
        'ErAykG8kXqpAjGGWbZ9BuQMq7j5SEk9fhrXV8JRpysx4') // sender/recipient address
    const wallet2: PublicKey = new PublicKey(
            '2ibSirDWk5P68ZKmQQSxUMtiWQFRuanpPfMfaYzxgSRv'); //token address
    let address = new PublicKey("J75jd3kjsABQSDrEdywcyhmbq8eHDowfW9xtEWsVALy9"); // sender address
    console.log(address)
    let recipient = new PublicKey("BvNbvbaE6NKdGXMYK3Vtrosq46vdxDwif4SJ9qLzEJ7b"); // sender address
    let base58publicKey = new PublicKey('7FNWTfCo3AyRBFCvr49daqKHehdn2GjNgpjuTsqy5twk'); // program address
    let base58publicKeyspl  =new PublicKey ("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
    let sender_recipientw = await PublicKey.findProgramAddress([wallet.toBuffer(),base58publicKeyspl.toBuffer(),wallet2.toBuffer()], SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID);
    let validProgramAddress = await PublicKey.findProgramAddress([address.toBuffer()], base58publicKey);
    let sender_recipient = await PublicKey.findProgramAddress([address.toBuffer(),recipient.toBuffer()], base58publicKey);
    let stringofwithdraw = "withdraw_sol"
    let withdraw_data = await PublicKey.findProgramAddress([Buffer.from(stringofwithdraw),address.toBuffer()], base58publicKey);

    console.log(`Master PDA: `+validProgramAddress);
    console.log(`withdraw_data `+withdraw_data);

}

main()
pda_seed()
