mod aho;

fn main() {
    let mut t = aho::aho_tree::new();
    t.insert_rule(b"lola", Some(45));
    t.insert_rule(b"lol", Some(46));
    t.insert_rule(b"lolc", Some(65));
    t.insert_rule(b"rotb", Some(75));
    t.insert_rule(b"kj55rotb", Some(85));
    t.insert_rule(b"\0kj55rotb\0", Some(95));
    println!("{:?}", t.search(b"\0kj55rotb\0"));
}
