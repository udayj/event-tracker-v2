use alloy::providers::ProviderBuilder;

use alloy::{
    primitives::{keccak256, U256},
    sol,
};
use chrono::{TimeZone, Utc};
use csv::Writer;
use event_tracker::config::get_config;
use num_bigint::BigUint;
use starknet::{
    core::types::{BlockId, EventFilter, Felt, MaybePendingBlockWithTxHashes},
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, Url,
    },
};
use std::{fs::File, str::FromStr};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get all network config, addresses alongwith block numbers for filtering events
    let config = get_config()?;

    let file = File::create("output.csv")?;
    let mut wrt = Writer::from_writer(file);
    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(&config.starknet_rpc_url)?));

    // This filter will be unused for now since l2_sender in config is currently always empty
    let l2_sender_filter = if config.l2_sender.is_empty() {
        vec![]
    } else {
        vec![Felt::from_hex(&config.l2_sender).unwrap()]
    };

    let keys = vec![
        vec![],
        l2_sender_filter,
        vec![],
        vec![Felt::from_hex("0x5749544844524157").unwrap()], // This hex value corresponds to 'WITHDRAW' in felt
        vec![],
        vec![],
    ];

    // Output file header
    wrt.write_record([
        "Tx Hash",
        "Timestamp",
        "From L2 Addr",
        "To L1 Recipient",
        "L1 Token",
        "Amount",
        "Is completed on L1 (zero = yes)",
        "Message Hash",
    ])?;

    let mut cont: Option<String> = Some("initial".to_string());

    let mut index = 1;
    while cont.is_some() {
        // We can do this unwrap here since cont will not be None inside the loop
        if cont.clone().unwrap() == "initial" {
            cont = None;
        }

        // Get events in chunks of 50 events
        let result = provider
            .get_events(
                EventFilter {
                    from_block: Some(BlockId::Number(config.from_block as u64)),
                    to_block: Some(BlockId::Number(config.to_block as u64)),
                    address: Some(Felt::from_hex(&config.starkway_l2)?),
                    keys: Some(keys.clone()),
                },
                cont.clone(),
                50,
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
                    let amount_low = event.data[0];
                    let block_number = event.block_number.unwrap();
                    let amount_low = amount_low.to_biguint();
                    let amount_high = event.data[1].to_biguint();
                    let total_value: BigUint = amount_high << 256 | amount_low;
                    let total_value_felt = Felt::from_bytes_be_slice(&total_value.to_bytes_be());

                    // Event data does not have timestamp nor does the transaction data
                    // Get block number and retrieve timestamp
                    let result_timestamp = provider
                        .get_block_with_tx_hashes(BlockId::Number(block_number))
                        .await;
                    let formatted: String;

                    match result_timestamp {
                        Ok(pending_block_data) => {
                            match pending_block_data {
                                // If we find a non-pending block then proceed
                                MaybePendingBlockWithTxHashes::Block(block_data) => {
                                    let timestamp = block_data.timestamp;
                                    // Convert timestamp to DateTime<Utc>
                                    let datetime =
                                        Utc.timestamp_opt(timestamp.try_into()?, 0).unwrap();

                                    // Format the datetime to a human-readable string
                                    formatted =
                                        datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string();
                                }
                                _ => {
                                    formatted = "BLOCK STILL PENDING".to_string();
                                }
                            }
                        }
                        Err(_) => {
                            formatted = "BLOCK NOT FOUND".to_string();
                        }
                    }
                    // Form message hash
                    // Message hash is formed using following solidity code
                    // bytes32 msgHash = keccak256(
                    // abi.encodePacked(starkwayL2, uint256(uint160(starkwayL1)), payload.length, payload)
                    // );

                    let starkway_l2 = U256::from_str(&config.starkway_l2)?;

                    let starkway_l1 = U256::from_str(&config.starkway_l1)?;

                    // This is the payload sent as a message from L2 to L1 by starkway
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

                    let rpc_url = config.eth_rpc_url.parse()?;

                    let provider = ProviderBuilder::new().on_http(rpc_url);

                    sol!(
                        #[sol(rpc)]
                        interface Starknet {
                            function l2ToL1Messages(bytes32 msgHash) external view returns (uint256);
                        }
                    );

                    let contract =
                        Starknet::new(config.starknet_core.as_str().parse().unwrap(), provider);
                    let num_messages = contract.l2ToL1Messages(hash);
                    // Number of messages > 0 implies that there is an unconsumed message and hence
                    // incomplete withdrawal on L1
                    // However, a value of 0 can either mean that message has been consumed or that no message was available
                    let Starknet::l2ToL1MessagesReturn { _0 } = num_messages.call().await.unwrap();
                    let formatted_num_messages = format!("{_0}");
                    wrt.write_record(&[
                        tx_hash.clone(),
                        formatted,
                        l2_sender.clone(),
                        l1_recipient.clone(),
                        l1_token_address.clone(),
                        total_value_felt.to_string(),
                        formatted_num_messages,
                        hash.to_string(),
                    ])?;

                    println!(
                        "Found event data for transaction {} with hash:{}",
                        index, tx_hash
                    );
                    index += 1;
                }
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }
        wrt.flush()?;
    }

    Ok(())
}
