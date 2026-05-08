use crate::helpers::*;
use crate::RedisClient;
use mini_redis::resp::RespType;

pub async fn test_rpush_new_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["RPUSH", "test_rs:list", "a", "b", "c"]).await?;
    crate::assert_resp!(r, int(3), "RPUSH new key");
    let r = client.cmd(&["LRANGE", "test_rs:list", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["a", "b", "c"]), "LRANGE verify");
    Ok(())
}

pub async fn test_rpush_existing_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["RPUSH", "test_rs:list", "d", "e"]).await?;
    crate::assert_resp!(r, int(5), "RPUSH existing key");
    let r = client.cmd(&["LRANGE", "test_rs:list", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["a", "b", "c", "d", "e"]), "LRANGE after RPUSH");
    Ok(())
}

pub async fn test_lpush_new_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LPUSH", "test_rs:list2", "x", "y"]).await?;
    crate::assert_resp!(r, int(2), "LPUSH new key");
    let r = client.cmd(&["LRANGE", "test_rs:list2", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["y", "x"]), "LRANGE after LPUSH");
    Ok(())
}

pub async fn test_lrange_positive_indices(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LRANGE", "test_rs:list", "1", "2"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["b", "c"]), "LRANGE positive indices");
    Ok(())
}

pub async fn test_lrange_negative_indices(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LRANGE", "test_rs:list", "-2", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["d", "e"]), "LRANGE negative indices");
    Ok(())
}

pub async fn test_lrange_out_of_bounds(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LRANGE", "test_rs:list", "10", "20"]).await?;
    crate::assert_resp!(r, empty_array(), "LRANGE out of bounds");
    Ok(())
}

pub async fn test_lrange_empty_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LRANGE", "test_rs:nonexlist", "0", "-1"]).await?;
    crate::assert_resp!(r, empty_array(), "LRANGE empty key");
    Ok(())
}

pub async fn test_llen(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LLEN", "test_rs:list"]).await?;
    crate::assert_resp!(r, int(5), "LLEN");
    Ok(())
}

pub async fn test_llen_empty_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LLEN", "test_rs:nonexlist"]).await?;
    crate::assert_resp!(r, int(0), "LLEN empty key");
    Ok(())
}

pub async fn test_lpop_single(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LPOP", "test_rs:list"]).await?;
    crate::assert_resp!(r, bulk_str("a"), "LPOP single");
    let r = client.cmd(&["LLEN", "test_rs:list"]).await?;
    crate::assert_resp!(r, int(4), "LLEN after LPOP");
    Ok(())
}

pub async fn test_lpop_with_count(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LPOP", "test_rs:list", "2"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["b", "c"]), "LPOP with count 2");
    let r = client.cmd(&["LLEN", "test_rs:list"]).await?;
    crate::assert_resp!(r, int(2), "LLEN after LPOP 2");
    Ok(())
}

pub async fn test_lpop_count_zero(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LPOP", "test_rs:list", "0"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.is_empty() => Ok(()),
        _ => Err(format!("LPOP count=0: expected empty array, got {}", r)),
    }
}

pub async fn test_lpop_empty_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LPOP", "test_rs:nonexlist"]).await?;
    crate::assert_resp!(r, null_bulk(), "LPOP empty key");
    Ok(())
}

pub async fn test_lpop_count_larger_than_list(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LPOP", "test_rs:list", "10"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 2 => {
            let r2 = client.cmd(&["LLEN", "test_rs:list"]).await?;
            crate::assert_resp!(r2, int(0), "LLEN after LPOP count > len");
            Ok(())
        }
        _ => Err(format!("LPOP count>len: expected Array of 2, got {}", r)),
    }
}

pub async fn test_large_list_lrange(client: &mut RedisClient) -> Result<(), String> {
    let mut args: Vec<&str> = vec!["RPUSH", "test_rs:biglist"];
    let num_strs: Vec<String> = (0..1000).map(|i| i.to_string()).collect();
    let str_refs: Vec<&str> = num_strs.iter().map(|s| s.as_str()).collect();
    args.extend(&str_refs);
    let r = client.cmd(&args).await?;
    crate::assert_resp!(r, int(1000), "RPUSH 1000 elements");
    let r = client.cmd(&["LRANGE", "test_rs:biglist", "0", "-1"]).await?;
    match &r {
        RespType::Array(Some(items)) if items.len() == 1000 => Ok(()),
        _ => Err(format!("LRANGE 1000: expected Array of 1000, got {}", r)),
    }
}

pub async fn test_list_empty_string_element(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["RPUSH", "test_rs:emptylist", ""]).await?;
    crate::assert_resp!(r, int(1), "RPUSH empty string");
    let r = client.cmd(&["LPOP", "test_rs:emptylist"]).await?;
    crate::assert_resp!(r, bulk_str(""), "LPOP empty string");
    Ok(())
}
