use crate::helpers::*;
use crate::RedisClient;
use mini_redis::resp::RespType;

pub async fn test_auth_basic(client: &mut RedisClient) -> Result<(), String> {
    // Enable password
    client.cmd(&["CONFIG", "SET", "requirepass", "testpw"]).await?;

    // Without auth, commands should be rejected
    let r = client.cmd(&["GET", "foo"]).await?;
    match &r {
        RespType::Error(msg) if msg.contains("NOAUTH") => {}
        _ => return Err(format!("expected NOAUTH error, got {}", r)),
    }

    // Wrong password
    let r = client.cmd(&["AUTH", "wrongpass"]).await?;
    match &r {
        RespType::Error(msg) if msg.contains("invalid password") => {}
        _ => return Err(format!("expected invalid password error, got {}", r)),
    }

    // Still rejected after wrong password
    let r = client.cmd(&["GET", "foo"]).await?;
    match &r {
        RespType::Error(msg) if msg.contains("NOAUTH") => {}
        _ => return Err(format!("expected NOAUTH error after failed auth, got {}", r)),
    }

    // Correct password
    let r = client.cmd(&["AUTH", "testpw"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "AUTH with correct password");

    // Now commands should work
    let r = client.cmd(&["GET", "foo"]).await?;
    crate::assert_resp!(r, null_bulk(), "GET after AUTH");

    // SET then GET
    let r = client.cmd(&["SET", "foo", "bar"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "SET after AUTH");
    let r = client.cmd(&["GET", "foo"]).await?;
    crate::assert_resp!(r, bulk_str("bar"), "GET after AUTH");

    // Disable password
    let r = client.cmd(&["CONFIG", "SET", "requirepass", ""]).await?;
    crate::assert_resp!(r, simple_str("OK"), "disable password");

    Ok(())
}

pub async fn test_auth_bypass(client: &mut RedisClient) -> Result<(), String> {
    // Enable password
    client.cmd(&["CONFIG", "SET", "requirepass", "testpw"]).await?;

    // PING should work without auth (bypass)
    let r = client.cmd(&["PING"]).await?;
    crate::assert_resp!(r, simple_str("PONG"), "PING before auth");

    // ECHO should work without auth (bypass)
    let r = client.cmd(&["ECHO", "hello"]).await?;
    crate::assert_resp!(r, bulk_str("hello"), "ECHO before auth");

    // Disable password
    client.cmd(&["CONFIG", "SET", "requirepass", ""]).await?;
    Ok(())
}

pub async fn test_auth_config(client: &mut RedisClient) -> Result<(), String> {
    // Initially no password
    let r = client.cmd(&["CONFIG", "GET", "requirepass"]).await?;
    crate::assert_resp!(r, empty_array(), "CONFIG GET requirepass (no password)");

    // Set password via CONFIG SET
    let r = client.cmd(&["CONFIG", "SET", "requirepass", "newpass"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "CONFIG SET requirepass");

    // Verify via CONFIG GET (should return the password)
    let r = client.cmd(&["CONFIG", "GET", "requirepass"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {}
        _ => return Err(format!("CONFIG GET requirepass: expected array of 2, got {}", r)),
    }

    // AUTH with new password
    let r = client.cmd(&["AUTH", "newpass"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "AUTH with new password");

    // Clear password
    client.cmd(&["CONFIG", "SET", "requirepass", ""]).await?;
    Ok(())
}

pub async fn test_auth_disabled(client: &mut RedisClient) -> Result<(), String> {
    // Ensure no password is set
    client.cmd(&["CONFIG", "SET", "requirepass", ""]).await?;

    // All commands should work normally
    let r = client.cmd(&["PING"]).await?;
    crate::assert_resp!(r, simple_str("PONG"), "PING with auth disabled");

    let r = client.cmd(&["SET", "x", "1"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "SET with auth disabled");

    let r = client.cmd(&["GET", "x"]).await?;
    crate::assert_resp!(r, bulk_str("1"), "GET with auth disabled");

    // AUTH without password configured should give error
    let r = client.cmd(&["AUTH", "anything"]).await?;
    match &r {
        RespType::Error(msg) if msg.contains("without a password") => {}
        _ => return Err(format!("expected 'without a password' error, got {}", r)),
    }

    Ok(())
}
