use crate::helpers::*;
use crate::RedisClient;
use mini_redis::protocol::resp::RespType;

pub async fn test_geoadd_basic(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["GEOADD", "geo:test", "13.361389", "38.115556", "Palermo"]).await?;
    crate::assert_resp!(r, int(1), "GEOADD single");
    Ok(())
}

pub async fn test_geoadd_multiple(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&[
        "GEOADD", "geo:multi",
        "13.361389", "38.115556", "Palermo",
        "15.087269", "37.502669", "Catania",
    ]).await?;
    crate::assert_resp!(r, int(2), "GEOADD multiple");
    Ok(())
}

pub async fn test_geoadd_update(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["GEOADD", "geo:upd", "13.361389", "38.115556", "Palermo"]).await?;
    let r = client.cmd(&["GEOADD", "geo:upd", "13.361389", "38.115556", "Palermo"]).await?;
    crate::assert_resp!(r, int(1), "GEOADD update should still return 1 (redis counts it as added)");
    Ok(())
}

pub async fn test_geopos_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["GEOADD", "geo:pos", "13.361389", "38.115556", "Palermo"]).await?;
    let r = client.cmd(&["GEOPOS", "geo:pos", "Palermo"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.len() == 1 {
                match &items[0] {
                    RespType::Array(Some(coord)) => {
                        if coord.len() == 2 {
                            Ok(())
                        } else {
                            Err(format!("GEOPOS: expected 2 coords, got {}", coord.len()))
                        }
                    }
                    _ => Err(format!("GEOPOS: expected inner Array, got {}", items[0])),
                }
            } else {
                Err(format!("GEOPOS: expected 1 result, got {}", items.len()))
            }
        }
        _ => Err(format!("GEOPOS: expected Array, got {}", r)),
    }
}

pub async fn test_geopos_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["GEOADD", "geo:pos2", "13.361389", "38.115556", "Palermo"]).await?;
    let r = client.cmd(&["GEOPOS", "geo:pos2", "Nonexistent"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.len() == 1 && items[0] == RespType::Array(None) {
                Ok(())
            } else {
                Err(format!("GEOPOS nonexistent: expected [nil], got {}", r))
            }
        }
        _ => Err(format!("GEOPOS nonexistent: expected Array, got {}", r)),
    }
}

pub async fn test_geodist_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["GEOADD", "geo:dist", "13.361389", "38.115556", "Palermo"]).await?;
    let _ = client.cmd(&["GEOADD", "geo:dist", "15.087269", "37.502669", "Catania"]).await?;
    let r = client.cmd(&["GEODIST", "geo:dist", "Palermo", "Catania", "km"]).await?;
    match &r {
        RespType::BulkString(Some(_)) => Ok(()),
        _ => Err(format!("GEODIST: expected BulkString, got {}", r)),
    }
}

pub async fn test_geodist_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["GEOADD", "geo:dist2", "13.361389", "38.115556", "Palermo"]).await?;
    let r = client.cmd(&["GEODIST", "geo:dist2", "Palermo", "Nowhere"]).await?;
    crate::assert_resp!(r, null_bulk(), "GEODIST nonexistent member");
    Ok(())
}

pub async fn test_geohash_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["GEOADD", "geo:hash", "13.361389", "38.115556", "Palermo"]).await?;
    let r = client.cmd(&["GEOHASH", "geo:hash", "Palermo"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.len() == 1 {
                match &items[0] {
                    RespType::BulkString(Some(_)) => Ok(()),
                    _ => Err("GEOHASH: expected BulkString".to_string()),
                }
            } else {
                Err(format!("GEOHASH: expected 1 result, got {}", items.len()))
            }
        }
        _ => Err(format!("GEOHASH: expected Array, got {}", r)),
    }
}

pub async fn test_geohash_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["GEOADD", "geo:hash2", "13.361389", "38.115556", "Palermo"]).await?;
    let r = client.cmd(&["GEOHASH", "geo:hash2", "Nowhere"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.len() == 1 && items[0] == RespType::BulkString(None) {
                Ok(())
            } else {
                Err(format!("GEOHASH nonexistent: expected [nil], got {}", r))
            }
        }
        _ => Err(format!("GEOHASH nonexistent: expected Array, got {}", r)),
    }
}

pub async fn test_georadius_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&[
        "GEOADD", "geo:rad",
        "13.361389", "38.115556", "Palermo",
        "15.087269", "37.502669", "Catania",
    ]).await?;
    let r = client.cmd(&["GEORADIUS", "geo:rad", "15", "37", "200", "km"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if !items.is_empty() {
                Ok(())
            } else {
                Err("GEORADIUS: expected non-empty array".to_string())
            }
        }
        _ => Err(format!("GEORADIUS: expected Array, got {}", r)),
    }
}

pub async fn test_georadius_withdist(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&[
        "GEOADD", "geo:rad2",
        "13.361389", "38.115556", "Palermo",
        "15.087269", "37.502669", "Catania",
    ]).await?;
    let r = client.cmd(&["GEORADIUS", "geo:rad2", "15", "37", "200", "km", "WITHDIST"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.is_empty() {
                return Err("GEORADIUS WITH DIST: expected non-empty array".to_string());
            }
            // Each item should be a sub-array with member + distance
            match &items[0] {
                RespType::Array(Some(parts)) => {
                    if parts.len() == 2 {
                        Ok(())
                    } else {
                        Err(format!("GEORADIUS WITH DIST: expected 2 parts, got {}", parts.len()))
                    }
                }
                _ => Err("GEORADIUS WITH DIST: expected sub-array items".to_string()),
            }
        }
        _ => Err(format!("GEORADIUS WITH DIST: expected Array, got {}", r)),
    }
}

pub async fn test_georadius_withcoord(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&[
        "GEOADD", "geo:rad3",
        "13.361389", "38.115556", "Palermo",
    ]).await?;
    let r = client.cmd(&["GEORADIUS", "geo:rad3", "13", "38", "100", "km", "WITHCOORD"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.is_empty() {
                return Err("GEORADIUS WITHCOORD: expected non-empty array".to_string());
            }
            match &items[0] {
                RespType::Array(Some(parts)) => {
                    if parts.len() == 2 {
                        Ok(())
                    } else {
                        Err(format!("GEORADIUS WITHCOORD: expected 2 parts, got {}", parts.len()))
                    }
                }
                _ => Err("GEORADIUS WITHCOORD: expected sub-array items".to_string()),
            }
        }
        _ => Err(format!("GEORADIUS WITHCOORD: expected Array, got {}", r)),
    }
}

pub async fn test_georadius_count(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&[
        "GEOADD", "geo:rad4",
        "13.361389", "38.115556", "Palermo",
        "15.087269", "37.502669", "Catania",
        "12.5", "41.9", "Rome",
    ]).await?;
    let r = client.cmd(&["GEORADIUS", "geo:rad4", "13", "38", "500", "km", "COUNT", "1"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if items.len() == 1 {
                Ok(())
            } else {
                Err(format!("GEORADIUS COUNT 1: expected 1 result, got {}", items.len()))
            }
        }
        _ => Err(format!("GEORADIUS COUNT: expected Array, got {}", r)),
    }
}

pub async fn test_georadiusbymember_basic(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&[
        "GEOADD", "geo:rbm",
        "13.361389", "38.115556", "Palermo",
        "15.087269", "37.502669", "Catania",
    ]).await?;
    let r = client.cmd(&["GEORADIUSBYMEMBER", "geo:rbm", "Palermo", "200", "km"]).await?;
    match &r {
        RespType::Array(Some(items)) => {
            if !items.is_empty() {
                Ok(())
            } else {
                Err("GEORADIUSBYMEMBER: expected non-empty array".to_string())
            }
        }
        _ => Err(format!("GEORADIUSBYMEMBER: expected Array, got {}", r)),
    }
}

pub async fn test_georadiusbymember_nonexistent(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["GEOADD", "geo:rbm2", "13.361389", "38.115556", "Palermo"]).await?;
    let r = client.cmd(&["GEORADIUSBYMEMBER", "geo:rbm2", "Nowhere", "100", "km"]).await?;
    crate::assert_resp!(r, empty_array(), "GEORADIUSBYMEMBER nonexistent member");
    Ok(())
}

pub async fn test_geo_wrongtype(client: &mut RedisClient) -> Result<(), String> {
    let _ = client.cmd(&["SET", "geo:wt", "stringvalue"]).await?;
    let r = client.cmd(&["GEOADD", "geo:wt", "13.36", "38.11", "Palermo"]).await?;
    match &r {
        RespType::Error(msg) => {
            if msg.starts_with("WRONGTYPE") {
                Ok(())
            } else {
                Err(format!("expected WRONGTYPE error, got: {}", msg))
            }
        }
        _ => Err(format!("GEOADD on string: expected Error, got {}", r)),
    }
}

pub async fn test_geoadd_wrong_arg_count(client: &mut RedisClient) -> Result<(), String> {
    let r = client.cmd(&["GEOADD", "geo:err"]).await?;
    match &r {
        RespType::Error(_) => Ok(()),
        _ => Err(format!("GEOADD wrong args: expected Error, got {}", r)),
    }
}
