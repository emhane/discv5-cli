use clap::ArgMatches;
use discv5::{advertisement::topic::TopicHash, enr, enr::CombinedKey, Discv5, Discv5ConfigBuilder};
use log::{error, info, warn};
use std::net::{IpAddr, SocketAddr};

pub async fn hashes(matches: &ArgMatches<'_>) {
    // Obtain the topic string
    let topic_string = matches
        .value_of("topic")
        .expect("A <topic> must be supplied");

    // Request the hashes
    info!("Fetching hashes of: {}", topic_string);

    let hashes = Discv5::hashes(topic_string.to_owned());
    print_hashes(hashes);
}

// Print each hash and the hashing algortihm it derives from
fn print_hashes(hashes: Vec<(TopicHash, String)>) {
    info!("Topic hashes:");
    hashes
        .into_iter()
        .for_each(|(hash, hash_func)| info!("{} {}", hash, hash_func));
}

/// Remove topic from set of topics to republish, effective from next republish interval on
pub async fn remove_topic(matches: &ArgMatches<'_>) {
    // Obtain the topic string
    let topic = matches
        .value_of("topic")
        .expect("A <topic> must be supplied")
        .to_owned();

    let hash_bytes = base64::decode(topic.clone()).unwrap();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&hash_bytes);
    let topic_hash = TopicHash::from_raw(buf);

    // set up a server to receive the response
    let listen_address = "0.0.0.0"
        .parse::<IpAddr>()
        .expect("This is a valid address");
    let listen_port = 9011;
    let enr_key = CombinedKey::generate_secp256k1();

    // build a local ENR
    let enr = enr::EnrBuilder::new("v4")
        .ip(listen_address)
        .udp4(listen_port)
        .build(&enr_key)
        .unwrap();

    let listen_socket = SocketAddr::new(listen_address, listen_port);
    // default discv5 configuration
    let config = Discv5ConfigBuilder::new().build();
    // construct the discv5 service
    let mut discv5 = Discv5::new(enr, enr_key, config).unwrap();

    // start the server
    discv5.start(listen_socket).await.unwrap();

    // Request the hashes
    info!("Removing topic: {}", topic);

    match discv5.remove_topic(topic_hash).await {
        Ok(topic_string) => info!("Removed topic: {}", topic_string),
        Err(e) => error!("Failed to remove topic. Error: {}", e),
    }
}

pub async fn topic_query(matches: &ArgMatches<'_>) {
    // Obtain the topic hash as base64 encoded string
    let topic_hash = matches
        .value_of("topic-hash")
        .expect("A <topic-hash> must be supplied");
    let hash_bytes = base64::decode(topic_hash).unwrap();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&hash_bytes);
    let topic_hash = TopicHash::from_raw(buf);

    // Set up a server to receive the response
    let listen_address = "127.0.0.1"
        .parse::<IpAddr>()
        .expect("This is a valid address");
    let listen_port = 9006;
    let enr_key = CombinedKey::generate_secp256k1();
    // Build a local ENR
    let enr = enr::EnrBuilder::new("v4")
        .ip(listen_address)
        .udp4(listen_port)
        .build(&enr_key)
        .unwrap();
    let listen_socket = SocketAddr::new(listen_address, listen_port);
    // Default discv5 configuration
    let config = Discv5ConfigBuilder::new().build();
    // Construct the discv5 service
    let mut discv5 = Discv5::new(enr, enr_key, config).unwrap();
    // Start the server
    discv5.start(listen_socket).await.unwrap();

    let connect_enr = matches.value_of("enr").map(|enr| {
        enr.parse::<enr::Enr<enr::CombinedKey>>()
            .expect("Invalid base64 encoded ENR")
    });

    if let Some(connect_enr) = connect_enr {
        info!(
            "Connecting to ENR. ip: {:?}, udp_port: {:?},  tcp_port: {:?}",
            connect_enr.ip4(),
            connect_enr.udp4(),
            connect_enr.tcp4()
        );
        if let Err(e) = discv5.add_enr(connect_enr) {
            warn!("ENR not added: {:?}", e);
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    info!("Searching for peers...");
    // pick a random node target
    let target_random_node_id = enr::NodeId::random();
    match discv5.find_node(target_random_node_id).await {
        Err(e) => println!("Find Node result failed: {:?}", e),
        Ok(found_enrs) => {
            info!("Query Completed. Nodes found: {}", found_enrs.len());
            for enr in found_enrs {
                info!("Node: {}", enr.node_id());
            }
        }
    }
    info!("Connected Peers: {}", discv5.connected_peers());

    info!("Sending TOPICQUERYs");
    discv5
        .topic_query_req(topic_hash)
        .await
        .map_err(|e| error!("Failed to register. Error: {}", e))
        .map(|enrs| {
            info!("Ads found for {}:", topic_hash);
            enrs.into_iter()
                .for_each(|enr| info!("NodeId: {}", enr.node_id()));
        })
        .ok();
}

pub async fn reg_topic(matches: &ArgMatches<'_>) {
    // Obtain the topic string
    let topic = matches
        .value_of("topic")
        .expect("A <topic> must be supplied")
        .to_owned();

    // Set up a server to receive the response
    let listen_address = "127.0.0.1"
        .parse::<IpAddr>()
        .expect("This is a valid address");
    let listen_port = 9017;
    let enr_key = CombinedKey::generate_secp256k1();
    // Build a local ENR
    let enr = enr::EnrBuilder::new("v4")
        .ip(listen_address)
        .udp4(listen_port)
        .build(&enr_key)
        .unwrap();
    let listen_socket = SocketAddr::new(listen_address, listen_port);
    // Default discv5 configuration
    let config = Discv5ConfigBuilder::new().build();
    // Construct the discv5 service
    let mut discv5 = Discv5::new(enr, enr_key, config).unwrap();
    // Start the server
    discv5.start(listen_socket).await.unwrap();

    let connect_enr = matches.value_of("enr").map(|enr| {
        enr.parse::<enr::Enr<enr::CombinedKey>>()
            .expect("Invalid base64 encoded ENR")
    });

    if let Some(connect_enr) = connect_enr {
        info!(
            "Connecting to ENR. ip: {:?}, udp_port: {:?},  tcp_port: {:?}",
            connect_enr.ip4(),
            connect_enr.udp4(),
            connect_enr.tcp4()
        );
        if let Err(e) = discv5.add_enr(connect_enr) {
            warn!("ENR not added: {:?}", e);
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    info!("Searching for peers...");
    // pick a random node target
    let target_random_node_id = enr::NodeId::random();
    match discv5.find_node(target_random_node_id).await {
        Err(e) => println!("Find Node result failed: {:?}", e),
        Ok(found_enrs) => {
            info!("Query Completed. Nodes found: {}", found_enrs.len());
            for enr in found_enrs {
                info!("Node: {}", enr.node_id());
            }
        }
    }
    info!("Connected Peers: {}", discv5.connected_peers());

    info!("Sending REGTOPIC requests");
    discv5
        .reg_topic_req(topic)
        .await
        .map_err(|e| error!("Failed to register. Error: {}", e))
        .ok();

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
        info!("Requesting active topics");

        match discv5.active_topics().await {
            Ok(ads) => info!("Ads published by us active on other nodes: {}", ads),
            Err(e) => error!(
                "Failed to obtain ads published on other nodes. Error: {}",
                e
            ),
        };
    }
}
