use block_explorer_backend::db;
use std::path::Path;
use std::env;
use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = Path::new("blocks.db");

    println!("Block Explorer Indexer");
    println!("Fetching blocks from regtest node at http://127.0.0.1:18443");

    println!("Initializing database...");
    let conn = db::init_db(db_path)?;

    let client = reqwest::Client::new();

    // Get block count
    let response = client
        .post("http://127.0.0.1:18443")
        .basic_auth("user", Some("pass"))
        .json(&json!({"jsonrpc": "1.0", "id": "1", "method": "getblockcount", "params": []}))
        .send()
        .await?;
    let result: serde_json::Value = response.json().await?;
    let count = result["result"].as_u64().unwrap_or(0) as u32;

    println!("Starting block indexing... Total blocks: {}", count);

    for height in 0..count {
        // Get block hash
        let response = client
            .post("http://127.0.0.1:18443")
            .basic_auth("user", Some("pass"))
            .json(&json!({"jsonrpc": "1.0", "id": "1", "method": "getblockhash", "params": [height]}))
            .send()
            .await?;
        let result: serde_json::Value = response.json().await?;
        let hash = result["result"].as_str().unwrap();

        // Get block hex
        let response = client
            .post("http://127.0.0.1:18443")
            .basic_auth("user", Some("pass"))
            .json(&json!({"jsonrpc": "1.0", "id": "1", "method": "getblock", "params": [hash, 0]}))
            .send()
            .await?;
        let result: serde_json::Value = response.json().await?;
        let hex = result["result"].as_str().unwrap();

        let block_bytes = hex::decode(hex).unwrap();
        let block: bitcoin::Block = bitcoin::consensus::deserialize(&block_bytes).unwrap();

        db::insert_block(&conn, &block, height)?;
        println!("Indexed block at height {}: {}", height, hash);
    }

    let block_count: u32 = conn.query_row(
        "SELECT COUNT(*) FROM blocks", [], |row| row.get(0)
    )?;
    let tx_count: u64 = conn.query_row(
        "SELECT COUNT(*) FROM transactions", [], |row| row.get(0)
    )?;

    println!("Indexing complete!");
    println!("Blocks: {}", block_count);
    println!("Transactions: {}", tx_count);

    Ok(())
}
