use clap::{App, Arg};
use config::{Config as C, ConfigError, File};
use serde::Deserialize;
pub struct Config {
    pub from_block: u32,
    pub to_block: u32,
    pub starknet_rpc_url: String,
    pub eth_rpc_url: String,
    pub l2_sender: String,
    pub starkway_l1: String,
    pub starkway_l2: String,
    pub starknet_core: String,
}

#[derive(Debug, Deserialize)]
pub struct NetworkConfig {
    pub starknet_rpc_url: String,
    pub eth_rpc_url: String,
    pub starkway_l1: String,
    pub starkway_l2: String,
    pub starknet_core: String,
}

fn load_config(network: String) -> Result<NetworkConfig, ConfigError> {
    let settings = C::builder()
        .add_source(File::with_name((network + ".yaml").as_str()))
        .build()?;
    let network_config: NetworkConfig = settings.try_deserialize()?;
    Ok(network_config)
}

pub fn get_config() -> Result<Config, Box<dyn std::error::Error>> {
    let matches = App::new("Event Tracker")
        .version("0.1.0")
        .author("Uday <uday@zkx.fi>")
        .about("Track Starkway Events")
        .arg(
            Arg::with_name("from_block")
                .short("from")
                .long("from-block")
                .help("Get events from this block number")
                .takes_value(true)
                .validator(|val| match val.parse::<u32>().is_ok() {
                    true => Ok(()),
                    _ => Err(String::from("unexpected value of From-block number")),
                })
                .required(true),
        )
        .arg(
            Arg::with_name("to_block")
                .short("to")
                .long("to-block")
                .help("Get events till this block number")
                .takes_value(true)
                .validator(|val| match val.parse::<u32>().is_ok() {
                    true => Ok(()),
                    _ => Err(String::from("unexpected value of To-block number")),
                })
                .required(true),
        )
        .arg(
            Arg::with_name("network")
                .short("n")
                .long("network")
                .help("Retrieve events for this network sepolia/mainnet")
                .takes_value(true)
                .default_value("mainnet")
                .required(false),
        )
        .arg(
            Arg::with_name("l2_sender")
                .short("s")
                .long("sender")
                .help("THIS OPTION IS UNUSED FOR NOW")
                .takes_value(true)
                .required(false),
        )
        .get_matches();

    let from_block = matches
        .value_of("from_block")
        .map(|val| val.parse::<u32>().unwrap())
        .unwrap();
    let to_block = matches
        .value_of("to_block")
        .map(|val| val.parse::<u32>().unwrap())
        .unwrap();
    let network = matches.value_of("network").unwrap();
    let network_config = load_config(network.to_string())?;
    let l2_sender = if matches.is_present("l2_sender") {
        matches.value_of("l2_sender").unwrap()
    } else {
        ""
    };

    let formatted_l2_sender = if matches.is_present("l2_sender") {
        matches.value_of("l2_sender").unwrap()
    } else {
        "NO L2 SENDER SPECIFIED"
    };

    println!("From Block:{}", from_block);
    println!("To Block:{}", to_block);
    println!("Network:{}", network);
    println!(
        "Starknet RPC URL:{}",
        network_config.starknet_rpc_url.clone()
    );
    println!("Ethereum RPC URL:{}", network_config.eth_rpc_url.clone());
    println!("Starkway L1 Address:{}", network_config.starkway_l1.clone());
    println!("Starkway L2 Address:{}", network_config.starkway_l2.clone());
    println!(
        "Starknet Core Address:{}",
        network_config.starknet_core.clone()
    );
    println!("L2 Sender:{}", formatted_l2_sender);
    let l2_sender = "";
    Ok(Config {
        from_block,
        to_block,
        starknet_rpc_url: network_config.starknet_rpc_url.clone(),
        eth_rpc_url: network_config.eth_rpc_url.clone(),
        l2_sender: l2_sender.to_string(),
        starkway_l1: network_config.starkway_l1.clone(),
        starkway_l2: network_config.starkway_l2.clone(),
        starknet_core: network_config.starknet_core.clone(),
    })
}
