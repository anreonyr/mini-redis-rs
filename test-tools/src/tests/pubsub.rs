use crate::helpers;
use crate::RedisClient;

pub async fn test_publish_no_subscribers(client: &mut RedisClient) -> Result<(), String> {
    let resp = client.cmd(&["PUBLISH", "test:channel", "hello"]).await?;
    crate::assert_resp!(resp, helpers::int(0), "PUBLISH with no subscribers should return 0");
    Ok(())
}
