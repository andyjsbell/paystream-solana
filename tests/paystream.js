const anchor = require('@project-serum/anchor');
const { SystemProgram } = anchor.web3;
const assert = require('assert');
const {web3} = require("@project-serum/anchor");

describe('paystream', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  it('Is initialized!', async () => {

    const program = anchor.workspace.Paystream;
    const provider = anchor.Provider.local();

    // Create an account and fund
    const lamports = new anchor.BN(10000000);
    const stream = anchor.web3.Keypair.generate();
    const time_in_seconds = new anchor.BN(60);

    const receiver = anchor.web3.Keypair.generate();

    const tx = await program.rpc.create(lamports, time_in_seconds, {
      accounts: {
        stream: stream.publicKey,
        payer: provider.wallet.publicKey,
        receiver: receiver.publicKey,
        systemProgram: SystemProgram.programId,
      },
      signers: [stream],
    });

    const transaction = new anchor.web3.Transaction().add(
        anchor.web3.SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: stream.publicKey,
          lamports: 10000000
        }),
    );

    // Setting the variables for the transaction
    transaction.feePayer = await provider.wallet.publicKey;
    let blockhashObj = await provider.connection.getRecentBlockhash();
    transaction.recentBlockhash = await blockhashObj.blockhash;

    let signature = await provider.wallet.signTransaction(transaction);
    await anchor.web3.sendAndConfirmRawTransaction(provider.connection, signature.serialize());

    const accountStream = await provider.connection.getAccountInfo(stream.publicKey);
    console.log(accountStream);

    const new_stream = await program.account.stream.fetch(stream.publicKey);
    const accountBalance = await provider.connection.getBalance(SystemProgram.programId);
    console.log("balance of program=", accountBalance);
    console.log("account owner of stream=", accountStream.owner.toBase58());

    // Check some basic values on the account creation
    assert.ok(lamports.eq(new_stream.amountInLamports));
    assert.ok(lamports.eq(new_stream.remainingLamports));
    assert.ok(time_in_seconds.eq(new_stream.timeInSeconds));


    // await program.rpc.fund(lamports, {
    //   accounts: {
    //     stream: stream.publicKey,
    //     payer: provider.wallet.publicKey,
    //     receiver: receiver.publicKey,
    //   },
    //   signers: [stream],
    // });
    //
    // console.log(new_stream);
  });
});
