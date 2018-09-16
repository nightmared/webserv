use std::io::{ErrorKind, Error};
use std::convert::From;

#[derive(Debug)]
pub enum MatchingError {
    StringNotFound
}

impl From<MatchingError> for Error {
    fn from(e: MatchingError) -> Error {
        Error::from(ErrorKind::NotFound)
    }
}

#[derive(Debug)]
pub struct aho_tree<T: Clone> {
    content: Option<u8>,
    children: Vec<aho_tree<T>>,
    value: Option<T>
}

impl<T: Clone> aho_tree<T> {
    pub fn new() -> Self {
        aho_tree {
            content: None,
            children: vec![],
            value: None
        }
    }

    fn search_children(&self, arr: &[u8]) -> Result<Option<T>, MatchingError> {
        for e in self.children.iter() {
            let tmp = e.search(arr);
            if tmp.is_ok() {
                return tmp;
            }
        }
       Err(MatchingError::StringNotFound)
    }

    pub fn search(&self, arr: &[u8]) -> Result<Option<T>, MatchingError> {
        match self.content {
            // root of the tree
            None => self.search_children(arr),
            Some(x) => {
                if x == arr[0] {
                    if arr.len() == 1 {
                        return Ok(self.value.clone());
                    }
                    return self.search_children(&arr[1..]);
                }
                Err(MatchingError::StringNotFound)
            }
        }
    }

    pub fn insert_rule(&mut self, arr: &[u8], val: Option<T>) {
        if arr.len() == 0 {
            // update the value
            self.value = val;
            return;
        }
        for e in self.children.iter_mut() {
            // it should be impossible to construct cases where a non-root node has a 'None' content
            if e.content.unwrap() == arr[0] {
                return e.insert_rule(&arr[1..], val);
            }
        }
        let mut node = aho_tree::new();
        node.content = Some(arr[0]);
        node.insert_rule(&arr[1..], val);
        self.children.push(node);
    }
}
