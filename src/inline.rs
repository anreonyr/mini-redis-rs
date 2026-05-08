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

