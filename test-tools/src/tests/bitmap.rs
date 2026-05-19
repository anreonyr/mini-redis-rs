use crate::helpers::*;
use crate::RedisClient;
use mini_redis::resp::RespType;

pub async fn test_getbit_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "bit:test", "a"]).await?; // 'a' = 0b01100001
    let r = client.cmd(&["GETBIT", "bit:test", "1"]).await?;
    crate::assert_resp!(r, int(1), "GETBIT offset 1");
    let r2 = client.cmd(&["GETBIT", "bit:test", "0"]).await?;
    crate::assert_resp!(r2, int(0), "GETBIT offset 0");
    Ok(())
}

pub async fn test_getbit_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["GETBIT", "bit:nonexist", "0"]).await?;
    crate::assert_resp!(r, int(0), "GETBIT nonexistent");
    Ok(())
}

pub async fn test_setbit_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "bit:setbit", "\x00"]).await?;
    let r = client.cmd(&["SETBIT", "bit:setbit", "0", "1"]).await?;
    crate::assert_resp!(r, int(0), "SETBIT returns old bit 0");
    let r2 = client.cmd(&["GETBIT", "bit:setbit", "0"]).await?;
    crate::assert_resp!(r2, int(1), "GETBIT after SETBIT");
    Ok(())
}

pub async fn test_setbit_new_key(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["SETBIT", "bit:new", "7", "1"]).await?;
    crate::assert_resp!(r, int(0), "SETBIT on new key returns 0");
    let r2 = client.cmd(&["GETBIT", "bit:new", "7"]).await?;
    crate::assert_resp!(r2, int(1), "GETBIT after SETBIT on new key");
    Ok(())
}

pub async fn test_bitcount(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "bit:count", "\x00\x55"]).await?; // 0 + 4 = 4 bits set
    let r = client.cmd(&["BITCOUNT", "bit:count"]).await?;
    crate::assert_resp!(r, int(4), "BITCOUNT");
    Ok(())
}

pub async fn test_bitcount_range(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "bit:countr", "\x55\x00\x55"]).await?;
    let r = client.cmd(&["BITCOUNT", "bit:countr", "0", "0"]).await?;
    crate::assert_resp!(r, int(4), "BITCOUNT range start=0 end=0");
    let r2 = client.cmd(&["BITCOUNT", "bit:countr", "1", "1"]).await?;
    crate::assert_resp!(r2, int(0), "BITCOUNT range start=1 end=1");
    Ok(())
}

pub async fn test_bitop_and(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "bit:op1", "U"]).await?; // 'U' = 0x55 = 0b01010101
    let _ = client.cmd(&["SET", "bit:op2", "\x03"]).await?; // 0x03 = 0b00000011
    let r = client.cmd(&["BITOP", "AND", "bit:op_dest", "bit:op1", "bit:op2"]).await?;
    crate::assert_resp!(r, int(1), "BITOP AND length");
    let r2 = client.cmd(&["GET", "bit:op_dest"]).await?;
    crate::assert_resp!(r2, bulk_str("\x01"), "BITOP AND result"); // 0x55 & 0x03 = 0x01
    Ok(())
}

pub async fn test_bitop_or(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "bit:or1", "U"]).await?; // 0x55
    let _ = client.cmd(&["SET", "bit:or2", "\x0a"]).await?; // 0x0a = '\n'
    let r = client.cmd(&["BITOP", "OR", "bit:or_dest", "bit:or1", "bit:or2"]).await?;
    crate::assert_resp!(r, int(1), "BITOP OR length");
    let r2 = client.cmd(&["GET", "bit:or_dest"]).await?;
    crate::assert_resp!(r2, bulk_str("\x5f"), "BITOP OR result"); // 0x55 | 0x0a = 0x5f
    Ok(())
}

pub async fn test_bitop_not(client: &mut RedisClient) -> Result<(), String> {
    // BITOP NOT produces bytes with high bit set, so we verify via double-NOT
    let _ = client.cmd(&["SET", "bit:not_src", "U"]).await?; // 0x55
    let r1 = client.cmd(&["BITOP", "NOT", "bit:not_mid", "bit:not_src"]).await?;
    crate::assert_resp!(r1, int(1), "BITOP NOT length");
    let r2 = client.cmd(&["BITOP", "NOT", "bit:not_dest", "bit:not_mid"]).await?;
    crate::assert_resp!(r2, int(1), "BITOP NOT (2nd) length");
    let r3 = client.cmd(&["GET", "bit:not_dest"]).await?;
    crate::assert_resp!(r3, bulk_str("U"), "BITOP double NOT restores original");
    Ok(())
}

pub async fn test_bitop_xor(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "bit:xor1", "U"]).await?; // 0x55
    let _ = client.cmd(&["SET", "bit:xor2", "\x0a"]).await?; // 0x0a
    let r = client.cmd(&["BITOP", "XOR", "bit:xor_dest", "bit:xor1", "bit:xor2"]).await?;
    crate::assert_resp!(r, int(1), "BITOP XOR length");
    let r2 = client.cmd(&["GET", "bit:xor_dest"]).await?;
    crate::assert_resp!(r2, bulk_str("\x5f"), "BITOP XOR result"); // 0x55 ^ 0x0a = 0x5f
    Ok(())
}

pub async fn test_bitpos(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "bit:pos", "\x00"]).await?; // 0x00 = 0b00000000
    let r = client.cmd(&["BITPOS", "bit:pos", "1"]).await?;
    crate::assert_resp!(r, int(-1), "BITPOS first 1 in all-zero string returns -1");
    let _ = client.cmd(&["SET", "bit:pos2", "\x01"]).await?; // 0x01 = 0b00000001
    let r2 = client.cmd(&["BITPOS", "bit:pos2", "1"]).await?;
    crate::assert_resp!(r2, int(7), "BITPOS first 1 at position 7");
    let r3 = client.cmd(&["BITPOS", "bit:pos2", "0"]).await?;
    crate::assert_resp!(r3, int(0), "BITPOS first 0 at position 0");
    Ok(())
}

pub async fn test_getbit_wrongtype(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["RPUSH", "bit:wt", "a"]).await?;
    let r = client.cmd(&["GETBIT", "bit:wt", "0"]).await?;
    assert!(matches!(r, RespType::Error(_)), "GETBIT on list should return error");
    Ok(())
}
