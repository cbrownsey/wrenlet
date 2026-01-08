use std::{cell::Cell, mem::offset_of, ptr::NonNull};

#[repr(transparent)]
pub struct ErasedRc {
    ptr: NonNull<()>,
}

impl ErasedRc {
    /// Creates a new type erased reference counted pointer.
    pub fn new<T>(value: T) -> ErasedRc {
        fn drop_fn<T>(e: &ErasedRc) {
            let ptr = e.inner_ptr::<T>();
            let _ = unsafe { Box::from_raw(ptr) };
        }

        let x = Box::new(ErasedRcInner {
            ref_count: Cell::new(1),
            drop_fn: drop_fn::<T>,
            value,
        });

        ErasedRc {
            ptr: unsafe { NonNull::new_unchecked(Box::into_raw(x).cast()) },
        }
    }

    /// Converts this reference into a mutable pointer.
    ///
    /// This does not decrement the reference count. To avoid a memory leak, it must be converted
    /// back into an ErasedRc with [`from_raw`].
    ///
    /// # Safety
    /// The given type `T` must be the same type with which this pointer was created.
    #[must_use]
    pub unsafe fn into_raw<T>(self) -> *mut T {
        // Safety: As per function precondition.
        let ptr = unsafe { self.value_ptr() };
        std::mem::forget(self);
        ptr
    }

    /// Converts a pointer back into an `ErasedRc`.
    ///
    /// # Safety
    /// This pointer must have previously come from a call to [`into_raw`].
    #[must_use]
    pub unsafe fn from_raw<T>(ptr: *mut T) -> ErasedRc {
        let ptr = ptr.cast::<()>();
        let offset = offset_of!(ErasedRcInner<T>, value);

        let ptr = unsafe { NonNull::new_unchecked(ptr.byte_sub(offset)) };

        ErasedRc { ptr }
    }

    /// Retrieves a pointer to the value stored in this `ErasedRc`.
    ///
    /// # Safety
    /// The given type `T` must be the same type with which this pointer was created.
    unsafe fn value_ptr<T>(&self) -> *mut T {
        let ptr = self.inner_ptr::<T>();
        let offset = offset_of!(ErasedRcInner<T>, value);
        // SAFETY: The pointer is originally to an `ErasedRcInner<T>`, so this offset must be
        // valid.
        unsafe { ptr.byte_add(offset) }.cast()
    }

    /// Retrieves the value pointer of this `ErasedGc`.
    ///
    /// # Safety
    /// The given type `T` must be the same type with which this pointer was created.
    pub unsafe fn value<T>(&self) -> &T {
        unsafe { &*self.value_ptr::<T>() }
    }

    /// A convenience method for getting the typed pointer.
    fn inner_ptr<T>(&self) -> *mut ErasedRcInner<T> {
        self.ptr.as_ptr().cast()
    }

    /// Retrieves a reference to the `ErasedRcInner<T>` associated with this pointer.
    ///
    /// # Safety
    /// The given type `T` must either be the same type with which this pointer was created, or the unit type `()`.
    unsafe fn inner_ref<T>(&self) -> &ErasedRcInner<T> {
        unsafe { &*self.inner_ptr::<T>() }
    }

    /// Retrieves the reference count associated with this pointer.
    pub fn ref_count(&self) -> usize {
        self.count_cell_ref().get()
    }

    fn count_cell_ref(&self) -> &Cell<usize> {
        &unsafe { self.inner_ref::<()>() }.ref_count
    }

    /// Increments the reference count associated with this pointer by one.
    unsafe fn inc_count(&self) {
        self.count_cell_ref().update(|n| n + 1);
    }

    /// Decrements the reference count associated with this pointer by one.
    ///
    /// # Safety
    /// This function must not be called if the reference count is already zero.
    unsafe fn dec_count(&self) {
        self.count_cell_ref()
            .update(|n| unsafe { n.unchecked_sub(1) });
    }
}

impl std::fmt::Debug for ErasedRc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErasedRc").finish_non_exhaustive()
    }
}

impl Clone for ErasedRc {
    fn clone(&self) -> Self {
        unsafe { self.inc_count() };
        ErasedRc { ptr: self.ptr }
    }
}

impl Drop for ErasedRc {
    fn drop(&mut self) {
        unsafe { self.dec_count() };
        if self.ref_count() == 0 {
            let drop_fn = unsafe { self.inner_ref::<()>() }.drop_fn;

            drop_fn(self)
        }
    }
}

#[repr(C)]
struct ErasedRcInner<T> {
    ref_count: Cell<usize>,
    drop_fn: fn(&ErasedRc),
    value: T,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_usage() {
        let erased = ErasedRc::new(String::from("Hello, World!"));

        assert_eq!(unsafe { erased.value::<String>() }, "Hello, World!")
    }

    #[test]
    fn multiple_instances() {
        let erased1 = ErasedRc::new(String::from("Hello, World!"));
        let erased2 = erased1.clone();

        assert_eq!(unsafe { erased1.value::<String>() }, "Hello, World!");
        assert_eq!(unsafe { erased2.value::<String>() }, "Hello, World!");

        assert!(std::ptr::eq(unsafe { erased1.value::<String>() }, unsafe {
            erased2.value::<String>()
        }));
    }

    #[test]
    fn into_and_from_raw() {
        let erased = ErasedRc::new(String::from("Hello, World!"));

        let ptr = unsafe { erased.into_raw::<String>() };

        assert_eq!(unsafe { &*ptr }, "Hello, World!");

        let _ = unsafe { ErasedRc::from_raw(ptr) };
    }
}
