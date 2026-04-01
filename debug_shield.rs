/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use bastion::net_guard::ShieldClient;

#[tokio::main]
async fn main() {
    let shield = ShieldClient::builder()
        .allow_endpoint("127.0.0.1:8188")
        .block_private_ips(true)
        .build()
        .unwrap();
    
    let url1 = "http://127.0.0.1:8188";
    let url2 = "http://127.0.0.1:8188/";
    
    println!("Testing {}: {:?}", url1, shield.validate_url(url1).await);
    println!("Testing {}: {:?}", url2, shield.validate_url(url2).await);
}
