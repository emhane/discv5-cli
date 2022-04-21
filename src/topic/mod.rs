use clap::ArgMatches;
use discv5::{enr, enr::CombinedKey, Discv5, Discv5ConfigBuilder, advertisement::TopicHash};
use log::info;
use std::net::{IpAddr, SocketAddr};

pub async fn run(matches: &ArgMatches<'_>) {
    // Obtain the topic string
    let topic_string = matches
        .value_of("topic")
        .expect("A <topic> must be supplied");

    // set up a server to receive the response
    let listen_address = "0.0.0.0"
        .parse::<IpAddr>()
        .expect("This is a valid address");
    let listen_port = 9001;
    let enr_key = CombinedKey::generate_secp256k1();

    // build a local ENR
    let enr = enr::EnrBuilder::new("v4")
        .ip(listen_address)
        .udp(listen_port)
        .build(&enr_key)
        .unwrap();

    let listen_socket = SocketAddr::new(listen_address, listen_port);
    // default discv5 configuration
    let config = Discv5ConfigBuilder::new().build();
    // construct the discv5 service
    let mut discv5 = Discv5::new(enr, enr_key, config).unwrap();

    // start the server
    discv5.start(listen_socket).await.unwrap();

    // Request the ENR
    info!("Requesting hashes of: {}", topic_string);

    let hashes = discv5.hashes(topic_string.to_owned());
    print_hashes(hashes);
}

// Print various information about the obtained ENR.
fn print_hashes(hashes: Vec<(TopicHash, String)>) {
    info!("Topic hashes:");
    hashes.into_iter().for_each(|(hash, hash_func)| info!("{} {}", hash, hash_func));
}
