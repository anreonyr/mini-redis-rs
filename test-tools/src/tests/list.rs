use crate::helpers::*;
use crate::RedisClient;
use mini_redis::protocol::resp::RespType;

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

pub async fn test_rpop_single(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["RPUSH", "test_rs:rpop", "a", "b", "c"]).await?;
    let r = client.cmd(&["RPOP", "test_rs:rpop"]).await?;
    crate::assert_resp!(r, bulk_str("c"), "RPOP single");
    Ok(())
}

pub async fn test_rpop_with_count(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["RPUSH", "test_rs:rpop2", "a", "b", "c", "d"]).await?;
    let r = client.cmd(&["RPOP", "test_rs:rpop2", "2"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["d", "c"]), "RPOP count 2");
    Ok(())
}

pub async fn test_rpop_empty_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["RPOP", "test_rs:nonex"]).await?;
    crate::assert_resp!(r, null_bulk(), "RPOP empty key");
    Ok(())
}

pub async fn test_lindex_basic(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["RPUSH", "test_rs:lidx", "a", "b", "c"]).await?;
    let r = client.cmd(&["LINDEX", "test_rs:lidx", "0"]).await?;
    crate::assert_resp!(r, bulk_str("a"), "LINDEX 0");
    let r = client.cmd(&["LINDEX", "test_rs:lidx", "-1"]).await?;
    crate::assert_resp!(r, bulk_str("c"), "LINDEX -1");
    Ok(())
}

pub async fn test_lindex_out_of_bounds(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LINDEX", "test_rs:lidx", "10"]).await?;
    crate::assert_resp!(r, null_bulk(), "LINDEX out of bounds");
    Ok(())
}

pub async fn test_lindex_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LINDEX", "test_rs:nokey", "0"]).await?;
    crate::assert_resp!(r, null_bulk(), "LINDEX nonexistent key");
    Ok(())
}

pub async fn test_lrem_positive_count(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["RPUSH", "test_rs:lrem", "a", "b", "a", "c", "a"]).await?;
    let r = client.cmd(&["LREM", "test_rs:lrem", "2", "a"]).await?;
    crate::assert_resp!(r, int(2), "LREM count 2");
    let r = client.cmd(&["LRANGE", "test_rs:lrem", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["b", "c", "a"]), "LREM result");
    Ok(())
}

pub async fn test_lrem_negative_count(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["RPUSH", "test_rs:lrem2", "a", "b", "a", "c", "a"]).await?;
    let r = client.cmd(&["LREM", "test_rs:lrem2", "-2", "a"]).await?;
    crate::assert_resp!(r, int(2), "LREM count -2");
    let r = client.cmd(&["LRANGE", "test_rs:lrem2", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["a", "b", "c"]), "LREM negative result");
    Ok(())
}

pub async fn test_lrem_all(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["RPUSH", "test_rs:lrem3", "a", "b", "a", "c", "a"]).await?;
    let r = client.cmd(&["LREM", "test_rs:lrem3", "0", "a"]).await?;
    crate::assert_resp!(r, int(3), "LREM count 0");
    let r = client.cmd(&["LRANGE", "test_rs:lrem3", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["b", "c"]), "LREM all result");
    Ok(())
}

pub async fn test_lrem_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LREM", "test_rs:nokey", "0", "a"]).await?;
    crate::assert_resp!(r, int(0), "LREM nonexistent key");
    Ok(())
}

pub async fn test_ltrim_basic(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["RPUSH", "test_rs:ltrim", "0", "1", "2", "3", "4"]).await?;
    let r = client.cmd(&["LTRIM", "test_rs:ltrim", "1", "3"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "LTRIM OK");
    let r = client.cmd(&["LRANGE", "test_rs:ltrim", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["1", "2", "3"]), "LTRIM result");
    Ok(())
}

pub async fn test_ltrim_negative_indices(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["RPUSH", "test_rs:ltrim2", "a", "b", "c", "d"]).await?;
    let r = client.cmd(&["LTRIM", "test_rs:ltrim2", "-2", "-1"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "LTRIM negative");
    let r = client.cmd(&["LRANGE", "test_rs:ltrim2", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["c", "d"]), "LTRIM negative result");
    Ok(())
}

pub async fn test_ltrim_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["LTRIM", "test_rs:nokey", "0", "1"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "LTRIM nonexistent key");
    Ok(())
}

pub async fn test_rpoplpush(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["RPUSH", "test_rs:rpls", "a", "b", "c"]).await?;
    let r = client.cmd(&["RPOPLPUSH", "test_rs:rpls", "test_rs:rpld"]).await?;
    crate::assert_resp!(r, bulk_str("c"), "RPOPLPUSH popped");
    let r = client.cmd(&["LRANGE", "test_rs:rpls", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["a", "b"]), "RPOPLPUSH source");
    let r = client.cmd(&["LRANGE", "test_rs:rpld", "0", "-1"]).await?;
    crate::assert_resp!(r, arr_of_bulks(&["c"]), "RPOPLPUSH dest");
    Ok(())
}

pub async fn test_lset(client: &mut RedisClient) -> Result<(), String> {
    client.cmd(&["RPUSH", "test_rs:lset", "a", "b", "c"]).await?;
    let r = client.cmd(&["LSET", "test_rs:lset", "1", "x"]).await?;
    crate::assert_resp!(r, simple_str("OK"), "LSET");
    let r = client.cmd(&["LINDEX", "test_rs:lset", "1"]).await?;
    crate::assert_resp!(r, bulk_str("x"), "LSET verify");
    Ok(())
}
