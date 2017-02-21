use bifrost::rpc::*;
use bifrost::raft::*;
use bifrost::raft::client::RaftClient;
use bifrost::raft::state_machine::callback::client::SubscriptionService;
use bifrost::membership::server::Membership;
use bifrost::membership::member::MemberService;
use bifrost::membership::client::Client;
use bifrost::dht::{DHT, DHTError};
use bifrost::dht::weights::Weights;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;

use raft::wait;

#[test]
fn primary() {
    let addr = String::from("127.0.0.1:2200");
    let raft_service = RaftService::new(Options {
        storage: Storage::Default(),
        address: addr.clone(),
        service_id: 0,
    });
    let server = Server::new(vec!((0, raft_service.clone())));
    let heartbeat_service = Membership::new(&server, &raft_service);
    Server::listen_and_resume(&server, &addr);
    RaftService::start(&raft_service);
    raft_service.bootstrap();

    let group_1 = String::from("test_group_1");
    let group_2 = String::from("test_group_2");
    let group_3 = String::from("test_group_3");

    let server_1 =  String::from("server1");
    let server_2 =  String::from("server2");
    let server_3 =  String::from("server3");

    let wild_raft_client = RaftClient::new(vec!(addr.clone()), 0).unwrap();
    let client = Client::new(&wild_raft_client);

    let subs_service = SubscriptionService::initialize(&server);
    wild_raft_client.set_subscription(&subs_service);

    client.new_group(&group_1).unwrap().unwrap();
    client.new_group(&group_2).unwrap().unwrap();
    client.new_group(&group_3).unwrap().unwrap();

    let member1_raft_client = RaftClient::new(vec!(addr.clone()), 0).unwrap();
    let member1_svr = MemberService::new(&server_1, &member1_raft_client);

    let member2_raft_client = RaftClient::new(vec!(addr.clone()), 0).unwrap();
    let member2_svr = MemberService::new(&server_2, &member2_raft_client);

    let member3_raft_client = RaftClient::new(vec!(addr.clone()), 0).unwrap();
    let member3_svr = MemberService::new(&server_3, &member3_raft_client);

    member1_svr.join_group(&group_1).unwrap().unwrap();
    member2_svr.join_group(&group_1).unwrap().unwrap();
    member3_svr.join_group(&group_1).unwrap().unwrap();

    member1_svr.join_group(&group_2).unwrap().unwrap();
    member2_svr.join_group(&group_2).unwrap().unwrap();

    member1_svr.join_group(&group_3).unwrap().unwrap();

    let weight_service = Weights::new(&raft_service);

    let dht1 = DHT::new(&group_1, &wild_raft_client).unwrap();
    let dht2 = DHT::new(&group_2, &wild_raft_client).unwrap();
    let dht3 = DHT::new(&group_3, &wild_raft_client).unwrap();

    dht1.set_weight(&server_1, 1);
    dht1.set_weight(&server_2, 2);
    dht1.set_weight(&server_3, 3);

    dht2.set_weight(&server_1, 1);
    dht2.set_weight(&server_2, 1);

    dht3.set_weight(&server_1, 1);

    dht1.init_table().unwrap();
    dht2.init_table().unwrap();
    dht3.init_table().unwrap();

    assert_eq!(dht1.nodes_count(), 2047);
    assert_eq!(dht2.nodes_count(), 2048);
    assert_eq!(dht3.nodes_count(), 2048);

    let mut dht_1_mapping: HashMap<String, u64> = HashMap::new();
    for i in 0..30000 {
        let k = format!("k - {}", i);
        let server = dht1.get_server_by_string(&k).unwrap();
        *dht_1_mapping.entry(server.clone()).or_insert(0) += 1;
    }
    for (k, v) in dht_1_mapping.iter() {
        println!("DHT 1: {} -> {}", k ,v);
    }
    assert_eq!(dht_1_mapping.get(&server_1).unwrap(), &4922);
    assert_eq!(dht_1_mapping.get(&server_2).unwrap(), &9963);
    assert_eq!(dht_1_mapping.get(&server_3).unwrap(), &15115); // hard coded due to constant

    let mut dht_2_mapping: HashMap<String, u64> = HashMap::new();
    for i in 0..30000 {
        let k = format!("k - {}", i);
        let server = dht2.get_server_by_string(&k).unwrap();
        *dht_2_mapping.entry(server.clone()).or_insert(0) += 1;
    }
    for (k, v) in dht_2_mapping.iter() {
        println!("DHT 2: {} -> {}", k ,v);
    }
    assert_eq!(dht_2_mapping.get(&server_1).unwrap(), &14981);
    assert_eq!(dht_2_mapping.get(&server_2).unwrap(), &15019);

    let mut dht_3_mapping: HashMap<String, u64> = HashMap::new();
    for i in 0..30000 {
        let k = format!("k - {}", i);
        let server = dht3.get_server_by_string(&k).unwrap();
        *dht_3_mapping.entry(server.clone()).or_insert(0) += 1;
    }
    for (k, v) in dht_3_mapping.iter() {
        println!("DHT 3: {} -> {}", k ,v);
    }
    assert_eq!(dht_3_mapping.get(&server_1).unwrap(), &30000);
}