use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{io, thread};
use crate::backingstore::BackingStore;

/// The whole point of this struct is to be able to share it inside an Arc to prevent the sender
/// from being deleted while having a Reader still exists, thus leading to memory unsafety (hereby
/// be dragons !)
#[derive(Debug)]
pub(crate) struct MessageQueueInternal<T> {
    pub len: usize,
    available: AtomicUsize,
    read_ptr: AtomicUsize,
    backing_store: BackingStore<T>
}

// this better work !
unsafe impl<T> Send for MessageQueueInternal<T> { }
unsafe impl<T> Sync for MessageQueueInternal<T> { }

#[derive(Debug)]
pub struct MessageQueueSender<T> {
    internal: Arc<MessageQueueInternal<T>>,
    write_pointer: usize
}

#[derive(Debug, Clone)]
pub struct MessageQueueReader<T> {
    internal: Arc<MessageQueueInternal<T>>
}

#[derive(Debug, PartialEq)]
pub enum MessageQueueError {
    UnvalidSize,
    MemoryAllocationFailed,
    MessageSendingFailed,
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

/// Create a queue.
/// This create a sender object from which you can then create readers.
impl<T: Sized> MessageQueueSender<T> {
    /// Create a new MessageQueueSender object, by specifying the number of elements it must be able to hold.
    /// The size is thus fixed at creation and cannot be changed at runtime.
    pub fn new(num_elements: usize) -> Result<MessageQueueSender<T>, MessageQueueError> {
        if num_elements < 2 {
            return Err(MessageQueueError::UnvalidSize);
        }

        let internal = MessageQueueInternal {
            len: num_elements,
            available: AtomicUsize::new(0),
            read_ptr: AtomicUsize::new(0),
            backing_store: BackingStore::new(num_elements)?
        };

        Ok(MessageQueueSender {
            internal: Arc::new(internal),
            write_pointer: 0
        })
    }

    /// Send a message to the queue
    pub fn send(&mut self, val: T) -> Result<(), MessageQueueError> {
        if self.internal.available.load(Ordering::Acquire) == self.internal.len {
            return Err(MessageQueueError::MessageQueueFull);
        }

        self.internal.backing_store.set(self.write_pointer, val);

        self.write_pointer = (self.write_pointer+1)%self.internal.len;
        self.internal.available.fetch_add(1, Ordering::AcqRel);

        Ok(())
    }

    pub fn new_reader(&mut self) -> MessageQueueReader<T> {
        MessageQueueReader {
            internal: self.internal.clone()
        }
    }
}

impl<T: Sized> MessageQueueReader<T> {
    /// Return number of entries available to read
    pub fn unread(&self) -> usize {
        self.internal.available.load(Ordering::Acquire)
    }

    pub fn is_ready(&self) -> bool {
        self.unread() > 0
    }

    /// Get current value pointed to by the read_pointer and update the read_pointer.
    /// WARNING: this must never *ever* be called when there is no data available to read
    fn get_current_val(&mut self) -> T {
        let rpos = self.internal.read_ptr.load(Ordering::Acquire)%self.internal.len;

        let val = self.internal.backing_store.get(rpos);

        self.internal.available.fetch_sub(1, Ordering::AcqRel);
        self.internal.read_ptr.fetch_add(1, Ordering::AcqRel);
        val
    }

    pub fn read(&mut self) -> Option<T> {
        if self.unread() == 0 {
            None
        } else {
            Some(self.get_current_val())
        }
    }

    pub fn blocking_read(&mut self) -> Option<T> {
        while self.unread() == 0 {
            thread::yield_now();
        }

        Some(self.get_current_val())
    }
}

/// Create a Message queue with a sender and a reader.
/// This is very akin to a ruststd channel.
/// However, the whole reason of this implementation is to be able to listen on its file descriptor
/// using epoll, which was apparently not possible on channels.
pub fn message_queue<T: Clone>(num_elements: usize) -> Result<(MessageQueueSender<T>, MessageQueueReader<T>), MessageQueueError> {
    let mut sender = MessageQueueSender::new(num_elements)?;
    let reader = sender.new_reader();
    Ok((sender, reader))
}
