// KLIK Runtime - Memory Allocator
// Arena-based allocator for fast allocation patterns

use std::alloc::{alloc, dealloc, Layout};
use std::ptr;

/// Arena allocator for fast bump allocation
pub struct Arena {
    chunks: Vec<Chunk>,
    current: usize,
}

struct Chunk {
    ptr: *mut u8,
    layout: Layout,
    offset: usize,
    capacity: usize,
}

impl Arena {
    const DEFAULT_CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks

    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            current: 0,
        }
    }

    pub fn with_capacity(size: usize) -> Self {
        let mut arena = Self::new();
        arena.new_chunk(size);
        arena
    }

    pub fn alloc(&mut self, size: usize, align: usize) -> *mut u8 {
        if self.chunks.is_empty() || !self.chunks[self.current].can_fit(size, align) {
            let chunk_size = std::cmp::max(Self::DEFAULT_CHUNK_SIZE, size + align);
            self.new_chunk(chunk_size);
        }

        self.chunks[self.current].alloc(size, align)
    }

    pub fn alloc_typed<T>(&mut self) -> *mut T {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();
        self.alloc(size, align) as *mut T
    }

    pub fn reset(&mut self) {
        for chunk in &mut self.chunks {
            chunk.offset = 0;
        }
        self.current = 0;
    }

    fn new_chunk(&mut self, size: usize) {
        let layout = Layout::from_size_align(size, 16).unwrap();
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            panic!("out of memory");
        }
        self.chunks.push(Chunk {
            ptr,
            layout,
            offset: 0,
            capacity: size,
        });
        self.current = self.chunks.len() - 1;
    }

    pub fn bytes_allocated(&self) -> usize {
        self.chunks.iter().map(|c| c.offset).sum()
    }

    pub fn bytes_capacity(&self) -> usize {
        self.chunks.iter().map(|c| c.capacity).sum()
    }
}

impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        for chunk in &self.chunks {
            unsafe {
                dealloc(chunk.ptr, chunk.layout);
            }
        }
    }
}

impl Chunk {
    fn can_fit(&self, size: usize, align: usize) -> bool {
        let aligned_offset = align_up(self.offset, align);
        aligned_offset + size <= self.capacity
    }

    fn alloc(&mut self, size: usize, align: usize) -> *mut u8 {
        let aligned_offset = align_up(self.offset, align);
        let result = unsafe { self.ptr.add(aligned_offset) };
        self.offset = aligned_offset + size;
        // Zero the memory
        unsafe {
            ptr::write_bytes(result, 0, size);
        }
        result
    }
}

fn align_up(offset: usize, align: usize) -> usize {
    (offset + align - 1) & !(align - 1)
}

/// Reference-counted allocation for shared ownership
pub struct RcAlloc<T> {
    inner: *mut RcInner<T>,
}

struct RcInner<T> {
    ref_count: usize,
    value: T,
}

impl<T> RcAlloc<T> {
    pub fn new(value: T) -> Self {
        let layout = Layout::new::<RcInner<T>>();
        let inner = unsafe {
            let ptr = alloc(layout) as *mut RcInner<T>;
            ptr::write(
                ptr,
                RcInner {
                    ref_count: 1,
                    value,
                },
            );
            ptr
        };
        Self { inner }
    }

    pub fn get(&self) -> &T {
        unsafe { &(*self.inner).value }
    }

    pub fn ref_count(&self) -> usize {
        unsafe { (*self.inner).ref_count }
    }
}

impl<T> Clone for RcAlloc<T> {
    fn clone(&self) -> Self {
        unsafe {
            (*self.inner).ref_count += 1;
        }
        Self { inner: self.inner }
    }
}

impl<T> Drop for RcAlloc<T> {
    fn drop(&mut self) {
        unsafe {
            (*self.inner).ref_count -= 1;
            if (*self.inner).ref_count == 0 {
                let layout = Layout::new::<RcInner<T>>();
                ptr::drop_in_place(self.inner);
                dealloc(self.inner as *mut u8, layout);
            }
        }
    }
}

// SAFETY: Since KLIK's runtime is single-ownership focused,
// these are safe for our use case
unsafe impl<T: Send> Send for RcAlloc<T> {}
unsafe impl<T: Sync> Sync for RcAlloc<T> {}
