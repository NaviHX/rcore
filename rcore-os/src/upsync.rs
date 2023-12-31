use core::cell::{RefCell, RefMut, Ref};
/// This cell is safe only when being used in single-processor environment
pub struct UPSyncCell<T> {
    inner: RefCell<T>,
}

impl<T> UPSyncCell<T> {
    /// SAFETY: The inner struct is only used in uniprocessor
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }

    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }

    pub fn borrow(&self) -> Ref<T> {
        self.inner.borrow()
    }
}

unsafe impl<T> Sync for UPSyncCell<T> {}
