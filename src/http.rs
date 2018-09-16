use std::collections::HashMap;
use std::io;
use crate::aho::aho_tree;

lazy_static! {
    static ref verb_parser: aho_tree<HTTPVerb> = {
        let mut t = aho_tree::new();
        t.insert_rule(b"GET", Some(HTTPVerb::GET));
        t.insert_rule(b"POST", Some(HTTPVerb::POST));
        t.insert_rule(b"PUT", Some(HTTPVerb::PUT));
        t.insert_rule(b"HEAD", Some(HTTPVerb::HEAD));
        t.insert_rule(b"DELETE", Some(HTTPVerb::DELETE));
        t.insert_rule(b"OPTIONS", Some(HTTPVerb::OPTIONS));
        t.insert_rule(b"TRACE", Some(HTTPVerb::TRACE));
        t.insert_rule(b"CONNECT", Some(HTTPVerb::CONNECT));
        t
    };
}

#[derive(Debug, Clone)]
enum HTTPVerb {
    GET,
    POST,
    PUT,
    HEAD,
    DELETE,
    OPTIONS,
    TRACE,
    CONNECT
}

// yes, there are many allocations, deal with it ;)
#[derive(Debug, Clone)]
pub struct http_query {
    verb: HTTPVerb,
    url: String,
    body: String,
    headers: HashMap<String, String>
}

fn get_header_length(arr: &[u8]) -> Result<usize, io::Error> {
    let mut pos = 0;
    while pos < arr.len()-1 {
        if arr[pos] == b'\r' && arr[pos+1] == b'\n' {
            return Ok(pos+2);
        }
        pos+=1;
    }
    Err(io::Error::from(io::ErrorKind::UnexpectedEof))
}

impl http_query {
    pub fn from_string(q: &[u8]) -> Result<Self, io::Error> {
        let len = q.len();
        let mut pos = 0;
        // ignore any CLRF before the Request-Line, per the specification (https://www.w3.org/Protocols/rfc2616/rfc2616-sec4.html)
        while pos < len && (q[pos] == b'\n' || q[pos] == b'\r') {
            pos+=1;
        }

        // match the http verb
        let verb = {
            let mut verb_length = 0;
            while verb_length+pos < len && q[pos+verb_length] != b' ' {
                verb_length+=1;
            }
            let verb = verb_parser.search(&q[pos..pos+verb_length])?;
            pos += verb_length+1;
            verb
        };

        // Let's get the Request-URI size, it's 'req_len-11'
        let req_len = get_header_length(&q[pos..])?;
        // No Request-URI !? Let's drop that packet
        if req_len < 12 {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }

        // let's copy Request-URI if it is a true HTTP Request
        if &q[pos+req_len-11..pos+req_len-2] != b" HTTP/1.1" {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }

        // this better be valid utf8
        let url = String::from_utf8(q[pos..pos+req_len-11].to_vec()).unwrap();
        pos += req_len;

        let mut headers = HashMap::new();
        let mut header_len = get_header_length(&q[pos..])?;
        while header_len != 2 {
            let mut delim = 0;
            while q[pos+delim] != b':' && delim < header_len-2 {
                delim+=1;
            }
            if delim == header_len-2 {
                // invalid header
                return Err(io::Error::from(io::ErrorKind::InvalidData));
            }
            headers.insert(String::from_utf8(q[pos..pos+delim].to_vec()).unwrap(),
                String::from_utf8(q[pos+delim+1..pos+header_len-2].to_vec()).unwrap());

            pos += header_len;
            header_len = get_header_length(&q[pos..])?;
        }
        // do not forget to account for the '\r\n' at the end of headers
        pos += 2;

        // we reached the end of headers, time to copy the body
        let body = String::from_utf8(q[pos..].to_vec()).unwrap();

        Ok(http_query {
            verb: verb.unwrap_or(HTTPVerb::GET),
            url,
            headers,
            body
        })
    }
}
