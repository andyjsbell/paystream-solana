use paystream::entrypoint::process_instruction;
use borsh::BorshDeserialize;
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Signer,
    transaction::Transaction,
};

use solana_program::hash::Hash;
use solana_sdk::signature::Keypair;
use solana_sdk::transport::TransportError;
use paystream::instruction::{create, cancel, PaystreamInstruction, withdrawal};
use paystream::state::{StreamAccount, StreamStatus};

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

fn add_stream_account(program_id: Pubkey, program_test: &mut ProgramTest, stream_key: &Keypair, amount: u64) {
    //TODO calculate lamports for rent for stream account
    let rent_exemption = 10_000_000;
    // Add stream account
    program_test.add_account(
        stream_key.pubkey(),
        Account {
            lamports: amount + rent_exemption,
            data: vec![0_u8; 97],
            owner: program_id,
            ..Account::default()
        },
    );
}

async fn get_stream_account(banks_client: &mut BanksClient, stream_key: &Keypair) -> StreamAccount {
    let stream_data = banks_client
        .get_account(stream_key.pubkey()).await.unwrap().unwrap();

    StreamAccount::try_from_slice(stream_data.data.as_slice()).unwrap()
}

fn create_program_test() -> (Pubkey, ProgramTest, Keypair, Keypair) {
    let program_id = Pubkey::new_unique();
    // Payer keypair
    let payer_key = Keypair::new();
    // Payee keypair
    let payee_key = Keypair::new();

    let mut program_test = ProgramTest::new(
        "paystream", // Run the BPF version with `cargo test-bpf`
        program_id,
        processor!(process_instruction), // Run the native version with `cargo test`
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

    (program_id, program_test, payer_key, payee_key)
}

fn create_stream_transaction(program_id: Pubkey, 
    payee_key: &Keypair, 
    payer_key: &Keypair, 
    stream_key: &Keypair, 
    amount: u64, 
    payer: &Keypair, 
    recent_blockhash: Hash
) -> Transaction {
    // Create stream instruction
    let create_stream_instruction = create(
        program_id,
        PaystreamInstruction::Create {
            payee_pubkey: payee_key.pubkey(),
            payer_pubkey: payer_key.pubkey(),
            amount,
            duration_in_seconds: 60,
        },
        stream_key.pubkey(),
    ).unwrap();

    let mut transaction = Transaction::new_with_payer(
            &[create_stream_instruction],
                      Some(&payer.pubkey()));

    transaction.sign(&[payer, stream_key], recent_blockhash);

    transaction
}

fn withdrawal_stream_transaction(program_id: Pubkey, 
    stream_key: &Keypair, 
    payee_key: &Keypair, 
    payer: &Keypair,
    amount: u64,
    recent_blockhash: Hash
) -> Transaction {
    // Withdrawal stream instruction
    let withdrawal_stream_instruction = withdrawal(
        program_id,
        PaystreamInstruction::Withdrawal {
            amount
        },
        stream_key.pubkey(),
        payee_key.pubkey(),
    ).unwrap();

    let mut transaction = Transaction::new_with_payer(
            &[withdrawal_stream_instruction],
                    Some(&payer.pubkey()));

    transaction.sign(&[stream_key, payee_key, payer], recent_blockhash);

    transaction
}

fn cancel_stream_transaction(program_id: Pubkey, 
    stream_key: &Keypair, 
    payee_key: &Keypair, 
    payer_key: &Keypair,
    payer: &Keypair,
    recent_blockhash: Hash
) -> Transaction {
    // Cancel stream instruction
    let cancel_stream_instruction = cancel(
        program_id,
        PaystreamInstruction::Cancel {},
        stream_key.pubkey(),
        payee_key.pubkey(),
        payer_key.pubkey(),
    ).unwrap();

    let mut transaction = Transaction::new_with_payer(
            &[cancel_stream_instruction],
                    Some(&payer.pubkey()));

    transaction.sign(&[stream_key, payee_key, payer], recent_blockhash);

    transaction
}

#[tokio::test]
async fn should_cancel_stream() {
    let (program_id, mut program_test, payer_key, payee_key) = create_program_test();
    let stream_key = Keypair::new();
    let amount = 1000;
    add_stream_account(program_id, &mut program_test, &stream_key, amount);
   
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let transaction = create_stream_transaction(program_id, 
        &payee_key, 
        &payer_key, 
        &stream_key, 
        amount, 
        &payer, 
        recent_blockhash
    );
    
    banks_client.process_transaction(transaction).await.unwrap();

    let transaction = cancel_stream_transaction(program_id, 
        &stream_key,
        &payee_key, 
        &payer_key, 
        &payer, 
        recent_blockhash
    );

    banks_client.process_transaction(transaction).await.unwrap();

    let stream = get_stream_account(&mut banks_client, &stream_key).await;
    assert_eq!(stream.status, StreamStatus::Terminated as u8);
}

#[tokio::test]
async fn should_create_stream() {
    let (program_id, mut program_test, payer_key, payee_key) = create_program_test();
    
    let stream_key = Keypair::new();
    let amount = 1000;
    add_stream_account(program_id, &mut program_test, &stream_key, amount);
    
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let transaction = create_stream_transaction(program_id, 
        &payee_key, 
        &payer_key, 
        &stream_key, 
        amount, 
        &payer, 
        recent_blockhash
    );
    
    banks_client.process_transaction(transaction).await.unwrap();

    let stream = get_stream_account(&mut banks_client, &stream_key).await;
    assert_eq!(stream.remaining_lamports, amount);
}

#[tokio::test]
async fn should_withdrawal_from_stream() {
    // Create program test
    let (program_id, mut program_test, payer_key, payee_key) = create_program_test();

    let stream_key = Keypair::new();
    let amount = 1000;
    add_stream_account(program_id, &mut program_test, &stream_key, amount);
    
    let mut ctx = program_test.start_with_context().await;

    // let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    let transaction = create_stream_transaction(program_id, 
        &payee_key, 
        &payer_key, 
        &stream_key, 
        amount, 
        &ctx.payer, 
        ctx.last_blockhash
    );
    
    ctx.banks_client.process_transaction(transaction).await.unwrap();

    let slots_to_warp = 10u64;
    ctx.warp_to_slot(slots_to_warp).unwrap();

    let transaction = withdrawal_stream_transaction(
        program_id, 
        &stream_key, 
        &payee_key, 
        &ctx.payer, 
        amount / 2, 
        ctx.last_blockhash
    );

    ctx.banks_client.process_transaction(transaction).await.unwrap();

    let stream = get_stream_account(&mut ctx.banks_client, &stream_key).await;
    assert_eq!(stream.remaining_lamports, amount / 2);
}
