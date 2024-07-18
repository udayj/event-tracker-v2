use alloy::providers::{Provider as P, ProviderBuilder};
use alloy::rpc::client::{ClientBuilder, RpcClient};
use alloy::sol_types::SolCall;
use alloy::sol_types::SolValue;
use alloy::{
    primitives::{keccak256, Address, Bytes, U256},
    sol,
};
use chrono::{DateTime, TimeZone, Utc};
use csv::Writer;
use num_bigint::BigUint;
use starknet::{
    core::types::{BlockId, BlockTag, EventFilter, Felt, MaybePendingBlockWithTxHashes},
    macros::felt,
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};
use std::{fs::File, str::FromStr};

#[tokio::main]
async fn main() {
    let file = File::create("output.csv").unwrap();
    let mut wrt = Writer::from_writer(file);
    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse("https://rpc.nethermind.io/mainnet-juno?apikey=5n1kZyTyMGiYmPn5YtGxlwHYSFTDRGCTGTfzFIn8nGKMdyOa").unwrap(),
    ));

    let keys = vec![
        vec![],
        vec![],
        vec![],
        vec![Felt::from_hex("0x5749544844524157").unwrap()],
        vec![],
        vec![],
    ];
    wrt.write_record(&["Tx Hash", "L2 Addr", "L1 Recipient", "L1 Token", "Amount"]);
    let mut cont: Option<String> = Some("initial".to_string());
    while cont.is_some() {
        if cont.clone().unwrap() == "initial".to_string() {
            cont = None;
        }

        let result = provider
            .get_events(
                EventFilter {
                    from_block: Some(BlockId::Number(657447)),
                    to_block: Some(BlockId::Number(657486)), // 657586
                    address: Some(
                        Felt::from_hex(
                            "0x3adccae1d7b4c8832133c0d090b84d4bd85f53a260dee461d51ab8dd07c9ef8",
                        )
                        .unwrap(),
                    ),
                    keys: Some(keys.clone()),
                },
                cont.clone(),
                10,
            )
            .await;
        match result {
            Ok(events_page) => {
                let emitted_events = events_page.events;
                cont = events_page.continuation_token;

                for event in emitted_events {
                    let l1_recipient = event.keys[0].to_hex_string();
                    let l2_sender = event.keys[1].to_hex_string();
                    let l1_token_address = event.keys[4].to_hex_string();
                    let tx_hash = event.transaction_hash.to_hex_string();
                    let amount = event.data[0];
                    let block_number = event.block_number.unwrap();
                    let amount_low = amount.to_biguint();
                    let amount_high = event.data[1].to_biguint();
                    let total_value: BigUint = amount_high << 256 | amount_low;
                    let total_value_felt = Felt::from_bytes_be_slice(&total_value.to_bytes_be());

                    wrt.write_record(&[
                        tx_hash,
                        l2_sender.clone(),
                        l1_recipient.clone(),
                        l1_token_address.clone(),
                        total_value_felt.to_string(),
                    ]);

                    let result_timestamp = provider
                        .get_block_with_tx_hashes(BlockId::Number(block_number))
                        .await;
                    match result_timestamp {
                        Ok(pending_block_data) => {
                            match pending_block_data {
                                MaybePendingBlockWithTxHashes::Block(block_data) => {
                                    let timestamp = block_data.timestamp;
                                    // Convert timestamp to DateTime<Utc>
                                    let datetime = Utc
                                        .timestamp_opt(timestamp.try_into().unwrap(), 0)
                                        .unwrap();

                                    // Format the datetime to a human-readable string
                                    let formatted =
                                        datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string();

                                    println!("Timestamp: {}", timestamp);
                                    println!("Formatted UTC time: {}", formatted);
                                }
                                _ => {
                                    panic!("Block not yet available");
                                }
                            }
                        }
                        Err(_) => {
                            panic!("Error");
                        }
                    }
                    let starkway_l2 = U256::from_str(
                        "0x3adccae1d7b4c8832133c0d090b84d4bd85f53a260dee461d51ab8dd07c9ef8",
                    )
                    .unwrap();
                    let starkway_l1 =
                        U256::from_str("0xCAbb5DDff712598B3c8183B988f082ef3dc74E00").unwrap();
                    let payload = [
                        U256::from_str(&l1_token_address).unwrap(),
                        U256::from_str(&l1_recipient).unwrap(),
                        U256::from_str(&l2_sender).unwrap(),
                        U256::from_str(event.data[0].to_hex_string().as_str()).unwrap(),
                        U256::from_str(event.data[1].to_hex_string().as_str()).unwrap(),
                    ];
                    let mut packed = Vec::new();
                    packed.extend_from_slice(&starkway_l2.to_be_bytes::<32>());
                    packed.extend_from_slice(&starkway_l1.to_be_bytes::<32>());

                    packed.extend_from_slice(&U256::from_str("5").unwrap().to_be_bytes::<32>());
                    for item in &payload {
                        packed.extend_from_slice(&item.to_be_bytes::<32>());
                    }
                    let hash = keccak256(&packed);
                    println!("Message hash={}", hash);

                    let rpc_url = "https://mainnet.infura.io/v3/aebb09967b724dfd91107bea3daba50f"
                        .parse()
                        .unwrap();

                    // Create a provider with the HTTP transport using the `reqwest` crate.
                    let provider = ProviderBuilder::new().on_http(rpc_url);

                    sol!(
                        #[sol(rpc)]
                        interface Starknet {
                            function l2ToL1Messages(bytes32 msgHash) external view returns (uint256);
                        }
                    );

                    let contract = Starknet::new(
                        "0xc662c410C0ECf747543f5bA90660f6ABeBD9C8c4"
                            .parse()
                            .unwrap(),
                        provider,
                    );
                    let num_messages = contract.l2ToL1Messages(hash);
                    let Starknet::l2ToL1MessagesReturn { _0 } = num_messages.call().await.unwrap();
                    println!("number of messages:{_0}");
                    // form message hash
                    // connect to starknet core contract
                    // check whether message consumed
                    // get block details based on block number and get current timestamp
                    // convert timestamp to human readable format
                }
                //println!("{:#?}", events_page);
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }
        wrt.flush();
    }
}
