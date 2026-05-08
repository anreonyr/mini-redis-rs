use mini_redis::inline;

// --- strip_iac ---

#[test]
fn test_strip_iac_empty() {
    let mut buf = vec![];
    inline::strip_iac(&mut buf);
    assert!(buf.is_empty());
}

#[test]
fn test_strip_iac_no_iac() {
    let mut buf = b"hello world".to_vec();
    inline::strip_iac(&mut buf);
    assert_eq!(buf, b"hello world");
}

#[test]
fn test_strip_iac_double_ff() {
    let mut buf = vec![0xFF, 0xFF];
    inline::strip_iac(&mut buf);
    assert_eq!(buf, vec![0xFF]); // IAC IAC -> literal 0xFF
}

#[test]
fn test_strip_iac_will() {
    let mut buf = vec![0xFF, 0xFB, 0x03]; // IAC WILL 3
    inline::strip_iac(&mut buf);
    assert!(buf.is_empty());
}

#[test]
fn test_strip_iac_dont() {
    let mut buf = vec![0xFF, 0xFE, 0x18]; // IAC DONT 24
    inline::strip_iac(&mut buf);
    assert!(buf.is_empty());
}

#[test]
fn test_strip_iac_sb_se() {
    let mut buf = vec![0xFF, 0xFA, 0x01, 0x02, 0xFF, 0xF0]; // IAC SB ... IAC SE
    inline::strip_iac(&mut buf);
    assert!(buf.is_empty());
}

#[test]
fn test_strip_iac_mid_command() {
    let mut buf = vec![
        b'S', b'E', b'T', 0xFF, 0xFA, 0xFF, 0xF0, b' ', b'k', b'e', b'y',
    ];
    inline::strip_iac(&mut buf);
    assert_eq!(buf, b"SET key");
}

#[test]
fn test_strip_iac_mixed() {
    let mut buf = vec![b'P', 0xFF, 0xFD, 0x01, b'I', 0xFF, 0xFF, b'N', b'G'];
    inline::strip_iac(&mut buf);
    assert_eq!(buf, b"PI\xFFNG"); // IAC DO stripped, IAC IAC -> 0xFF
}

// --- apply_backspace ---

#[test]
fn test_apply_backspace_empty() {
    let mut buf = vec![];
    inline::apply_backspace(&mut buf);
    assert!(buf.is_empty());
}

#[test]
fn test_apply_backspace_no_backspace() {
    let mut buf = b"hello".to_vec();
    inline::apply_backspace(&mut buf);
    assert_eq!(buf, b"hello");
}

#[test]
fn test_apply_backspace_simple() {
    let mut buf = b"AB\x08C".to_vec();
    inline::apply_backspace(&mut buf);
    assert_eq!(buf, b"AC");
}

#[test]
fn test_apply_backspace_at_start() {
    let mut buf = b"\x08ABC".to_vec();
    inline::apply_backspace(&mut buf);
    assert_eq!(buf, b"ABC");
}

#[test]
fn test_apply_backspace_del() {
    let mut buf = b"AB\x7FC".to_vec();
    inline::apply_backspace(&mut buf);
    assert_eq!(buf, b"AC");
}

#[test]
fn test_apply_backspace_multiple() {
    let mut buf = b"ABC\x08\x08D".to_vec();
    inline::apply_backspace(&mut buf);
    assert_eq!(buf, b"AD");
}

#[test]
fn test_apply_backspace_erase_all() {
    let mut buf = b"AB\x08\x08".to_vec();
    inline::apply_backspace(&mut buf);
    assert!(buf.is_empty());
}

#[test]
fn test_apply_backspace_only_control() {
    let mut buf = b"\x08\x08\x7F".to_vec();
    inline::apply_backspace(&mut buf);
    assert!(buf.is_empty());
}

// --- find_line ---

#[test]
fn test_find_line_crlf() {
    assert_eq!(inline::find_line(b"hello\r\n"), Some((5, 2)));
}

#[test]
fn test_find_line_lf() {
    assert_eq!(inline::find_line(b"hello\n"), Some((5, 1)));
}

#[test]
fn test_find_line_crlf_before_lf() {
    // \r\n should be found before bare \n
    assert_eq!(inline::find_line(b"a\r\nb\n"), Some((1, 2)));
}

#[test]
fn test_find_line_no_delim() {
    assert_eq!(inline::find_line(b"hello"), None);
}

#[test]
fn test_find_line_empty() {
    assert_eq!(inline::find_line(b""), None);
}

#[test]
fn test_find_line_multiple_crlf() {
    let (pos, len) = inline::find_line(b"\r\nhello\r\n").unwrap();
    assert_eq!(pos, 0);
    assert_eq!(len, 2);
}

#[test]
fn test_find_line_bare_cr() {
    // bare \r is NOT a delimiter
    assert_eq!(inline::find_line(b"hello\rworld"), None);
}

#[test]
fn test_find_line_lf_at_start() {
    assert_eq!(inline::find_line(b"\nhello"), Some((0, 1)));
}

// --- parse_quoted_args ---

#[test]
fn test_parse_args_basic() {
    let args = inline::parse_quoted_args("SET key val").unwrap();
    assert_eq!(args, vec!["SET", "key", "val"]);
}

#[test]
fn test_parse_args_double_quoted() {
    let args = inline::parse_quoted_args("SET key \"hello world\"").unwrap();
    assert_eq!(args, vec!["SET", "key", "hello world"]);
}

#[test]
fn test_parse_args_single_quoted() {
    let args = inline::parse_quoted_args("SET key 'hello world'").unwrap();
    assert_eq!(args, vec!["SET", "key", "hello world"]);
}

#[test]
fn test_parse_args_escaped_quotes() {
    let args = inline::parse_quoted_args("SET key \"hello \\\"world\\\"\"").unwrap();
    assert_eq!(args, vec!["SET", "key", "hello \"world\""]);
}

#[test]
fn test_parse_args_unclosed_double_quote() {
    let result = inline::parse_quoted_args("SET key \"unclosed");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unclosed quote"));
}

#[test]
fn test_parse_args_empty_quoted_arg() {
    let args = inline::parse_quoted_args("SET \"\" key").unwrap();
    assert_eq!(args, vec!["SET", "", "key"]);
}

#[test]
fn test_parse_args_only_whitespace() {
    let args = inline::parse_quoted_args("   ").unwrap();
    assert!(args.is_empty());
}

#[test]
fn test_parse_args_empty_input() {
    let args = inline::parse_quoted_args("").unwrap();
    assert!(args.is_empty());
}

#[test]
fn test_parse_args_trailing_backslash() {
    let args = inline::parse_quoted_args("SET key\\").unwrap();
    assert_eq!(args, vec!["SET", "key\\"]);
}

#[test]
fn test_parse_args_mixed_quotes() {
    let args = inline::parse_quoted_args("SET \"double\" 'single'").unwrap();
    assert_eq!(args, vec!["SET", "double", "single"]);
}

#[test]
fn test_parse_args_escape_in_single_quote() {
    let args = inline::parse_quoted_args("SET 'hello \\'world\\''").unwrap();
    assert_eq!(args, vec!["SET", "hello 'world'"]);
}

#[test]
fn test_parse_args_multiple_spaces() {
    let args = inline::parse_quoted_args("PING").unwrap();
    assert_eq!(args, vec!["PING"]);
}
