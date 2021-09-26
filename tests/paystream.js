const anchor = require('@project-serum/anchor');
const { SystemProgram } = anchor.web3;
const assert = require('assert');
const {web3} = require("@project-serum/anchor");

async function fundAccountFromWallet(toPubkey, lamports) {

  console.log(`fundAccountFromWallet(${toPubkey}, ${lamports})`);

  const provider = anchor.Provider.local();
  const fromPubkey = provider.wallet.publicKey;
  const transaction = new anchor.web3.Transaction().add(
      anchor.web3.SystemProgram.transfer({
        fromPubkey,
        toPubkey,
        lamports
      }),
  );

  transaction.feePayer = await provider.wallet.publicKey;
  let blockHashObj = await provider.connection.getRecentBlockhash();
  transaction.recentBlockhash = await blockHashObj.blockhash;

  let signature = await provider.wallet.signTransaction(transaction);
  await anchor.web3.sendAndConfirmRawTransaction(provider.connection, signature.serialize());
}

// Get the user's account in the program
async function userPubKey() {

  console.log(`userPubKey()`);

  const provider = anchor.Provider.local();
  const program = anchor.workspace.Paystream;
  const authority = provider.wallet.publicKey;

  return (
      await anchor.web3.PublicKey.findProgramAddress(
          [authority.toBuffer()],
          program.programId
      )
  )[0];
}

describe('stream payments', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  it('Register user', async () => {

    const program = anchor.workspace.Paystream;
    const provider = anchor.Provider.local();
    const authority = provider.wallet.publicKey;
    const name = "Lord Byron";

    const [user, bump] = await anchor.web3.PublicKey.findProgramAddress(
        [authority.toBuffer()],
        program.programId
    );

    await program.rpc.register(bump, name, {
      accounts: {
        user,
        authority,
        systemProgram: SystemProgram.programId,
      },
    });

  });

  it('Create stream', async () => {

    const program = anchor.workspace.Paystream;
    const provider = anchor.Provider.local();

    // Get the user's account in the program
    const user = await userPubKey();

    // Create an account and fund
    const lamports = 10000000;
    const stream = anchor.web3.Keypair.generate();
    const time_in_seconds = 60;

    // The account to receive the stream
    const receiver = anchor.web3.Keypair.generate();

    // Fund account
    await fundAccountFromWallet(stream.publicKey, lamports);

    console.log(`calling create(
      ${lamports}, 
      ${time_in_seconds}, 
      ${user}, 
      ${stream.publicKey},
      ${provider.wallet.publicKey},
      ${receiver.publicKey},
      ${SystemProgram.programId})`);

    // Create the stream
    await program.rpc.create(new anchor.BN(lamports), new anchor.BN(time_in_seconds), {
      accounts: {
        user,
        stream: stream.publicKey,
        authority: provider.wallet.publicKey,
        receiver: receiver.publicKey,
        systemProgram: SystemProgram.programId,
      },
      signers: [stream],
    });

    const current_user = await program.account.user.fetch(user);
    const new_stream = await program.account.stream.fetch(stream.publicKey);

    // Check that we have added this stream to our account
    assert.ok(current_user.streams[0].toBase58() === stream.publicKey.toBase58());
    // Check some basic values on the account creation
    assert.ok(new anchor.BN(lamports).eq(new_stream.amountInLamports));
    assert.ok(new anchor.BN(lamports).eq(new_stream.remainingLamports));
    assert.ok(new anchor.BN(time_in_seconds).eq(new_stream.timeInSeconds));
  });

  it('Withdraw', async () => {
    const program = anchor.workspace.Paystream;
    const provider = anchor.Provider.local();
    const authority = provider.wallet.publicKey;
    const lamports = 100;
    const pubKey = await userPubKey();
    // Get a stream
    const user = await program.account.user.fetch(pubKey);
    const stream = await program.account.stream.fetch(user.streams[0]);

    console.log("user:", user);
    console.log("stream:", stream);

    // await program.rpc.withdraw(new anchor.BN(lamports), {
    //   accounts: {
    //     payer: user.authority,
    //     stream,
    //     receiver: stream.receiver.toBase58(),
    //   },
    //   signers: [],
    // });
  });
});
