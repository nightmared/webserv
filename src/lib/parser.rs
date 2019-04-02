use std::marker::PhantomData;

pub trait Parser where Self: Sized {
    /// Consume data until it matches a given pattern.
    fn consume_until<'cs>(self, end_pattern: &'cs [u8]) -> CombineParser<Consumer<'cs>, Self> {
        CombineParser::new(Consumer {
            end_pattern
        }, self)
    }

    /// Peak `num` bytes.
    fn peek(self, num: usize) -> CombineParser<Peeker, Self> {
        CombineParser::new(Peeker {
            peek_number: num
        }, self)
    }
}

pub struct ParserState {
    pos: usize
}

impl ParserState {
    fn index(&self, string: &[u8], index: usize) -> Result<u8, ParserError> {
        if index <= self.pos {
            Err(ParserError::OutOfBoundsAccess)
        } else {
            Ok(string[self.pos])
        }
    }

    fn index_size<'a>(&self, string: &'a [u8], start: usize, size: usize) -> Result<&'a [u8], ParserError> {
        let end = start.checked_add(size);
        match end {
            None => Err(ParserError::Overflow),
            Some(end) => {
                if end > string.len() {
                    Err(ParserError::OutOfBoundsAccess)
                } else {
                    Ok(&string[self.pos..end])
                }
            }
        }
    }

    fn get_current(&self, string: &[u8]) -> Result<u8, ParserError> {
        self.index(string, self.pos)
    }

    fn get_n<'a>(&self, string: &'a [u8], size: usize) -> Result<&'a [u8], ParserError> {
        self.index_size(string, self.pos, size)
    }

    fn check_get_n(&self, string: &[u8], size: usize) -> Result<(), ParserError> {
        let end = self.pos.checked_add(size);
        match end {
            None => Err(ParserError::Overflow),
            Some(end) => {
                if end > string.len() {
                    Err(ParserError::OutOfBoundsAccess)
                } else {
                    Ok(())
                }
            }
        }
    }
}

pub trait ParserEvaluator<'a> {
    type Output;

    fn evaluate(&'a self, string: &'a [u8], state: &mut ParserState) -> Result<Self::Output, ParserError>;
}


pub struct CombineParser<A, B> where A: Parser, B: Parser {
    pa: A,
    pb: B
}

impl<A: Parser, B: Parser> Parser for CombineParser<A, B> {}
impl<'a, A: Parser+ParserEvaluator<'a>, B: Parser+ParserEvaluator<'a>> ParserEvaluator<'a> for CombineParser<A, B> {
    type Output = (A::Output, B::Output);

    fn evaluate(&'a self, string: &'a [u8], state: &mut ParserState) -> Result<Self::Output, ParserError> {
        let res_a = self.pa.evaluate(string, state)?;
        let res_b = self.pb.evaluate(string, state)?;
        Ok((res_a, res_b))
    }
}

impl<A: Parser, B: Parser> CombineParser<A, B> {
    fn new(pa: A, pb: B) -> Self {
        CombineParser {
            pa,
            pb
        }
    }
}

pub struct Consumer<'cs> {
    end_pattern: &'cs [u8]
}

impl<'cs> Parser for Consumer<'cs> {}
impl<'a, 'cs> ParserEvaluator<'a> for Consumer<'cs> {
    type Output = &'a [u8];

    fn evaluate(&'a self, string: &'a [u8], state: &mut ParserState) -> Result<Self::Output, ParserError> {
        let old_pos = state.pos;
        let len = string.len();
        while !string[state.pos..].starts_with(self.end_pattern) {
            state.pos += 1;
            if state.pos == len {
                return Err(ParserError::EOF);
            }
        }

        Ok(&string[old_pos..state.pos])
    }
}

pub struct Peeker {
    peek_number: usize
}

impl Parser for Peeker {}
impl<'a> ParserEvaluator<'a> for Peeker {
    type Output = &'a [u8];

    fn evaluate(&'a self, string: &'a [u8], state: &mut ParserState) -> Result<Self::Output, ParserError> {
        state.check_get_n(string, self.peek_number)?;
        state.pos += self.peek_number;
        Ok(state.get_n(string, self.peek_number)?)
    }
}

#[derive(Debug)]
pub enum ParserError {
    /// EOF reached while parsing
    EOF,
    InvalidData,
    OutOfBoundsAccess,
    Overflow,
    UTFError(std::string::FromUtf8Error)
}

impl std::convert::From<std::string::FromUtf8Error> for ParserError {
    fn from(data: std::string::FromUtf8Error) -> ParserError {
        ParserError::UTFError(data)
    }
}

//impl<'s> InternalParser<'s> {
//    pub fn new(txt: &'s str) -> Self {
//        InternalParser {
//            string: txt.as_bytes(),
//            pos: 0
//        }
//    }
//
//    /// Advnce the parser while the predicate `F` holds true.
//    fn advance_predicate<F>(&mut self, fun: F) -> Result<&[u8], ParserError> 
//    where F: Fn(&[u8]) -> bool {
//        let len = self.string.len();
//        while self.pos != len && fun(&self.string[self.pos..]) {
//            self.pos += 1;
//            if self.pos == len {
//                return Err(ParserError::EOF);
//            }
//        }
//        Ok(&self.string[self.pos..])
//    }
//
//    /// Advance the parser while any sequences of the characters in 'cmp' can be matched.
//    fn advance_while_any(&mut self, cmp: &[u8]) -> Result<(), ParserError> {
//        self.advance_predicate(|txt| cmp.contains(&txt[0])).map(|_| ())
//    }
//
//    /// Return the chain of character pointed to by the parser until the string 'cmp' match.
//    /// This will advance the parser past the end of the matching 'cmp' substring.
//    fn get_until(&mut self, cmp: &[u8]) -> Result<&[u8], ParserError> {
//        self.advance_predicate(|txt| !txt.starts_with(cmp))
//    }
//
//    fn get_until_eof(self) -> &'s[u8] {
//        &self.string[self.pos..]
//    }
//}
