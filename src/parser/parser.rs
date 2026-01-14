//! PDF Parser
//!
//! Parses PDF tokens into PDF objects.

use crate::error::{Error, Result};
use crate::objects::{
    PdfArray, PdfDict, PdfHexString, PdfName, PdfObject, PdfRef, PdfStream, PdfString,
};

use super::lexer::{Lexer, Token};

/// PDF Parser
///
/// Converts PDF tokens into a tree of PDF objects.
pub struct Parser<'a, 'b> {
    lexer: &'a mut Lexer<'b>,
    /// Lookahead token
    current: Option<Token>,
}

impl<'a, 'b> Parser<'a, 'b> {
    /// Creates a new parser
    pub fn new(lexer: &'a mut Lexer<'b>) -> Self {
        Parser {
            lexer,
            current: None,
        }
    }

    /// Advances to the next token
    fn advance(&mut self) -> Result<Token> {
        let token = if let Some(t) = self.current.take() {
            t
        } else {
            self.lexer.next_token()?
        };
        Ok(token)
    }

    /// Peeks at the current token without consuming it
    fn peek(&mut self) -> Result<&Token> {
        if self.current.is_none() {
            self.current = Some(self.lexer.next_token()?);
        }
        Ok(self.current.as_ref().unwrap())
    }

    /// Parses a single PDF object
    pub fn parse_object(&mut self) -> Result<PdfObject> {
        let token = self.advance()?;

        match token {
            Token::Null => Ok(PdfObject::Null),
            Token::True => Ok(PdfObject::Bool(true)),
            Token::False => Ok(PdfObject::Bool(false)),
            Token::Integer(i) => {
                // Check if this is a reference (num gen R)
                // Object number must be non-negative and fit in u32
                if let Ok(obj_num) = u32::try_from(i) {
                    let saved_current = self.current.take();
                    let saved_pos = self.lexer.position();

                    let reference = 'try_ref: {
                        let Token::Integer(gen) = self.lexer.next_token()? else {
                            break 'try_ref None;
                        };
                        let Ok(gen) = u16::try_from(gen) else {
                            break 'try_ref None;
                        };
                        if !matches!(self.lexer.next_token()?, Token::R) {
                            break 'try_ref None;
                        }
                        Some(PdfObject::Reference(PdfRef::with_generation(obj_num, gen)))
                    };

                    if let Some(r) = reference {
                        return Ok(r);
                    }

                    // Not a reference - restore state
                    self.lexer.set_position(saved_pos);
                    self.current = saved_current;
                }
                Ok(PdfObject::Integer(i))
            }
            Token::Real(r) => Ok(PdfObject::Real(r)),
            Token::Name(name) => Ok(PdfObject::Name(PdfName::new(name))),
            Token::String(bytes) => Ok(PdfObject::String(PdfString::new(bytes))),
            Token::HexString(bytes) => Ok(PdfObject::HexString(PdfHexString::new(bytes))),
            Token::ArrayStart => self.parse_array(),
            Token::DictStart => self.parse_dict_or_stream(),
            Token::Eof => Err(Error::Parse {
                message: "Unexpected end of file".to_string(),
                position: self.lexer.position(),
            }),
            _ => Err(Error::Parse {
                message: format!("Unexpected token: {:?}", token),
                position: self.lexer.position(),
            }),
        }
    }

    /// Parses an array [...]
    fn parse_array(&mut self) -> Result<PdfObject> {
        let mut array = PdfArray::new();

        loop {
            match self.peek()? {
                Token::ArrayEnd => {
                    self.advance()?;
                    break;
                }
                Token::Eof => {
                    return Err(Error::Parse {
                        message: "Unterminated array".to_string(),
                        position: self.lexer.position(),
                    });
                }
                _ => {
                    let obj = self.parse_object()?;
                    array.push(obj);
                }
            }
        }

        Ok(PdfObject::Array(array))
    }

    /// Parses a dictionary <<...>> or stream
    fn parse_dict_or_stream(&mut self) -> Result<PdfObject> {
        let dict = self.parse_dict_contents()?;

        // Check if followed by stream
        if let Ok(Token::StreamStart) = self.peek() {
            self.advance()?;

            // Get stream length
            let length = dict.get_integer("Length").ok_or_else(|| Error::Parse {
                message: "Stream missing Length".to_string(),
                position: self.lexer.position(),
            })?;

            let length = usize::try_from(length).map_err(|_| Error::Parse {
                message: "Stream Length out of range".to_string(),
                position: self.lexer.position(),
            })?;

            let data = self.lexer.read_stream_data(length)?;

            // Expect endstream
            let token = self.advance()?;
            if token != Token::StreamEnd {
                return Err(Error::Parse {
                    message: "Expected endstream".to_string(),
                    position: self.lexer.position(),
                });
            }

            Ok(PdfObject::Stream(PdfStream::new(dict, data)))
        } else {
            Ok(PdfObject::Dict(dict))
        }
    }

    /// Parses dictionary contents
    fn parse_dict_contents(&mut self) -> Result<PdfDict> {
        let mut dict = PdfDict::new();

        loop {
            match self.peek()? {
                Token::DictEnd => {
                    self.advance()?;
                    break;
                }
                Token::Eof => {
                    return Err(Error::Parse {
                        message: "Unterminated dictionary".to_string(),
                        position: self.lexer.position(),
                    });
                }
                Token::Name(_) => {
                    let Token::Name(key) = self.advance()? else {
                        unreachable!()
                    };
                    let value = self.parse_object()?;
                    dict.insert(PdfName::new(key), value);
                }
                _ => {
                    return Err(Error::Parse {
                        message: "Expected name as dictionary key".to_string(),
                        position: self.lexer.position(),
                    });
                }
            }
        }

        Ok(dict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &[u8]) -> Result<PdfObject> {
        let mut lexer = Lexer::new(input);
        let mut parser = Parser::new(&mut lexer);
        parser.parse_object()
    }

    #[test]
    fn test_parse_null() {
        assert_eq!(parse(b"null").unwrap(), PdfObject::Null);
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse(b"true").unwrap(), PdfObject::Bool(true));
        assert_eq!(parse(b"false").unwrap(), PdfObject::Bool(false));
    }

    #[test]
    fn test_parse_integer() {
        assert_eq!(parse(b"42").unwrap(), PdfObject::Integer(42));
        assert_eq!(parse(b"-17").unwrap(), PdfObject::Integer(-17));
    }

    #[test]
    fn test_parse_real() {
        match parse(b"3.14").unwrap() {
            PdfObject::Real(r) => assert!((r - 3.14).abs() < 0.001),
            _ => panic!("Expected real"),
        }
    }

    #[test]
    fn test_parse_name() {
        match parse(b"/Type").unwrap() {
            PdfObject::Name(n) => assert_eq!(n.as_str(), "Type"),
            _ => panic!("Expected name"),
        }
    }

    #[test]
    fn test_parse_string() {
        match parse(b"(Hello World)").unwrap() {
            PdfObject::String(s) => assert_eq!(s.as_bytes(), b"Hello World"),
            _ => panic!("Expected string"),
        }
    }

    #[test]
    fn test_parse_array() {
        match parse(b"[1 2 3]").unwrap() {
            PdfObject::Array(arr) => {
                assert_eq!(arr.len(), 3);
                assert_eq!(arr[0], PdfObject::Integer(1));
                assert_eq!(arr[1], PdfObject::Integer(2));
                assert_eq!(arr[2], PdfObject::Integer(3));
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_parse_dict() {
        match parse(b"<< /Type /Page /Count 5 >>").unwrap() {
            PdfObject::Dict(dict) => {
                assert_eq!(dict.get_type(), Some("Page"));
                assert_eq!(dict.get_integer("Count"), Some(5));
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_parse_reference() {
        match parse(b"5 0 R").unwrap() {
            PdfObject::Reference(r) => {
                assert_eq!(r.object_number(), 5);
                assert_eq!(r.generation(), 0);
            }
            _ => panic!("Expected reference"),
        }
    }

    #[test]
    fn test_parse_nested() {
        let input = b"<< /Type /Page /MediaBox [0 0 612 792] >>";
        match parse(input).unwrap() {
            PdfObject::Dict(dict) => {
                assert_eq!(dict.get_type(), Some("Page"));
                let media_box = dict.get_array("MediaBox").unwrap();
                assert_eq!(media_box.len(), 4);
            }
            _ => panic!("Expected dict"),
        }
    }

    #[test]
    fn test_parse_stream_negative_length() {
        let input = b"<< /Length -1 >>\nstream\nendstream";
        assert!(parse(input).is_err());
    }

    #[test]
    fn test_parse_stream_length_too_long() {
        let input = b"<< /Length 10 >>\nstream\nabc\nendstream";
        assert!(parse(input).is_err());
    }
}
