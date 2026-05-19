use crate::helpers;
use crate::RedisClient;

pub async fn test_multi_exec_basic(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["MULTI"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "MULTI");

    let resp = client.cmd(&["SET", "txn:basic", "value1"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("QUEUED"), "SET should be queued");

    let resp = client.cmd(&["EXEC"]).await?;
    match &resp {
        mini_redis::protocol::resp::RespType::Array(Some(items)) if items.len() == 1 => {
            crate::assert_resp!(items[0].clone(), helpers::simple_str("OK"), "EXEC SET result");
        }
        _ => return Err(format!("EXEC: expected Array(1), got {}", resp)),
    }

    // Verify key was actually set
    let resp = client.cmd(&["GET", "txn:basic"]).await?;
    crate::assert_resp!(resp, helpers::bulk_str("value1"), "GET after EXEC");
    Ok(())
}

pub async fn test_multi_discard(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["MULTI"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "MULTI");

    let resp = client.cmd(&["SET", "txn:discard", "should_not_exist"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("QUEUED"), "SET should be queued");

    let resp = client.cmd(&["DISCARD"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "DISCARD");

    // After DISCARD, the key should not have been set
    let resp = client.cmd(&["GET", "txn:discard"]).await?;
    crate::assert_resp!(resp, helpers::null_bulk(), "GET after DISCARD should be nil");
    Ok(())
}

pub async fn test_exec_without_multi(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["EXEC"]).await?;
    crate::assert_resp!(
        resp,
        mini_redis::protocol::resp::RespType::Error("ERR EXEC without MULTI".to_string()),
        "EXEC without MULTI"
    );
    Ok(())
}

pub async fn test_discard_without_multi(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["DISCARD"]).await?;
    crate::assert_resp!(
        resp,
        mini_redis::protocol::resp::RespType::Error("ERR DISCARD without MULTI".to_string()),
        "DISCARD without MULTI"
    );
    Ok(())
}

pub async fn test_nested_multi(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["MULTI"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "first MULTI");

    let resp = client.cmd(&["MULTI"]).await?;
    crate::assert_resp!(
        resp,
        mini_redis::protocol::resp::RespType::Error("ERR MULTI calls can not be nested".to_string()),
        "nested MULTI"
    );

    // Clean up — discard the transaction left open by the first MULTI
    let resp = client.cmd(&["DISCARD"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "DISCARD to clean up");
    Ok(())
}

pub async fn test_watch_then_exec(client: &mut RedisClient) -> Result<(), String> {
    // Set a key first
    let resp = client.cmd(&["SET", "txn:watch", "original"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET before WATCH");

    // WATCH the key
    let resp = client.cmd(&["WATCH", "txn:watch"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "WATCH");

    // Start transaction
    let resp = client.cmd(&["MULTI"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "MULTI");

    // Queue a SET
    let resp = client.cmd(&["SET", "txn:watch", "newvalue"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("QUEUED"), "SET should be queued");

    // EXEC — should succeed since key wasn't modified between WATCH and EXEC
    let resp = client.cmd(&["EXEC"]).await?;
    match &resp {
        mini_redis::protocol::resp::RespType::Array(Some(items)) if items.len() == 1 => {
            crate::assert_resp!(items[0].clone(), helpers::simple_str("OK"), "EXEC SET result");
        }
        _ => return Err(format!("EXEC: expected Array(1), got {}", resp)),
    }

    // Verify
    let resp = client.cmd(&["GET", "txn:watch"]).await?;
    crate::assert_resp!(resp, helpers::bulk_str("newvalue"), "GET after WATCH EXEC");
    Ok(())
}

pub async fn test_unwatch(client: &mut RedisClient) -> Result<(), String> {
    // Set a key
    let resp = client.cmd(&["SET", "txn:unwatch", "original"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "SET before WATCH");

    // WATCH then UNWATCH
    let resp = client.cmd(&["WATCH", "txn:unwatch"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "WATCH");

    let resp = client.cmd(&["UNWATCH"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "UNWATCH");

    // Start transaction
    let resp = client.cmd(&["MULTI"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("OK"), "MULTI");

    let resp = client.cmd(&["SET", "txn:unwatch", "after_unwatch"]).await?;
    crate::assert_resp!(resp, helpers::simple_str("QUEUED"), "SET should be queued");

    // EXEC should succeed
    let resp = client.cmd(&["EXEC"]).await?;
    match &resp {
        mini_redis::protocol::resp::RespType::Array(Some(items)) if items.len() == 1 => {
            crate::assert_resp!(items[0].clone(), helpers::simple_str("OK"), "EXEC SET result");
        }
        _ => return Err(format!("EXEC: expected Array(1), got {}", resp)),
    }

    let resp = client.cmd(&["GET", "txn:unwatch"]).await?;
    crate::assert_resp!(resp, helpers::bulk_str("after_unwatch"), "GET after UNWATCH EXEC");
    Ok(())
}
