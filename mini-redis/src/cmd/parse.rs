use crate::protocol::resp;

use super::types::{CmdError, ParsedCmd};

impl ParsedCmd {
    /// Parse a command string and arguments into a `ParsedCmd` variant.
    /// Delegates to category-specific parsers in `cmd::parsers::*`.
    pub fn parse(cmd: &str, args: Vec<String>) -> Result<Self, CmdError> {
        // Try each category parser in order. Each returns Err(UnknownCommand)
        // for commands it doesn't handle. Any other error (WrongArgCount,
        // SyntaxError, InvalidInteger) is a real parse failure — propagate it.
        macro_rules! try_parser {
            ($parser:ident) => {
                match super::parsers::$parser::cmd(cmd, args.clone()) {
                    Err(CmdError::UnknownCommand) => {}
                    other => return other,
                }
            };
        }
        try_parser!(strs);
        try_parser!(lists);
        try_parser!(streams);
        try_parser!(hashes);
        try_parser!(sets);
        try_parser!(zsets);
        // Last parser — takes ownership of args to avoid one final clone.
        match super::parsers::admin::cmd(cmd, args) {
            Err(CmdError::UnknownCommand) => Err(CmdError::UnknownCommand),
            other => other,
        }
    }
}

/// Parse a RESP frame into a parsed command.
/// Returns `None` if the frame is not a command array; `Some(Err(..))` for unknown commands
/// or invalid arguments.
pub fn parse_command(frame: &resp::RespType) -> Option<Result<ParsedCmd, CmdError>> {
    if let resp::RespType::Array(Some(items)) = frame {
        let cmd = items.first().and_then(|v| {
            if let resp::RespType::BulkString(Some(bytes)) = v {
                Some(String::from_utf8_lossy(bytes).to_uppercase())
            } else {
                None
            }
        })?;
        let args: Vec<String> = items[1..]
            .iter()
            .filter_map(|v| {
                if let resp::RespType::BulkString(Some(bytes)) = v {
                    Some(String::from_utf8_lossy(bytes).to_string())
                } else {
                    None
                }
            })
            .collect();
        Some(ParsedCmd::parse(&cmd, args))
    } else {
        None
    }
}
