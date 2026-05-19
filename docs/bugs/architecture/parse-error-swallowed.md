---
phase: architecture
issue_id: parse-error-swallowed
keywords: [dispatch-logic, error-handling, parser, command-parsing]
files: [mini-redis/src/cmd/parse.rs]
severity: high
---

## 现象

SET 命令测试失败：合法参数收到 `ERR unknown command`（如 `SET key value INVALID_FLAG val` 应当返回 syntax error，实际返回 unknown command）。

## 根本原因

`parse()` 重构后用 `if let Ok(p) = parser_func(cmd, args)` 来判断命令是否被解析器处理。但分类解析器对**识别到命令但参数错误**的情况返回 `Err(SyntaxError)` / `Err(WrongArgCount)` 等非 `UnknownCommand` 错误——`if let Ok` 不匹配这些错误，代码继续尝试下一个解析器，最终所有解析器都返回 `Err(UnknownCommand)`。

```
parse_string_cmd("SET", bad_args) → Err(SyntaxError)
  → 被 if let Ok 跳过
parse_list_cmd("SET", bad_args) → Err(UnknownCommand)
  → 被 if let Ok 跳过
...
→ 最终返回 Err(UnknownCommand) ← BUG
```

## 修复方案

使用 `try_parser!` 宏：只对 `Err(CmdError::UnknownCommand)` 不做处理（继续尝试下一个解析器），其他错误直接传播。

```rust
macro_rules! try_parser {
    ($parser:ident) => {
        match super::parsers::$parser::cmd(cmd, args.clone()) {
            Err(CmdError::UnknownCommand) => {}
            other => return other,
        }
    };
}
```

## 禁止模式

- 使用 fallthrough 分派时，必须区分"我不处理这个输入"和"我处理了但出错了"，不能把两者都当作 Err 跳过
