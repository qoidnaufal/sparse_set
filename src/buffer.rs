use std::alloc;
use std::ptr::NonNull;
use std::marker::PhantomData;

use crate::error::Error;

/// The underlying buffer which will manage grow, push, dealloc, etc.
pub(crate) struct RawBuffer<T> {
    pub(crate) ptr: NonNull<T>,
    pub(crate) capacity: usize,
    marker: PhantomData<T>,
}

impl<T> RawBuffer<T> {
    const LAYOUT: alloc::Layout = alloc::Layout::new::<T>();

    pub(crate) const fn new() -> Self {
        Self {
            ptr: NonNull::dangling(),
            capacity: if Self::LAYOUT.size() == 0 { usize::MAX } else { 0 },
            marker: PhantomData,
        }
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        let mut this = Self::new();

        if capacity > 0 && this.capacity == 0 {
            let size_t = Self::LAYOUT.size();
            let align_t = Self::LAYOUT.align();
            let layout = alloc::Layout::from_size_align(size_t * capacity, align_t).unwrap();
            let ptr = unsafe { alloc::alloc(layout) };

            match NonNull::new(ptr) {
                Some(new) => {
                    this.ptr = new.cast();
                    this.capacity = capacity;
                },
                None => alloc::handle_alloc_error(layout),
            }
        }

        this
    }

    pub(crate) fn from_ptr(src: *const T, len: usize) -> Self {
        let this = Self::with_capacity(len);
        let dst = this.ptr.as_ptr();
        unsafe {
            std::ptr::copy_nonoverlapping(src, dst, len);
        }
        this
    }

    pub(crate) fn grow(&mut self) {
        let size_t = Self::LAYOUT.size();
        let align_t = Self::LAYOUT.align();

        let (layout, ptr, new_capacity) = if self.capacity == 0 {
            let layout = alloc::Layout::from_size_align(size_t * 4, align_t).unwrap();
            let ptr = unsafe { alloc::alloc(layout) };

            (layout, ptr, 4)
        } else {
            let new_capacity = self.capacity * 2;
            let new_size = size_t * new_capacity;
            let layout = alloc::Layout::from_size_align(size_t * self.capacity, align_t).unwrap();
            let ptr = unsafe { alloc::realloc(self.ptr.as_ptr().cast::<u8>(), layout, new_size) };

            (layout, ptr, new_capacity)
        };

        match NonNull::new(ptr) {
            Some(new) => {
                self.ptr = new.cast();
                self.capacity = new_capacity;
            },
            None => alloc::handle_alloc_error(layout),
        }
    }

    pub(crate) const fn is_zst() -> bool {
        Self::LAYOUT.size() == 0
    }

    pub(crate) const fn check(&self, offset: usize) -> Result<(), Error> {
        if self.capacity == 0 {
            return Err(Error::Uninitialized);
        } else if offset >= self.capacity {
            return Err(Error::ExceedCurrentCapacity);
        } else {
            Ok(())
        }
    }

    pub(crate) const unsafe fn get_raw(&self, offset: usize) -> *mut T {
        unsafe {
            self.ptr.add(offset).as_ptr()
        }
    }

    pub(crate) const unsafe fn push(&mut self, data: T, offset: usize) {
        if Self::is_zst() {
            unsafe {
                self.ptr.write(data);
                return;
            }
        }

        unsafe {
            self.ptr.add(offset).write(data);
        }
    }

    pub(crate) unsafe fn swap_remove(
        &mut self,
        index: usize,
        last_index: usize,
    ) -> *mut T {
        if Self::is_zst() {
            let ptr = std::ptr::without_provenance_mut(self.ptr.as_ptr() as usize + index);
            return ptr;
        }

        unsafe {
            let last = self.get_raw(last_index);

            if index < last_index {
                let to_remove = self.get_raw(index);
                std::ptr::swap_nonoverlapping(to_remove, last, 1);
            }

            last
        }
    }

    pub(crate) fn swap(&mut self, a: usize, b: usize) {
        if !Self::is_zst() {
            unsafe {
                let ptr_a = self.ptr.add(a).as_ptr();
                let ptr_b = self.ptr.add(b).as_ptr();
                std::ptr::swap_nonoverlapping(ptr_a, ptr_b, 1);
            }
        }
    }

    pub(crate) fn pop(&mut self, last_index: usize) -> *mut T {
        if Self::is_zst() {
            let ptr = std::ptr::without_provenance_mut(self.ptr.as_ptr() as usize + last_index);
            return ptr;
        }

        unsafe {
            self.get_raw(last_index)
        }
    }

    pub(crate) fn clear(&mut self, len: usize) {
        if std::mem::needs_drop::<T>() {
            unsafe {
                std::ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), len).drop_in_place();
            }
        }
    }

    pub(crate) fn dealloc(&mut self) {
        let size_t = Self::LAYOUT.size();

        if self.capacity > 0 && size_t > 0 {
            unsafe {
                let size = size_t * self.capacity;
                let align = Self::LAYOUT.align();
                let alloc_layout = alloc::Layout::from_size_align_unchecked(size, align);
                alloc::dealloc(self.ptr.as_ptr().cast::<u8>(), alloc_layout);
            }
        }
    }
}
