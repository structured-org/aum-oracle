import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { OracleDataReceiver } from "../target/types/oracle_data_receiver";
import { assert } from "chai";
import { createHash } from "node:crypto";

describe("oracle_data_receiver", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.OracleDataReceiver as Program<OracleDataReceiver>;
  const admin = anchor.web3.Keypair.generate();
  const oracles = '123456'.split('').map(_ => anchor.web3.Keypair.generate());
  const oracle = oracles[0]; // Use the first oracle for tests
  const randomUser = anchor.web3.Keypair.generate();

  let configPda: anchor.web3.PublicKey;
  let oracleDataPda: anchor.web3.PublicKey;
  let consensusPda: anchor.web3.PublicKey;
  

  const instanceKey = anchor.web3.Keypair.generate();
  
  before(async () => {
    for (const user of [admin, oracle, randomUser]) {
      const sig = await provider.connection.requestAirdrop(user.publicKey, 1e9);
      await provider.connection.confirmTransaction(sig);
    }

    [configPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("config"), instanceKey.publicKey.toBuffer()],
      program.programId
    );

    [oracleDataPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("oracle_data"), instanceKey.publicKey.toBuffer(), oracle.publicKey.toBuffer()],
      program.programId
    );

    [consensusPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("consensus"), instanceKey.publicKey.toBuffer()],
      program.programId
    );
  });

  it("initializes state with admin = payer", async () => {
    const updateTime = 60;
    const expirationTime = 120;    

    await program.methods
      .initialize({updateTime, expirationTime, owner: admin.publicKey})
      .accounts({
        instanceKey: instanceKey.publicKey,
        signer: admin.publicKey,
      })
      .signers([admin, instanceKey])
      .rpc();

    const config = await program.account.config.fetch(configPda);
    assert.strictEqual(config.updateTime, updateTime);
    assert.strictEqual(config.expirationTime, expirationTime);
    assert.deepEqual(config.oracles, []);
  });

  it("adds oracle", async () => {
    await program.methods
      .addOracle(oracle.publicKey)
      .accounts({
        instanceKey: instanceKey.publicKey,
        signer: admin.publicKey,
      })
      .signers([admin])
      .rpc();

    const config = await program.account.config.fetch(configPda);
    assert.deepInclude(config.oracles.map(pk => pk.toBase58()), oracle.publicKey.toBase58());
  });

  it("publishes data from oracle", async () => {
    const data = Buffer.from("example oracle payload");
    const hash = createHash("sha256").update(data).digest("hex"); 

    const sig = await program.methods.publishData(Buffer.from(data))
      .accounts({
        oracle: oracle.publicKey,
        instanceKey: instanceKey.publicKey,
      })
      .signers([oracle])
      .rpc();

    const txDetails = await provider.connection.getTransaction(sig, {
      commitment: "confirmed",
      maxSupportedTransactionVersion: 0,
    });
    console.log("Logs:");
    console.log(txDetails?.meta?.logMessages?.join("\n"));

    const acc = await program.account.oracleData.fetch(oracleDataPda);
    assert.deepEqual(acc.oracle.toBase58(), oracle.publicKey.toBase58());
    assert.deepEqual(Buffer.from(acc.data), data);
    assert.deepEqual(Buffer.from(acc.hash).toString('hex'), hash);
    assert.isAbove(Number(acc.timestamp.toString()), 0);

    const consensus = await program.account.consensus.fetch(consensusPda);
    assert.deepEqual(consensus.data, Buffer.from(data));
  });

  it("fails if non-oracle tries to publish", async () => {
    const attacker = anchor.web3.Keypair.generate();
    const sig = await provider.connection.requestAirdrop(attacker.publicKey, 1e9);
    await provider.connection.confirmTransaction(sig);

    try {
      await program.methods.publishData(Buffer.from("bad data"))
        .accounts({
          instanceKey: instanceKey.publicKey,
          oracle: attacker.publicKey,
        })
        .signers([attacker])
        .rpc();
      assert.fail("Expected to fail for non-oracle");
    } catch (e) {
      assert.include(e.toString(), "Unauthorized");
    }
  });


  it("removes oracle", async () => {
    await program.methods
      .removeOracle(oracle.publicKey)
      .accounts({
        instanceKey: instanceKey.publicKey,
        signer: admin.publicKey,
      })
      .signers([admin])
      .rpc();

    const config = await program.account.config.fetch(configPda);
    assert.notDeepInclude(config.oracles.map(pk => pk.toBase58()), oracle.publicKey.toBase58());
  });

});