use std::collections::HashMap;
//use std::io;

#[derive(Debug, Clone)]
pub enum HTTPVerb {
    GET,
    POST,
    PUT,
    HEAD,
    DELETE,
    OPTIONS,
    TRACE,
    CONNECT
}

impl HTTPVerb {
    fn parse_from_utf8(verb: &[u8]) -> Option<Self> {
        match verb {
            b"GET" => Some(HTTPVerb::GET),
            b"POST" => Some(HTTPVerb::POST),
            b"PUT" => Some(HTTPVerb::PUT),
            b"HEAD" => Some(HTTPVerb::HEAD),
            b"DELETE" => Some(HTTPVerb::DELETE),
            b"OPTIONS" => Some(HTTPVerb::OPTIONS),
            b"TRACE" => Some(HTTPVerb::TRACE),
            b"CONNECT" => Some(HTTPVerb::CONNECT),
            _ => None
        }
    }
}

// yes, there are many allocations, deal with it ;)
#[derive(Debug, Clone)]
pub struct HttpQuery<'a> {
    pub verb: HTTPVerb,
    pub url: String,
    pub body: &'a [u8],
    pub headers: HashMap<String, String>
}

struct Parser<'a> {
    string: &'a [u8],
    pos: usize
}

#[derive(Debug)]
pub enum ParserError {
    /// EOF reached while parsing
    EOF,
    InvalidData,
    //IOError(io::Error),
    UTFError(std::string::FromUtf8Error)
}

impl std::convert::From<std::string::FromUtf8Error> for ParserError {
    fn from(data: std::string::FromUtf8Error) -> ParserError {
        ParserError::UTFError(data)
    }
}

impl<'a> Parser<'a> {
    /// Advance the parser while any sequences of the characters in 'cmp' can be matched
    fn advance_until_any(&mut self, cmp: &[u8]) -> Result<(), ParserError> {
        let len = self.string.len();
        while self.pos != len && cmp.contains(&self.string[self.pos]) {
            self.pos += 1;
            if self.pos == len {
                return Err(ParserError::EOF);
            }
        }
        Ok(())
    }

    /// Return the chain of character pointed to by the parser until the string 'cmp' match.
    /// This will advance the parser past the end of the matching 'cmp' substring.
    fn get_until(&mut self, cmp: &[u8]) -> Result<&[u8], ParserError> {
        let old_pos = self.pos;
        let len = self.string.len();
        while !self.string[self.pos..].starts_with(cmp) {
            self.pos += 1;
            if self.pos == len {
                return Err(ParserError::EOF);
            }
        }

        let res = &self.string[old_pos..self.pos];
        self.pos += cmp.len();
        Ok(res)
    }

    fn get_until_eof(self) -> &'a[u8] {
        &self.string[self.pos..]
    }
}

impl<'a> HttpQuery<'a> {
    pub fn from_string(q: &'a [u8]) -> Result<Self, ParserError> {
        let mut parser = Parser {
            string: q,
            pos: 0
        };
        // ignore any CLRF before the Request-Line, per the specification (https://www.w3.org/Protocols/rfc2616/rfc2616-sec4.html)
        parser.advance_until_any(b"\r\n")?;

        // match the http verb
        let verb = HTTPVerb::parse_from_utf8(parser.get_until(b" ")?).unwrap_or(HTTPVerb::GET);

        // retrieve the queried url
        let url = String::from_utf8(parser.get_until(b" ")?.to_vec())?;

        // check the request is well formed
        if parser.get_until(b"\r\n")? != b"HTTP/1.1" {
            return Err(ParserError::InvalidData);
        }

        let mut headers = HashMap::new();
        loop {
            let header = parser.get_until(b"\r\n")?;
            if header.len() == 0 {
                break;
            }

            match header.iter().enumerate().filter(|x| *x.1 == b':').next() {
                Some((pos, _)) => {
                    let (key, val) = header.split_at(pos);
                    headers.insert(String::from_utf8(key.to_vec())?, String::from_utf8(val.to_vec())?);
                },
                None => return Err(ParserError::InvalidData)
            };
        }

        Ok(HttpQuery {
            verb,
            url,
            headers,
            body: parser.get_until_eof()
        })
    }
}
