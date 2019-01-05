use nix::sys::mman;
use std::mem;

#[derive(Debug, Clone)]
/// An unsafe block to store an array of elements and provide interior mutability for them.
pub struct BackingStore<T> {
    len: usize,
    data: *mut T
}

pub struct AllocationFailed {}

//unsafe impl<T> Send for BackingStore<T> {}

impl<T> BackingStore<T> {
    pub fn new(len: usize) -> Result<BackingStore<T>, AllocationFailed> {
        let backing_store = unsafe {
            // Map into memory and let backing_store point to it
            // TODO: handle alignment
            match mman::mmap(0 as *mut libc::c_void, len*mem::size_of::<T>(), mman::ProtFlags::PROT_READ | mman::ProtFlags::PROT_WRITE, mman::MapFlags::MAP_SHARED | mman::MapFlags::MAP_ANONYMOUS, -1, 0) {
                Ok(x) => x as *mut T,
                Err(_) => {
                    return Err(AllocationFailed {});
                }
            }
        };
        Ok(BackingStore {
            len,
            data: backing_store
        })
    }

    // Beware of being within bounds, no checks will be done
    pub fn get(&self, pos: usize) -> T {
        let ptr = (self.data as usize + pos * mem::size_of::<T>()) as *mut T;
        unsafe {
            mem::transmute_copy(&*ptr)
        }
    }

    pub fn set(&self, pos: usize, val: T) {
        unsafe {
            *((self.data as usize + pos * mem::size_of::<T>()) as *mut T) = val;
        }
    }
}

impl<T> Drop for BackingStore<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = mman::munmap(self.data as *mut libc::c_void, self.len*mem::size_of::<T>());
        }
    }
}