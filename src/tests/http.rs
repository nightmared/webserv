use test::Bencher;
use std::str;
use crate::http;
use rand::{Rng, RngCore};

static BASE_QUERY: &'static str = "\r\n\r\nGET /lol17 HTTP/1.1\r\ntype: lol\r\n\r\n";

#[bench]
fn bench_http_parsing(b: &mut Bencher) {
    let req = format!("{}Hi, what's up ?", BASE_QUERY);
    b.iter(|| {
        http::HttpQuery::from_string(req.as_bytes()).unwrap();
    });
}

// generate num random headers
fn generate_headers(num: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut headers = Vec::new();
    for _ in 0..num {
        let entry = format!("{}: {}", rng.gen::<i64>(), rng.gen::<i128>());
        headers.extend_from_slice(entry.as_bytes());
        headers.extend_from_slice(b"\r\n");
    }
    headers
}

fn generate_long_http_query(headers_num: usize, garbage_size: usize) -> Vec<u8> {
    let mut req = b"GET /lol17 HTTP/1.1\r\ntype: lol\r\n".to_vec();
    req.extend_from_slice(&generate_headers(headers_num));
    req.extend_from_slice(b"\r\n");
    let mut buf = Vec::with_capacity(garbage_size);
    buf.resize(garbage_size, 0);
    rand::thread_rng().fill_bytes(&mut buf);
    req.extend_from_slice(&buf);
    req
}

#[bench]
fn bench_http_parsing_long_100_8192(b: &mut Bencher) {
    let req = generate_long_http_query(100, 8192);

    b.iter(|| {
        http::HttpQuery::from_string(&req).unwrap();
    });
}

#[bench]
fn bench_http_parsing_long_500_4096(b: &mut Bencher) {
    let req = generate_long_http_query(500, 4096);

    b.iter(|| {
        http::HttpQuery::from_string(&req).unwrap();
    });
}

#[bench]
fn bench_http_parsing_long_25000_65536(b: &mut Bencher) {
    let req = generate_long_http_query(25000, 65536);

    b.iter(|| {
        http::HttpQuery::from_string(&req).unwrap();
    });
}