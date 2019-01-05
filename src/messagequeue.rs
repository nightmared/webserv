use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{io, thread};
use std::time::Duration;
use crate::backingstore::BackingStore;

/// The whole point of this struct is to be able to share it inside an Arc to prevent the sender
/// from being deleted while having a Reader still exists, thus leading to memory unsafety (hereby
/// be dragons !)
#[derive(Debug)]
pub(crate) struct MessageQueueInternal<T> {
    pub len: usize,
    write_ptr: AtomicUsize,
    read_ptr: AtomicUsize,
    backing_store: BackingStore<T>
}

// this better work !
unsafe impl<T> Send for MessageQueueInternal<T> { }
unsafe impl<T> Sync for MessageQueueInternal<T> { }

#[derive(Debug)]
pub struct MessageQueueSender<T> {
    internal: Arc<MessageQueueInternal<T>>
}

#[derive(Debug, Clone)]
pub struct MessageQueueReader<T> {
    internal: Arc<MessageQueueInternal<T>>
}

#[derive(Debug, PartialEq)]
pub enum MessageQueueError {
    UnvalidSize,
    MemoryAllocationFailed,
    MessageQueueFull,
    MessageQueueEmpty,
    NixError(nix::Error)
}

impl From<nix::Error> for MessageQueueError {
    fn from(e: nix::Error) -> Self {
        MessageQueueError::NixError(e)
    }
}

impl From<crate::backingstore::AllocationFailed> for MessageQueueError {
    fn from(_: crate::backingstore::AllocationFailed) -> Self {
        MessageQueueError::MemoryAllocationFailed
    }
}

impl From<MessageQueueError> for io::Error {
    fn from(_: MessageQueueError) -> Self {
        io::Error::new(io::ErrorKind::Other, "MessageQueueError")
    }
}

impl<T> MessageQueueInternal<T> {
    /// Returns the distance between the reader and the writer on the data ring
    /// aka. the number of entries available to read
    pub fn dist(&self) -> usize {
        let writer_pos = self.write_ptr.load(Ordering::Acquire);
        let reader_pos = self.read_ptr.load(Ordering::Acquire);
        if writer_pos < reader_pos {
            self.len+writer_pos-reader_pos
        } else {
            writer_pos - reader_pos
        }
    }
}

/// Create a queue.
/// This create a sender object from which you can then create readers.
impl<T: Sized> MessageQueueSender<T> {
    /// Create a new MessageQueueSender object, by specifying the number of elements 
    /// it must be able to hold.
    /// The size is thus fixed at creation and cannot be changed at runtime.
    pub fn new(num_elements: usize) -> Result<MessageQueueSender<T>, MessageQueueError> {
        if num_elements < 2 {
            return Err(MessageQueueError::UnvalidSize);
        }

        let internal = MessageQueueInternal {
            len: num_elements,
            write_ptr: AtomicUsize::new(0),
            read_ptr: AtomicUsize::new(0),
            backing_store: BackingStore::new(num_elements)?
        };

        Ok(MessageQueueSender {
            internal: Arc::new(internal)
        })
    }

    /// Send a message to the queue
    pub fn send(&mut self, val: T) -> Result<(), MessageQueueError> {
        if self.internal.dist() == self.internal.len-1 {
            return Err(MessageQueueError::MessageQueueFull);
        }

        let wptr = self.internal.write_ptr.load(Ordering::Relaxed);
        self.internal.backing_store.set(wptr, val);

        self.internal.write_ptr.store((wptr+1)%self.internal.len, Ordering::Release);

        Ok(())
    }

    pub fn new_reader(&mut self) -> MessageQueueReader<T> {
        MessageQueueReader {
            internal: self.internal.clone()
        }
    }
}

impl<T: Sized> MessageQueueReader<T> {
    pub fn available(&self) -> usize {
        self.internal.dist()
    }

    pub fn is_ready(&self) -> bool {
        self.internal.dist() > 0
    }

    /// Get current value pointed to by the read_pointer and update the read_pointer.
    /// WARNING: this must never *ever* be called when there is no data available to read
    fn get_current_val(&mut self) -> T {
        let rpos = self.internal.read_ptr.load(Ordering::Acquire);

        let val = self.internal.backing_store.get(rpos);

        self.internal.read_ptr.store((rpos+1)%self.internal.len, Ordering::Release);
        val
    }

    pub fn read(&mut self) -> Option<T> {
        if self.is_ready() {
            Some(self.get_current_val())
        } else {
            None
        }
    }

    pub fn blocking_read(&mut self) -> Option<T> {
        // backing off algorithm
        for _ in 0..50 {
            if let Some(x) = self.read() {
                return Some(x);
            }
        }
        let mut count = 0;
        loop {
            let dur = match count {
                0..10 => 35,
                10..100 => 80,
                100..500 => 250,
                _ => 500
            };
            thread::sleep(Duration::from_micros(dur));
            if let Some(x) = self.read() {
                return Some(x);
            }
            count += 1;
        }
    }
}

/// Create a Message queue with a sender and a reader.
/// This is very akin to a ruststd channel.
pub fn message_queue<T: Clone>(num_elements: usize) -> Result<(MessageQueueSender<T>, MessageQueueReader<T>), MessageQueueError> {
    let mut sender = MessageQueueSender::new(num_elements)?;
    let reader = sender.new_reader();
    Ok((sender, reader))
}
