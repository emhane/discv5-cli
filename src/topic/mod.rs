use clap::ArgMatches;
use discv5::{advertisement::topic::{TopicHash, Sha256Topic}, enr, enr::CombinedKey, Discv5, Discv5ConfigBuilder, };
use log::{error, info, warn};
use std::net::{IpAddr, SocketAddr};
use futures::future::join_all;

pub async fn hashes(matches: &ArgMatches<'_>) {
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

    // Request the hashes
    info!("Requesting hashes of: {}", topic_string);

    let hashes = discv5.hashes(topic_string.to_owned());
    print_hashes(hashes);
}

// Print each hash and the hashing algortihm it derives from
fn print_hashes(hashes: Vec<(TopicHash, String)>) {
    info!("Topic hashes:");
    hashes
        .into_iter()
        .for_each(|(hash, hash_func)| info!("{} {}", hash, hash_func));
}

pub async fn topic_query(matches: &ArgMatches<'_>) {
    // Obtain the topic hash as base64 encoded string
    let topic_hash = matches
        .value_of("topic-hash")
        .expect("A <topic-hash> must be supplied");
    let hash_bytes = base64::decode(topic_hash).unwrap();
    let mut buf = [0u8;32];
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
        .udp(listen_port)
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
            connect_enr.ip(),
            connect_enr.udp(),
            connect_enr.tcp()
        );
        if let Err(e) = discv5.add_enr(connect_enr) {
            warn!("ENR not added: {:?}", e);
        }
    }

    for _ in 0..3 {
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

    }

    // Request the closest nodes to the topic hash
    info!("Requesting closest nodes to: {}", topic_hash);

    let enrs = match discv5.find_closest_nodes_to_topic(topic_hash).await {
        Ok(enrs) => enrs,
        Err(e) => {
            error!("Failed to obtain ENRs of closest nodes. Error: {}", e);
            Vec::new()
        },
    };

    let topic_query_futs = enrs.into_iter().map(|enr| discv5.topic_query_req(enr, topic_hash)).collect::<Vec<_>>();
    let fut = join_all(topic_query_futs);
    let res = fut.await;
    res.into_iter().for_each(|r| match r {
        Ok(enrs) => info!("Ads: {:?}", enrs),
        Err(e) => error!("Failed to obtain ads. Error: {}", e),
    })
}

pub async fn reg_topic(matches: &ArgMatches<'_>) {
    // Obtain the topic string
    let topic_string = matches
        .value_of("topic")
        .expect("A <topic> must be supplied");

    let topic = Sha256Topic::new(topic_string);

    // Set up a server to receive the response
    let listen_address = "127.0.0.1"
        .parse::<IpAddr>()
        .expect("This is a valid address");
    let listen_port = 9001;
    let enr_key = CombinedKey::generate_secp256k1();
    // Build a local ENR
    let enr = enr::EnrBuilder::new("v4")
        .ip(listen_address)
        .udp(listen_port)
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
            connect_enr.ip(),
            connect_enr.udp(),
            connect_enr.tcp()
        );
        if let Err(e) = discv5.add_enr(connect_enr) {
            warn!("ENR not added: {:?}", e);
        }
    }

    for _ in 0..3 {
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

    }

    // Request the closest nodes to the topic hash
    info!("Requesting closest nodes to: {}", topic.hash());

    let enrs = match discv5.find_closest_nodes_to_topic(topic.hash()).await {
        Ok(enrs) => enrs,
        Err(e) => {
            error!("Failed to obtain ENRs of closest nodes. Error: {}", e);
            Vec::new()
        },
    };

    let reg_topic_futs = enrs.into_iter().map(|enr| discv5.reg_topic_req(enr, topic.clone())).collect::<Vec<_>>();
    let fut = join_all(reg_topic_futs);
    let res = fut.await;
    res.into_iter().for_each(|r| match r {
        Ok(()) => info!("registered"),
        Err(e) => error!("Failed to register. Error: {}", e),
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    for _ in 0..3 {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        info!("Requesting active topics");

        match discv5.active_topics().await {
            Ok(ads) => info!("{:?}", ads),
            Err(e) => error!("Failed to obtain ads published on other nodes. Error: {}", e),
        };
    }


}
