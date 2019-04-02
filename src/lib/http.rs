use std::collections::HashMap;
use std::str;
use std::mem;

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
    pub url: &'a str,
    // the body remain an array of u8 because it can be binary data
    pub body: &'a [u8],
    pub headers: HashMap<&'a str, &'a str>
}

//impl<'a> HttpQuery<'a> {
//    pub fn from_string(q: &'a [u8]) -> Result<Self, ParserError> {
//        let mut parser = Parser {
//            string: q,
//            pos: 0
//        };
//        // ignore any CLRF before the Request-Line, per the specification (https://www.w3.org/Protocols/rfc2616/rfc2616-sec4.html)
//        parser.advance_while_any(b"\r\n")?;
//
//        // match the http verb
//        let verb = HTTPVerb::parse_from_utf8(parser.get_until(b" ")?).unwrap_or(HTTPVerb::GET);
//
//        // retrieve the queried url
//        let url = unsafe { mem::transmute(str::from_utf8_unchecked(parser.get_until(b" ")?)) };
//
//        // check the request is well formed
//        if parser.get_until(b"\r\n")? != b"HTTP/1.1" {
//            return Err(ParserError::InvalidData);
//        }
//
//        let mut headers = HashMap::new();
//        loop {
//            let header = parser.get_until(b"\r\n")?;
//            if header.len() == 0 {
//                break;
//            }
//
//            let mut pos = 0;
//            for i in 1..header.len()-1 {
//                if header[i] == b':' {
//                    pos = i;
//                    break;
//                }
//            }
//            if pos == 0 {
//                return Err(ParserError::InvalidData);
//            }
//            // yes, this is awfully wrong, but it works ! Besides, we can do less allocations like that.
//            unsafe {
//                headers.insert(mem::transmute(str::from_utf8_unchecked(&header[..pos])), mem::transmute(str::from_utf8_unchecked(&header[pos+1..])));
//            }
//        }
//
//        Ok(HttpQuery {
//            verb,
//            url,
//            headers,
//            body: parser.get_until_eof()
//        })
//    }
//}
