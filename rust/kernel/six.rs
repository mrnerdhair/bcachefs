// SPDX-License-Identifier: GPL-2.0


//! Shared/intent/exclusive locks.
//!
//! C header: [`include/linux/six.h`](../../../include/linux/six.h)

#![allow(missing_docs)]

use crate::bindings;
use core::{ffi::c_void, ptr::null_mut};

/// A sleepable read/write lock; much like a read/write semaphore, but with third intermediate state, intent.
#[repr(transparent)]
pub struct SixLock(*mut bindings::six_lock);

impl SixLock {
    /// Obtain a read lock, spinning until successful.
    pub fn read(&self) -> ReadGuard<'_> {
        unsafe {
            bindings::six_lock_read(self.0, None, null_mut());
        }
        unsafe { ReadGuard::new(self) }
    }

    /// Obtain a read lock, sleeping if indicated by the provided closure.
    pub fn read_or_sleep(&self, should_sleep: &mut ShouldSleepFn) -> ReadGuard<'_> {
        let mut should_sleep_holder = ShouldSleepRefHolder(should_sleep);
        unsafe {
            bindings::six_lock_read(
                self.0,
                Some(rust_helper_six_locks_should_sleep),
                &mut should_sleep_holder as *mut _ as *mut c_void,
            );
        }
        unsafe { ReadGuard::new(self) }
    }

    /// Attempt to obtain a read lock without blocking.
    pub fn try_read(&self) -> Option<ReadGuard<'_>> {
        if unsafe { bindings::six_trylock_read(self.0) } {
            Some(unsafe { ReadGuard::new(self) })
        } else {
            None
        }
    }

    /// Obtain a write lock, spinning until successful.
    pub fn intent(&self) -> IntentGuard<'_> {
        unsafe {
            bindings::six_lock_intent(self.0, None, null_mut());
        }
        unsafe { IntentGuard::new(self) }
    }

    /// Obtain an intent lock, sleeping if indicated by the provided closure.
    pub fn intent_or_sleep(&self, should_sleep: &mut ShouldSleepFn) -> IntentGuard<'_> {
        let mut should_sleep_holder = ShouldSleepRefHolder(should_sleep);
        unsafe {
            bindings::six_lock_intent(
                self.0,
                Some(rust_helper_six_locks_should_sleep),
                &mut should_sleep_holder as *mut _ as *mut c_void,
            );
        }
        unsafe { IntentGuard::new(self) }
    }

    /// Attempt to obtain an intent lock without blocking.
    pub fn try_intent(&self) -> Option<IntentGuard<'_>> {
        if unsafe { bindings::six_trylock_intent(self.0) } {
            Some(unsafe { IntentGuard::new(self) })
        } else {
            None
        }
    }
}

pub struct RelockHandle<'a> {
    lock: &'a SixLock,
    seq: u32,
}

impl<'a> RelockHandle<'a> {
    /// Attempt to relock a previously-held lock for reading. Will fail if a write lock has been taken since the RelockHandle's creation.
    pub fn try_read(&self) -> Option<ReadGuard<'a>> {
        if unsafe { bindings::six_relock_read(self.lock.0, self.seq) } {
            Some(unsafe { ReadGuard::new(self.lock) })
        } else {
            None
        }
    }

    /// Attempt to immediately relock a previously-held lock for intent. Will fail if a write lock has been taken since the RelockHandle's creation.
    pub fn try_intent(&self) -> Option<IntentGuard<'a>> {
        if unsafe { bindings::six_relock_read(self.lock.0, self.seq) } {
            Some(unsafe { IntentGuard::new(self.lock) })
        } else {
            None
        }
    }
}

pub struct ReadGuard<'a> {
    lock: &'a SixLock,
}

impl<'a> ReadGuard<'a> {
    /// SAFETY: The lock must be held for reading.
    unsafe fn new(lock: &'a SixLock) -> Self {
        Self { lock }
    }
    pub fn try_upgrade(self) -> Result<IntentGuard<'a>, Self> {
        if unsafe { bindings::six_lock_tryupgrade(self.lock.0) } {
            Ok(unsafe { IntentGuard::new(self.lock) })
        } else {
            Err(self)
        }
    }
    pub fn relock_handle(&'a self) -> RelockHandle<'a> {
        RelockHandle {
            lock: self.lock,
            // SAFETY: safe because we hold the lock.
            seq: unsafe { (*self.lock.0).state.__bindgen_anon_4.seq },
        }
    }
}

impl<'a> Clone for ReadGuard<'a> {
    fn clone(&self) -> Self {
        unsafe {
            bindings::six_lock_increment(self.lock.0, bindings::six_lock_type_SIX_LOCK_read);
        }
        unsafe { Self::new(self.lock) }
    }
}

impl Drop for ReadGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: safe because we hold the lock.
        unsafe {
            bindings::six_unlock_read(self.lock.0);
        }
    }
}

pub struct IntentGuard<'a> {
    lock: &'a SixLock,
}

impl<'a> IntentGuard<'a> {
    /// SAFETY: The lock must be held for intent.
    unsafe fn new(lock: &'a SixLock) -> Self {
        Self { lock }
    }

    /// Obtain a write lock, spinning until successful.
    pub fn write(&'a self) -> WriteGuard<'_> {
        unsafe {
            bindings::six_lock_write(self.lock.0, None, null_mut());
        }
        unsafe { WriteGuard::new(self) }
    }

    /// Obtain a write lock, sleeping if indicated by the provided closure.
    pub fn write_or_sleep(&'a self, should_sleep: &mut ShouldSleepFn) -> WriteGuard<'_> {
        let mut should_sleep_holder = ShouldSleepRefHolder(should_sleep);
        unsafe {
            bindings::six_lock_write(
                self.lock.0,
                Some(rust_helper_six_locks_should_sleep),
                &mut should_sleep_holder as *mut _ as *mut c_void,
            );
        }
        unsafe { WriteGuard::new(self) }
    }

    /// Attempt to obtain a write lock without blocking.
    pub fn try_write(&'a self) -> Option<WriteGuard<'_>> {
        if unsafe { bindings::six_trylock_write(self.lock.0) } {
            Some(unsafe { WriteGuard::new(self) })
        } else {
            None
        }
    }

    /// Convert an intent lock into a read lock.
    pub fn downgrade(self) -> ReadGuard<'a> {
        unsafe {
            bindings::six_lock_downgrade(self.lock.0);
        }
        unsafe { ReadGuard::new(self.lock) }
    }
}

impl Clone for IntentGuard<'_> {
    fn clone(&self) -> Self {
        unsafe {
            bindings::six_lock_increment(self.lock.0, bindings::six_lock_type_SIX_LOCK_intent);
        }
        unsafe { Self::new(self.lock) }
    }
}

impl Drop for IntentGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: safe because we hold the lock.
        unsafe {
            bindings::six_unlock_intent(self.lock.0);
        }
    }
}

pub struct WriteGuard<'a> {
    intent: &'a IntentGuard<'a>,
}

impl<'a> WriteGuard<'a> {
    /// SAFETY: The lock must be held for writing.
    unsafe fn new(intent: &'a IntentGuard<'_>) -> Self {
        Self { intent }
    }
}

impl Drop for WriteGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: safe because we hold the lock.
        unsafe {
            bindings::six_unlock_write(self.intent.lock.0);
        }
    }
}

pub type ShouldSleepFn = dyn FnMut(&SixLock) -> bool;

/// A C-compatible container for a Rust-style fat reference to a ShouldSleepFn trait object.
struct ShouldSleepRefHolder<'a>(&'a mut ShouldSleepFn);

/// SAFETY:
///  - @lock must be a valid pointer to an initialized six_lock, which must live at least as long as @closure.
///  - @closure must be a valid pointer to an initialized SixLockShouldSleepHolder.
unsafe extern "C" fn rust_helper_six_locks_should_sleep(
    lock: *mut bindings::six_lock,
    closure: *mut c_void,
) -> i32 {
    let closure = unsafe { &mut *(closure as *mut ShouldSleepRefHolder<'_>) };
    (closure.0)(&SixLock(lock)).into()
}
