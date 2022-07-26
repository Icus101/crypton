import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY,Keypair,LAMPORTS_PER_SOL } from "@solana/web3.js";
import { assert } from "chai";
import { Crypton } from "../target/types/crypton";
import {Wallet} from "@project-serum/anchor";
import { TOKEN_PROGRAM_ID,createMint, Account } from "@solana/spl-token";
import { getOrCreateAssociatedTokenAccount } from "@solana/spl-token";

const ACCOUNT_SEED = "acc-staking-pool";
const MINT_SEED = "mint-staking-pool";
const WALLET_SEED = "wallet";
//const VAULT_SEED = "mys-staking-pool";
let connection = anchor.getProvider().connection;
let _receiver = null;
let mintKey = null;
const program = anchor.workspace.Crypton as Program<Crypton>;
const chrtKeypair = anchor.web3.Keypair.generate();
// const chrtPubkey = chrtKeypair.publicKey;

const feePayer = async (lamports = LAMPORTS_PER_SOL) => {
  const wallet =anchor.web3.Keypair.generate();
  const signature = await connection.requestAirdrop(wallet.publicKey, lamports);
  await connection.confirmTransaction(signature);
  return wallet;
}


const getMintProgramAddress = async ():Promise<[PublicKey,number]> => {
  return await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from(MINT_SEED)],
    program.programId,
  )
}


const mintAdd = async () : Promise<PublicKey> => {
  const [mintPda,_bump] = await getMintProgramAddress();
  return await createMint(
    connection,
    await feePayer(),
    mintPda,
    null,
    3,
    chrtKeypair
  );
  
}


const ata = async (owner:PublicKey):Promise<Account> => {
  mintKey = await mintAdd();
  return await getOrCreateAssociatedTokenAccount(
    connection,
    await feePayer(),
    mintKey,
    owner,
    false,
  )
}


describe("crypton",  () => {
  // Configure the client to use the local cluster.
  let provider = anchor.Provider.env();
  anchor.setProvider(provider);
  const admin = provider.wallet;
  


 

  it("Is initializes fundraising!", async () => {
    // Add your test here.
    //const admin = await feePayer()
    const receivers =  await feePayer();
    const [adminPda,_bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(ACCOUNT_SEED)],
      program.programId
    );
    const [vaultPda,_vaultBump] = await anchor.web3.PublicKey.findProgramAddress(
      [admin.publicKey.toBuffer()],
      program.programId
    );
    const tx = await program.rpc.initializeFundraising(
      "description",{
        accounts : {
          fundraiserState : adminPda,
          vault : vaultPda,
          fundStarter :  admin.publicKey,
          systemProgram : SystemProgram.programId,
        },
      });
    console.log("Your transaction signature", tx);
    console.log("the admin is :",vaultPda);
    console.log("the bump is :",_vaultBump);
    _receiver = receivers;
  });

  it("Is donates sol!", async () => {
    const vaultAddr = anchor.web3.Keypair.generate();
    const [mintPda,bump1] = await getMintProgramAddress();
    const donators =  (await feePayer(10_000_000_000_000))
    const referalKP = anchor.web3.Keypair.generate();
    const refTA = (await ata(referalKP.publicKey)).address;
    //const receivers =  await feePayer();
    const [adminPda,_bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(ACCOUNT_SEED)],
      program.programId
    );
    const [vaultPda,_vaultBump] = await anchor.web3.PublicKey.findProgramAddress(
      [admin.publicKey.toBuffer()],
      program.programId
    );
    const amount  = new anchor.BN(1000);
    const tx = await program.rpc.donate( amount,bump1,{
      accounts : {
        fundraiserState : adminPda,
        vault : vaultPda,
        donor : donators.publicKey,
        feeVault : vaultAddr.publicKey,
        fundStarter : admin.publicKey,
        systemProgram: SystemProgram.programId,
        chrtMintAuthority:mintPda,
        referchrtTokenAccount:refTA,
        chrtMint:mintKey,
        tokenProgram:TOKEN_PROGRAM_ID,
      },
      signers : [donators]
    });
    console.log("Your transaction sig : ",tx);
  })

  it("withdraw money from the fundraising", async () => {
    const receivers = (await feePayer());
    const amount  = new anchor.BN(100000);
    //const destinations = anchor.web3.Keypair.generate()
    const [adminPda,_bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(ACCOUNT_SEED)],
      program.programId
    );
    const [vaultPda,vaultBump] = await anchor.web3.PublicKey.findProgramAddress(
      [admin.publicKey.toBuffer()],
      program.programId
    );
    console.log("the vault is :",vaultPda);
    console.log("the bump is :",vaultBump);
    let tx = await program.rpc.withdraw(amount,{
      accounts : {
        fundraiserState : adminPda,
        vault : vaultPda,
        fundStarter : admin.publicKey,
        destination: receivers.publicKey,
        systemProgram : SystemProgram.programId
      },
      //signers : [receivers]
    });
    console.log("tx is : ",tx);
  })

  it("donate chrt tokens", async () => {
    const [adminPda,_bump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(ACCOUNT_SEED)],
      program.programId
    );
    await program.rpc.donateChrt({ 
      accounts : {
        fundraiserState: adminPda,
        receivingWallet : ,
        chrtMint: mintKey,
        fundStarter : admin.publicKey,
        donatorWallet: ,
        donator : ,
        systemProgram : SystemProgram.programId,
        tokenProgram : TOKEN_PROGRAM_ID,
        rent : SYSVAR_RENT_PUBKEY,

      }
    })
  })
});
