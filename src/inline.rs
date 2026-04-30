/// Strip Telnet IAC (Interpret As Command, 0xFF) negotiation bytes from the buffer.
/// In-place compaction: modifies buf in place, then truncates.
pub fn strip_iac(buf: &mut Vec<u8>) {
    let mut write = 0;
    let mut i = 0;
    while i < buf.len() {
        if buf[i] == 0xFF {
            if i + 1 >= buf.len() {
                break; // truncated IAC at end of buffer
            }
            match buf[i + 1] {
                0xFF => {
                    // IAC IAC (escaped literal 0xFF): emit one 0xFF
                    buf[write] = 0xFF;
                    write += 1;
                    i += 2;
                }
                0xFA => {
                    // IAC SB (subnegotiation): scan for IAC SE
                    if let Some(end) = buf[i + 2..].windows(2).position(|w| w == [0xFF, 0xF0]) {
                        i += 2 + end + 2; // skip IAC SB ... IAC SE
                    } else {
                        i = buf.len(); // unterminated subnegotiation: skip rest
                    }
                }
                0xFB | 0xFC | 0xFD | 0xFE => {
                    // WILL/WONT/DO/DONT: IAC <cmd> <option> (3 bytes)
                    if i + 2 >= buf.len() {
                        break;
                    }
                    i += 3;
                }
                0xF0 => {
                    // IAC SE without SB — skip (malformed)
                    i += 2;
                }
                _ => {
                    // Other 2-byte IAC commands (NOP, DM, BRK, IP, AO, AYT, EC, EL, GA)
                    i += 2;
                }
            }
        } else {
            buf[write] = buf[i];
            write += 1;
            i += 1;
        }
    }
    buf.truncate(write);
}

/// Process backspace (0x08) and DEL (0x7F) characters by removing the preceding byte.
/// In-place compaction: modifies buf in place, then truncates.
pub fn apply_backspace(buf: &mut Vec<u8>) {
    let mut write = 0;
    let mut i = 0;
    while i < buf.len() {
        if buf[i] == 0x08 || buf[i] == 0x7F {
            if write > 0 {
                write -= 1; // erase previous character
            }
            i += 1; // drop the control byte
        } else {
            buf[write] = buf[i];
            write += 1;
            i += 1;
        }
    }
    buf.truncate(write);
}

/// Find the next line delimiter in the buffer.
/// Returns `(pos, delim_len)` where `pos` is the start of the delimiter
/// and `delim_len` is 2 for `\r\n` or 1 for bare `\n`.
/// Returns `None` if no delimiter is found.
pub fn find_line(buf: &[u8]) -> Option<(usize, usize)> {
    // Priority: \r\n
    if let Some(pos) = buf.windows(2).position(|w| w == b"\r\n") {
        return Some((pos, 2));
    }
    // Fallback: bare \n
    if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        return Some((pos, 1));
    }
    None
}

/// Parse command-line arguments with support for quoted strings.
/// Supports double quotes (`"..."`) and single quotes (`'...'`) with backslash escaping.
/// Returns an error if quotes are unclosed.
pub fn parse_quoted_args(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars();

    #[derive(PartialEq)]
    enum Quote {
        None,
        Single,
        Double,
    }
    let mut quote = Quote::None;
    let mut escape = false;

    while let Some(c) = chars.next() {
        if escape {
            current.push(c);
            escape = false;
            continue;
        }

        if quote == Quote::None {
            match c {
                c if c.is_ascii_whitespace() => {
                    if !current.is_empty() {
                        args.push(std::mem::take(&mut current));
                    }
                }
                '"' => quote = Quote::Double,
                '\'' => quote = Quote::Single,
                '\\' => escape = true,
                _ => current.push(c),
            }
        } else if quote == Quote::Double {
            match c {
                '"' => {
                    quote = Quote::None;
                    // Empty quoted string: push empty arg immediately
                    if current.is_empty() {
                        args.push(String::new());
                    }
                }
                '\\' => escape = true,
                _ => current.push(c),
            }
        } else if quote == Quote::Single {
            match c {
                '\'' => {
                    quote = Quote::None;
                    if current.is_empty() {
                        args.push(String::new());
                    }
                }
                '\\' => escape = true,
                _ => current.push(c),
            }
        }
    }

    // Trailing backslash — treat as literal
    if escape {
        current.push('\\');
    }

    if quote != Quote::None {
        return Err("ERR unclosed quote".to_string());
    }

    // Push the last token if non-empty
    if !current.is_empty() {
        args.push(current);
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- strip_iac ---

    #[test]
    fn test_strip_iac_empty() {
        let mut buf = vec![];
        strip_iac(&mut buf);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_strip_iac_no_iac() {
        let mut buf = b"hello world".to_vec();
        strip_iac(&mut buf);
        assert_eq!(buf, b"hello world");
    }

    #[test]
    fn test_strip_iac_double_ff() {
        let mut buf = vec![0xFF, 0xFF];
        strip_iac(&mut buf);
        assert_eq!(buf, vec![0xFF]); // IAC IAC -> literal 0xFF
    }

    #[test]
    fn test_strip_iac_will() {
        let mut buf = vec![0xFF, 0xFB, 0x03]; // IAC WILL 3
        strip_iac(&mut buf);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_strip_iac_dont() {
        let mut buf = vec![0xFF, 0xFE, 0x18]; // IAC DONT 24
        strip_iac(&mut buf);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_strip_iac_sb_se() {
        let mut buf = vec![0xFF, 0xFA, 0x01, 0x02, 0xFF, 0xF0]; // IAC SB ... IAC SE
        strip_iac(&mut buf);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_strip_iac_mid_command() {
        let mut buf = vec![
            b'S', b'E', b'T', 0xFF, 0xFA, 0xFF, 0xF0, b' ', b'k', b'e', b'y',
        ];
        strip_iac(&mut buf);
        assert_eq!(buf, b"SET key");
    }

    #[test]
    fn test_strip_iac_mixed() {
        let mut buf = vec![b'P', 0xFF, 0xFD, 0x01, b'I', 0xFF, 0xFF, b'N', b'G'];
        strip_iac(&mut buf);
        assert_eq!(buf, b"PI\xFFNG"); // IAC DO stripped, IAC IAC -> 0xFF
    }

    // --- apply_backspace ---

    #[test]
    fn test_apply_backspace_empty() {
        let mut buf = vec![];
        apply_backspace(&mut buf);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_apply_backspace_no_backspace() {
        let mut buf = b"hello".to_vec();
        apply_backspace(&mut buf);
        assert_eq!(buf, b"hello");
    }

    #[test]
    fn test_apply_backspace_simple() {
        let mut buf = b"AB\x08C".to_vec();
        apply_backspace(&mut buf);
        assert_eq!(buf, b"AC");
    }

    #[test]
    fn test_apply_backspace_at_start() {
        let mut buf = b"\x08ABC".to_vec();
        apply_backspace(&mut buf);
        assert_eq!(buf, b"ABC");
    }

    #[test]
    fn test_apply_backspace_del() {
        let mut buf = b"AB\x7FC".to_vec();
        apply_backspace(&mut buf);
        assert_eq!(buf, b"AC");
    }

    #[test]
    fn test_apply_backspace_multiple() {
        let mut buf = b"ABC\x08\x08D".to_vec();
        apply_backspace(&mut buf);
        assert_eq!(buf, b"AD");
    }

    #[test]
    fn test_apply_backspace_erase_all() {
        let mut buf = b"AB\x08\x08".to_vec();
        apply_backspace(&mut buf);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_apply_backspace_only_control() {
        let mut buf = b"\x08\x08\x7F".to_vec();
        apply_backspace(&mut buf);
        assert!(buf.is_empty());
    }

    // --- find_line ---

    #[test]
    fn test_find_line_crlf() {
        assert_eq!(find_line(b"hello\r\n"), Some((5, 2)));
    }

    #[test]
    fn test_find_line_lf() {
        assert_eq!(find_line(b"hello\n"), Some((5, 1)));
    }

    #[test]
    fn test_find_line_crlf_before_lf() {
        // \r\n should be found before bare \n
        assert_eq!(find_line(b"a\r\nb\n"), Some((1, 2)));
    }

    #[test]
    fn test_find_line_no_delim() {
        assert_eq!(find_line(b"hello"), None);
    }

    #[test]
    fn test_find_line_empty() {
        assert_eq!(find_line(b""), None);
    }

    #[test]
    fn test_find_line_multiple_crlf() {
        let (pos, len) = find_line(b"\r\nhello\r\n").unwrap();
        assert_eq!(pos, 0);
        assert_eq!(len, 2);
    }

    #[test]
    fn test_find_line_bare_cr() {
        // bare \r is NOT a delimiter
        assert_eq!(find_line(b"hello\rworld"), None);
    }

    #[test]
    fn test_find_line_lf_at_start() {
        assert_eq!(find_line(b"\nhello"), Some((0, 1)));
    }

    // --- parse_quoted_args ---

    #[test]
    fn test_parse_args_basic() {
        let args = parse_quoted_args("SET key val").unwrap();
        assert_eq!(args, vec!["SET", "key", "val"]);
    }

    #[test]
    fn test_parse_args_double_quoted() {
        let args = parse_quoted_args("SET key \"hello world\"").unwrap();
        assert_eq!(args, vec!["SET", "key", "hello world"]);
    }

    #[test]
    fn test_parse_args_single_quoted() {
        let args = parse_quoted_args("SET key 'hello world'").unwrap();
        assert_eq!(args, vec!["SET", "key", "hello world"]);
    }

    #[test]
    fn test_parse_args_escaped_quotes() {
        let args = parse_quoted_args("SET key \"hello \\\"world\\\"\"").unwrap();
        assert_eq!(args, vec!["SET", "key", "hello \"world\""]);
    }

    #[test]
    fn test_parse_args_unclosed_double_quote() {
        let result = parse_quoted_args("SET key \"unclosed");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unclosed quote"));
    }

    #[test]
    fn test_parse_args_empty_quoted_arg() {
        let args = parse_quoted_args("SET \"\" key").unwrap();
        assert_eq!(args, vec!["SET", "", "key"]);
    }

    #[test]
    fn test_parse_args_only_whitespace() {
        let args = parse_quoted_args("   ").unwrap();
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_args_empty_input() {
        let args = parse_quoted_args("").unwrap();
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_args_trailing_backslash() {
        let args = parse_quoted_args("SET key\\").unwrap();
        assert_eq!(args, vec!["SET", "key\\"]);
    }

    #[test]
    fn test_parse_args_mixed_quotes() {
        let args = parse_quoted_args("SET \"double\" 'single'").unwrap();
        assert_eq!(args, vec!["SET", "double", "single"]);
    }

    #[test]
    fn test_parse_args_escape_in_single_quote() {
        let args = parse_quoted_args("SET 'hello \\'world\\''").unwrap();
        assert_eq!(args, vec!["SET", "hello 'world'"]);
    }

    #[test]
    fn test_parse_args_multiple_spaces() {
        let args = parse_quoted_args("PING").unwrap();
        assert_eq!(args, vec!["PING"]);
    }
}
