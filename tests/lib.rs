use paystream::entrypoint::process_instruction;
use borsh::BorshDeserialize;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Signer,
    transaction::Transaction,
};

use solana_sdk::signature::Keypair;
use solana_sdk::transport::TransportError;
use paystream::instruction::{create, PaystreamInstruction, withdrawal};
use paystream::state::StreamAccount;

pub async fn sign_send_instruction(
    ctx: &mut ProgramTestContext,
    instruction: Instruction,
    signers: Vec<&Keypair>,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&ctx.payer.pubkey()));
    let mut payer_signers = vec![&ctx.payer];
    for s in signers {
        payer_signers.push(s);
    }
    transaction.partial_sign(&payer_signers, ctx.last_blockhash);
    ctx.banks_client.process_transaction(transaction).await
}

#[tokio::test]
async fn test_paystream() {
    let program_id = Pubkey::new_unique();

    let mut program_test = ProgramTest::new(
        "paystream", // Run the BPF version with `cargo test-bpf`
        program_id,
        processor!(process_instruction), // Run the native version with `cargo test`
    );

    // Payer keypair
    let payer_key = Keypair::new();
    // Payee keypair
    let payee_key = Keypair::new();
    // Stream keypair
    let stream_key = Keypair::new();

    let lamports = 10_000;
    let rent_exemption = 10_000_000;
    //TODO calculate lamports for rent for stream account

    // Add stream account
    program_test.add_account(
        stream_key.pubkey(),
        Account {
            lamports: lamports + rent_exemption,
            data: vec![0_u8; 81],
            owner: program_id,
            ..Account::default()
        },
    );

    // Add payee account
    program_test.add_account(
        payee_key.pubkey(),
        Account {
            lamports: 50000000,
            ..Account::default()
        },
    );

    // Add payee account
    program_test.add_account(
        payer_key.pubkey(),
        Account {
            lamports: 50000000,
            ..Account::default()
        },
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // Create stream instruction
    let create_stream_instruction = create(
        program_id,
        PaystreamInstruction::Create {
            payee_pubkey: payee_key.pubkey(),
            payer_pubkey: payer_key.pubkey(),
            amount: lamports,
            duration_in_seconds: 60,
        },
        stream_key.pubkey(),
    ).unwrap();

    let mut transaction = Transaction::new_with_payer(
            &[create_stream_instruction],
                      Some(&payer.pubkey()));

    transaction.sign(&[&payer, &stream_key], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    let stream_data = banks_client
        .get_account(stream_key.pubkey()).await.unwrap().unwrap();

    let stream = StreamAccount::try_from_slice(stream_data.data.as_slice()).unwrap();
    assert_eq!(stream.amount, lamports);


    // Withdrawal
    // The payee should be able to withdrawal the amount
    // Withdrawal stream instruction
    let withdrawal_stream_instruction = withdrawal(
        program_id,
        PaystreamInstruction::Withdrawal {
            amount: lamports,
        },
        stream_key.pubkey(),
        payee_key.pubkey(),
    ).unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[withdrawal_stream_instruction],
        Some(&payee_key.pubkey()));

    transaction.sign(&[&payee_key, &stream_key, &payee_key], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();
}