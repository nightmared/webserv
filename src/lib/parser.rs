use std::marker::PhantomData;

pub trait Parser where Self: Sized {
    /// Consume data until it matches a given pattern.
    fn read_until<'cs>(self, end_pattern: &'cs [u8]) -> Combine<ReaderUntil<'cs>, Self> {
        Combine::new(ReaderUntil {
            end_pattern
        }, self)
    }

    /// Read while the predicate holds true on the data the parser feeds it.
    /// The predicate must return how much data it should consume.
    /// If zero, we stop parsing, otherwise we try consuming data again.
    fn consume_while_predicate(self, predicate: for<'a> fn(&'a [u8]) -> Result<usize, ParserError>) -> Combine<Consumer, Self>  {
        Combine::new(Consumer {
            predicate
        }, self)
    }

    /// Read all the remaining input stream
    fn consume_to_end(self) -> Combine<ConsumerToEnd, Self> {
        Combine::new(ConsumerToEnd {}, self)
    }

    /// Peak `num` bytes.
    fn peek(self, num: usize) -> Combine<Peeker, Self> {
        Combine::new(Peeker {
            peek_number: num
        }, self)
    }
}


pub trait ParserEvaluator<'a> {
    type Output;

    fn evaluate(&'a self, string: &'a [u8], state: &mut ParserState) -> Result<Self::Output, ParserError>;
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
}


/// When we reach an invalid state e rust return as early as possible instead of continuing
/// evaluation, which is the reason theses errors are separated of the other ones in ParserError.
#[derive(Debug)]
pub enum InvalidStateError {
    /// EOF reached while parsing
    EOF
}

#[derive(Debug)]
pub enum ParserError {
    OutOfBoundsAccess,
    InvalidState(InvalidStateError),
    InvalidData,
    Overflow,
    UTFError(std::string::FromUtf8Error)
}

impl std::convert::From<std::string::FromUtf8Error> for ParserError {
    fn from(data: std::string::FromUtf8Error) -> ParserError {
        ParserError::UTFError(data)
    }
}


pub struct Combine<A, B> where A: Parser, B: Parser {
    pa: A,
    pb: B
}

impl<A: Parser, B: Parser> Parser for Combine<A, B> {}
impl<'a, A: Parser+ParserEvaluator<'a>, B: Parser+ParserEvaluator<'a>> ParserEvaluator<'a> for Combine<A, B> {
    type Output = (A::Output, B::Output);

    fn evaluate(&'a self, string: &'a [u8], state: &mut ParserState) -> Result<Self::Output, ParserError> {
        let res_a = self.pa.evaluate(string, state)?;
        let res_b = self.pb.evaluate(string, state)?;
        Ok((res_a, res_b))
    }
}

impl<A: Parser, B: Parser> Combine<A, B> {
    fn new(pa: A, pb: B) -> Self {
        Combine {
            pa,
            pb
        }
    }
}


pub enum OneOf<A, B> {
    First(A),
    Second(B)
}

pub struct TryOr<A, B> where A: Parser, B: Parser {
    pa: A,
    pb: B
}

impl<A: Parser, B: Parser> Parser for TryOr<A, B> {}
impl<'a, A: Parser+ParserEvaluator<'a>, B: Parser+ParserEvaluator<'a>> ParserEvaluator<'a> for TryOr<A, B> {
    type Output = OneOf<A::Output, B::Output>;

    fn evaluate(&'a self, string: &'a [u8], state: &mut ParserState) -> Result<Self::Output, ParserError> {
        match self.pa.evaluate(string, state) {
            Ok(x) => Ok(OneOf::First(x)),
            Err(e) => {
                if let ParserError::InvalidState(_) = e {
                    Err(e)
                } else {
                    Ok(OneOf::Second(self.pb.evaluate(string, state)?))
                }
            }
        }
    }
}

impl<A: Parser, B: Parser> TryOr<A, B> {
    fn new(pa: A, pb: B) -> Self {
        TryOr {
            pa,
            pb
        }
    }
}



pub struct ReaderUntil<'cs> {
    end_pattern: &'cs [u8]
}

impl<'cs> Parser for ReaderUntil<'cs> {}
impl<'a, 'cs> ParserEvaluator<'a> for ReaderUntil<'cs> {
    type Output = &'a [u8];

    fn evaluate(&'a self, string: &'a [u8], state: &mut ParserState) -> Result<Self::Output, ParserError> {
        let old_pos = state.pos;
        let len = string.len();
        while !string[state.pos..].starts_with(self.end_pattern) {
            state.pos += 1;
            if state.pos == len {
                // EOF
                return Ok(&string[old_pos..state.pos]);
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
        let res = state.get_n(string, self.peek_number)?;
        state.pos += self.peek_number;
        Ok(res)
    }
}


pub struct ConsumerToEnd {}

impl Parser for ConsumerToEnd {}
impl<'a> ParserEvaluator<'a> for ConsumerToEnd {
    type Output = &'a [u8];

    fn evaluate(&'a self, string: &'a [u8], state: &mut ParserState) -> Result<Self::Output, ParserError> {
        let res = state.get_n(string, string.len()-state.pos)?;
        state.pos = string.len();
        Ok(res)
    }
}

pub struct Consumer {
    predicate: for<'b> fn(&'b [u8]) -> Result<usize, ParserError>
}

impl Parser for Consumer {}
impl<'a> ParserEvaluator<'a> for Consumer {
    type Output = &'a [u8];

    fn evaluate(&'a self, string: &'a [u8], state: &mut ParserState) -> Result<Self::Output, ParserError> {
        let mut delta = 0;
        loop {
            let offset = (self.predicate)(&string[state.pos+delta..])?;
            if offset == 0 {
                // time to stop parsing
                break;
            }
            delta += offset;
        }
        let res = &string[state.pos..state.pos+delta];
        state.pos += delta;
        Ok(res)
    }
}


/// Return true if the substring is matched, false otherwise
pub struct Match<'cs> {
    pattern: &'cs [u8]
}

impl<'cs> Parser for Match<'cs> {}
impl<'a, 'cs> ParserEvaluator<'a> for Match<'cs> {
    type Output = bool;

    fn evaluate(&'a self, string: &'a [u8], state: &mut ParserState) -> Result<Self::Output, ParserError> {
        if string.len()-state.pos < self.pattern.len() {
            Err(ParserError::InvalidState(InvalidStateError::EOF))
        } else {
            Ok(
                string[state.pos..state.pos+self.pattern.len()]
                    .iter()
                    .zip(self.pattern.iter())
                    .all(|(x, y)| x == y)
            )
        }
    }
}
