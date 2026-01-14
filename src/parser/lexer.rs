//! PDF Lexer
//!
//! Tokenizes PDF byte streams into tokens for parsing.

use crate::error::{Error, Result};

/// Token types produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Integer number
    Integer(i64),
    /// Real number
    Real(f64),
    /// Name (without leading /)
    Name(String),
    /// Literal string (contents between parentheses)
    String(Vec<u8>),
    /// Hex string (contents between angle brackets)
    HexString(Vec<u8>),
    /// Boolean true
    True,
    /// Boolean false
    False,
    /// Null value
    Null,
    /// Object reference indicator 'R'
    R,
    /// Start of array '['
    ArrayStart,
    /// End of array ']'
    ArrayEnd,
    /// Start of dictionary '<<'
    DictStart,
    /// End of dictionary '>>'
    DictEnd,
    /// Start of stream
    StreamStart,
    /// End of stream
    StreamEnd,
    /// Object definition start 'obj'
    Obj,
    /// Object definition end 'endobj'
    EndObj,
    /// End of file
    Eof,
}

/// PDF Lexer
///
/// Tokenizes a PDF byte stream into tokens.
pub struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer for the given data
    pub fn new(data: &'a [u8]) -> Self {
        Lexer { data, pos: 0 }
    }

    /// Returns the current position in the input
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Sets the position
    pub fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Returns true if at end of input
    pub fn is_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Peeks at the current byte without advancing
    pub fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Peeks at the byte at the given offset from current position
    pub fn peek_at(&self, offset: usize) -> Option<u8> {
        self.data.get(self.pos + offset).copied()
    }

    /// Advances and returns the current byte
    pub fn advance(&mut self) -> Option<u8> {
        let byte = self.data.get(self.pos).copied();
        if byte.is_some() {
            self.pos += 1;
        }
        byte
    }

    /// Skips whitespace and comments
    pub fn skip_whitespace(&mut self) {
        while let Some(byte) = self.peek() {
            if is_whitespace(byte) {
                self.advance();
            } else if byte == b'%' {
                // Skip comment until end of line
                while let Some(b) = self.advance() {
                    if b == b'\n' || b == b'\r' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Returns the next token
    pub fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace();

        let Some(byte) = self.peek() else {
            return Ok(Token::Eof);
        };

        match byte {
            b'[' => {
                self.advance();
                Ok(Token::ArrayStart)
            }
            b']' => {
                self.advance();
                Ok(Token::ArrayEnd)
            }
            b'<' => {
                self.advance();
                if self.peek() == Some(b'<') {
                    self.advance();
                    Ok(Token::DictStart)
                } else {
                    self.read_hex_string()
                }
            }
            b'>' => {
                self.advance();
                if self.peek() == Some(b'>') {
                    self.advance();
                    Ok(Token::DictEnd)
                } else {
                    Err(Error::Parse {
                        message: "Unexpected '>'".to_string(),
                        position: self.pos,
                    })
                }
            }
            b'(' => self.read_literal_string(),
            b'/' => self.read_name(),
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.read_number(),
            b'a'..=b'z' | b'A'..=b'Z' => self.read_keyword(),
            _ => Err(Error::Parse {
                message: format!("Unexpected byte: {:02X}", byte),
                position: self.pos,
            }),
        }
    }

    /// Reads a literal string (...)
    fn read_literal_string(&mut self) -> Result<Token> {
        self.advance(); // Skip opening '('
        let mut result = Vec::new();
        let mut paren_depth = 1;

        while let Some(byte) = self.advance() {
            match byte {
                b'(' => {
                    paren_depth += 1;
                    result.push(byte);
                }
                b')' => {
                    paren_depth -= 1;
                    if paren_depth == 0 {
                        return Ok(Token::String(result));
                    }
                    result.push(byte);
                }
                b'\\' => {
                    // Handle escape sequences
                    if let Some(escaped) = self.advance() {
                        match escaped {
                            b'n' => result.push(b'\n'),
                            b'r' => result.push(b'\r'),
                            b't' => result.push(b'\t'),
                            b'b' => result.push(0x08),
                            b'f' => result.push(0x0C),
                            b'(' => result.push(b'('),
                            b')' => result.push(b')'),
                            b'\\' => result.push(b'\\'),
                            b'\r' | b'\n' => {
                                // Line continuation - skip
                                if escaped == b'\r' && self.peek() == Some(b'\n') {
                                    self.advance();
                                }
                            }
                            b'0'..=b'7' => {
                                // Octal escape
                                let mut octal = (escaped - b'0') as u32;
                                for _ in 0..2 {
                                    if let Some(b'0'..=b'7') = self.peek() {
                                        octal = octal * 8 + (self.advance().unwrap() - b'0') as u32;
                                    } else {
                                        break;
                                    }
                                }
                                result.push(octal as u8);
                            }
                            _ => result.push(escaped),
                        }
                    }
                }
                _ => result.push(byte),
            }
        }

        Err(Error::Parse {
            message: "Unterminated string".to_string(),
            position: self.pos,
        })
    }

    /// Reads a hex string <...>
    fn read_hex_string(&mut self) -> Result<Token> {
        let mut hex_chars = Vec::new();

        while let Some(byte) = self.advance() {
            match byte {
                b'>' => {
                    // Pad with 0 if odd number of hex digits
                    if hex_chars.len() % 2 != 0 {
                        hex_chars.push(0);
                    }

                    let result: Vec<u8> = hex_chars
                        .chunks(2)
                        .map(|pair| (pair[0] << 4) | pair[1])
                        .collect();

                    return Ok(Token::HexString(result));
                }
                b'0'..=b'9' => hex_chars.push(byte - b'0'),
                b'a'..=b'f' => hex_chars.push(byte - b'a' + 10),
                b'A'..=b'F' => hex_chars.push(byte - b'A' + 10),
                b if is_whitespace(b) => {}
                _ => {
                    return Err(Error::Parse {
                        message: format!("Invalid hex character: {:02X}", byte),
                        position: self.pos,
                    });
                }
            }
        }

        Err(Error::Parse {
            message: "Unterminated hex string".to_string(),
            position: self.pos,
        })
    }

    /// Reads a name /...
    fn read_name(&mut self) -> Result<Token> {
        self.advance(); // Skip leading '/'
        let mut name = String::new();

        while let Some(byte) = self.peek() {
            if is_delimiter(byte) || is_whitespace(byte) {
                break;
            }

            self.advance();

            if byte == b'#' {
                // Hex escape
                let high = self.advance().ok_or_else(|| Error::Parse {
                    message: "Incomplete hex escape in name".to_string(),
                    position: self.pos,
                })?;
                let low = self.advance().ok_or_else(|| Error::Parse {
                    message: "Incomplete hex escape in name".to_string(),
                    position: self.pos,
                })?;

                let value = hex_digit(high)? << 4 | hex_digit(low)?;
                name.push(value as char);
            } else {
                name.push(byte as char);
            }
        }

        Ok(Token::Name(name))
    }

    /// Reads a number (integer or real)
    fn read_number(&mut self) -> Result<Token> {
        let start = self.pos;
        let mut has_dot = false;

        // Handle sign
        if matches!(self.peek(), Some(b'+') | Some(b'-')) {
            self.advance();
        }

        // Read digits
        while let Some(byte) = self.peek() {
            match byte {
                b'0'..=b'9' => {
                    self.advance();
                }
                b'.' if !has_dot => {
                    has_dot = true;
                    self.advance();
                }
                _ => break,
            }
        }

        let s = std::str::from_utf8(&self.data[start..self.pos]).map_err(|_| Error::Parse {
            message: "Invalid number encoding".to_string(),
            position: start,
        })?;

        if has_dot {
            let value: f64 = s.parse().map_err(|_| Error::Parse {
                message: format!("Invalid real number: {}", s),
                position: start,
            })?;
            Ok(Token::Real(value))
        } else {
            let value: i64 = s.parse().map_err(|_| Error::Parse {
                message: format!("Invalid integer: {}", s),
                position: start,
            })?;
            Ok(Token::Integer(value))
        }
    }

    /// Reads a keyword (true, false, null, R, obj, endobj, stream, endstream)
    fn read_keyword(&mut self) -> Result<Token> {
        let start = self.pos;

        while let Some(byte) = self.peek() {
            if is_delimiter(byte) || is_whitespace(byte) {
                break;
            }
            self.advance();
        }

        let keyword =
            std::str::from_utf8(&self.data[start..self.pos]).map_err(|_| Error::Parse {
                message: "Invalid keyword encoding".to_string(),
                position: start,
            })?;

        match keyword {
            "true" => Ok(Token::True),
            "false" => Ok(Token::False),
            "null" => Ok(Token::Null),
            "R" => Ok(Token::R),
            "obj" => Ok(Token::Obj),
            "endobj" => Ok(Token::EndObj),
            "stream" => Ok(Token::StreamStart),
            "endstream" => Ok(Token::StreamEnd),
            _ => Err(Error::Parse {
                message: format!("Unknown keyword: {}", keyword),
                position: start,
            }),
        }
    }

    /// Reads raw bytes for stream content
    pub fn read_stream_data(&mut self, length: usize) -> Result<Vec<u8>> {
        // Skip whitespace after 'stream' keyword
        // The stream keyword must be followed by either CRLF or LF
        if self.peek() == Some(b'\r') {
            self.advance();
        }
        if self.peek() == Some(b'\n') {
            self.advance();
        }

        let end = self.pos.checked_add(length).ok_or_else(|| Error::Parse {
            message: "Stream length overflow".to_string(),
            position: self.pos,
        })?;

        if end > self.data.len() {
            return Err(Error::Parse {
                message: "Stream extends past end of file".to_string(),
                position: self.pos,
            });
        }

        let data = self.data[self.pos..end].to_vec();
        self.pos = end;

        Ok(data)
    }
}

/// Returns true if the byte is a PDF whitespace character
fn is_whitespace(byte: u8) -> bool {
    matches!(byte, 0 | 9 | 10 | 12 | 13 | 32)
}

/// Returns true if the byte is a PDF delimiter
fn is_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

/// Converts a hex digit to its value
fn hex_digit(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(Error::Parse {
            message: format!("Invalid hex digit: {:02X}", byte),
            position: 0,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_integer() {
        let mut lexer = Lexer::new(b"42");
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(42));
    }

    #[test]
    fn test_lexer_real() {
        let mut lexer = Lexer::new(b"3.14");
        assert_eq!(lexer.next_token().unwrap(), Token::Real(3.14));
    }

    #[test]
    fn test_lexer_name() {
        let mut lexer = Lexer::new(b"/Type");
        assert_eq!(lexer.next_token().unwrap(), Token::Name("Type".to_string()));
    }

    #[test]
    fn test_lexer_string() {
        let mut lexer = Lexer::new(b"(Hello World)");
        assert_eq!(
            lexer.next_token().unwrap(),
            Token::String(b"Hello World".to_vec())
        );
    }

    #[test]
    fn test_lexer_hex_string() {
        let mut lexer = Lexer::new(b"<48656C6C6F>");
        assert_eq!(
            lexer.next_token().unwrap(),
            Token::HexString(b"Hello".to_vec())
        );
    }

    #[test]
    fn test_lexer_array() {
        let mut lexer = Lexer::new(b"[1 2 3]");
        assert_eq!(lexer.next_token().unwrap(), Token::ArrayStart);
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(1));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(2));
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(3));
        assert_eq!(lexer.next_token().unwrap(), Token::ArrayEnd);
    }

    #[test]
    fn test_lexer_dict() {
        let mut lexer = Lexer::new(b"<< /Type /Page >>");
        assert_eq!(lexer.next_token().unwrap(), Token::DictStart);
        assert_eq!(lexer.next_token().unwrap(), Token::Name("Type".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::Name("Page".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::DictEnd);
    }

    #[test]
    fn test_lexer_keywords() {
        let mut lexer = Lexer::new(b"true false null");
        assert_eq!(lexer.next_token().unwrap(), Token::True);
        assert_eq!(lexer.next_token().unwrap(), Token::False);
        assert_eq!(lexer.next_token().unwrap(), Token::Null);
    }
}
