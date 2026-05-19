# Auth System Design

## 背景

Redis 服务器没有任何鉴权机制。需要添加标准密码认证功能。

## 方案选择

采用 ConnectionState 结构体的方案——创建 `cmd/auth.rs` 模块，每连接维护认证状态，通过 `dispatch_command()` 签名扩展注入鉴权检查。

## 架构

```
main.rs
  ├── config.rs       (全局 ServerConfig, requirepass)
  ├── cmd/auth.rs     (ConnectionState, handle_auth, is_allowed_before_auth)
  ├── cmd/types.rs    (ParsedCmd::Auth, name() 方法)
  ├── cmd/parse.rs    (AUTH 解析 + CONFIG SET 解析)
  ├── cmd/dispatch.rs (鉴权拦截 + AUTH/CONFIG SET 路由)
  └── cmd/handlers/connection.rs (CONFIG GET/SET requirepass)
```

## 核心流程

1. 启动时从 `--requirepass` CLI 参数或 `REDIS_PASSWORD` 环境变量读取密码
2. 每连接创建 `ConnectionState { authenticated: false }`
3. 每个命令执行前检查：requirepass 已设置 + 未认证 + 非 bypass 命令 → 返回 `-NOAUTH`
4. `AUTH <password>` 校验通过后设置 `authenticated = true`
5. `CONFIG SET requirepass <pw>` 运行时修改密码
6. `requirepass` 为 `None` 时鉴权完全禁用（向后兼容）

## 免认证命令

AUTH, PING, ECHO, COMMAND, QUIT, HELLO

## 文件变更

| 操作 | 文件 |
|------|------|
| 新建 | `mini-redis/src/config.rs` |
| 新建 | `mini-redis/src/cmd/auth.rs` |
| 新建 | `test-tools/src/tests/auth.rs` |
| 修改 | `mini-redis/src/cmd/types.rs` |
| 修改 | `mini-redis/src/cmd/parse.rs` |
| 修改 | `mini-redis/src/cmd/dispatch.rs` |
| 修改 | `mini-redis/src/cmd/handlers/connection.rs` |
| 修改 | `mini-redis/src/cmd/mod.rs` |
| 修改 | `mini-redis/src/cmd/handlers/mod.rs` |
| 修改 | `mini-redis/src/lib.rs` |
| 修改 | `mini-redis/src/registry.rs` |
| 修改 | `mini-redis/src/main.rs` |
| 修改 | `test-tools/src/tests/mod.rs` |
| 修改 | `test-tools/src/lib.rs` |

## 不涉及的文件

db.rs, resp.rs, inline.rs, blocking.rs

## 测试

- `test_auth_basic`: 设密码 → NOAUTH → AUTH 错误/正确 → 操作正常 → 恢复无密码
- `test_auth_bypass`: PING/ECHO 在未认证时仍可执行
- `test_auth_config`: CONFIG GET/SET requirepass 读写
- `test_auth_disabled`: 未设密码时所有命令正常 + AUTH 报错提示
