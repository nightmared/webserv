mod aho;
mod http;
#[macro_use] extern crate lazy_static;

fn main() {
    http::http_query::from_string(b"\r\n\r\nGET /lol17 HTTP/1.1\r\ntype: lol\r\n\r\nhi, what's up ?");
}
