#[cfg(feature = "openapi")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::sync::Arc;
#[cfg(feature = "openapi")]
use alloc::vec::Vec;
use core::fmt::Display;
use core::sync::atomic::{AtomicU8, Ordering};
#[cfg(any(test, feature = "test-utils"))]
use std::sync::Mutex;

#[cfg(feature = "alloc")]
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[cfg(feature = "alloc")]
static ID_SEQ: Lazy<Arc<AtomicU8>> = Lazy::new(Arc::default);

#[cfg(not(feature = "alloc"))]
static ID_SEQ: AtomicU8 = AtomicU8::new(0);

#[cfg(any(test, feature = "test-utils"))]
static RESET_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SlideId(u8);

impl SlideId {
    #[must_use]
    pub fn next() -> Self {
        #[cfg(feature = "alloc")]
        {
            let seq = Lazy::force(&ID_SEQ);
            let id = seq.fetch_add(1, Ordering::Relaxed);
            Self(id)
        }
        #[cfg(not(feature = "alloc"))]
        {
            let id = ID_SEQ.fetch_add(1, Ordering::Relaxed);
            Self(id)
        }
    }

    /// Reset the global ID sequence to a specific value. Only available for testing.
    /// This function is thread-safe and can be used in multi-threaded test environments.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned (which should not happen in normal test scenarios).
    #[cfg(any(test, feature = "test-utils"))]
    #[allow(clippy::unwrap_used)] // Acceptable in test-only code
    pub fn reset_sequence() {
        Self::reset_sequence_to(0);
    }

    /// Reset the global ID sequence to a specific value. Only available for testing.
    /// This allows tests to start with predictable IDs even when running in parallel.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned (which should not happen in normal test scenarios).
    #[cfg(any(test, feature = "test-utils"))]
    #[allow(clippy::unwrap_used)] // Acceptable in test-only code
    pub fn reset_sequence_to(value: u8) {
        let _guard = RESET_MUTEX.lock().unwrap();
        #[cfg(feature = "alloc")]
        {
            let seq = Lazy::force(&ID_SEQ);
            seq.store(value, Ordering::SeqCst);
        }
        #[cfg(not(feature = "alloc"))]
        {
            ID_SEQ.store(value, Ordering::SeqCst);
        }
    }

    /// Get the current sequence value. Only available for testing.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned (which should not happen in normal test scenarios).
    #[cfg(any(test, feature = "test-utils"))]
    #[allow(clippy::unwrap_used)] // Acceptable in test-only code
    pub fn current_sequence() -> u8 {
        let _guard = RESET_MUTEX.lock().unwrap();
        #[cfg(feature = "alloc")]
        {
            let seq = Lazy::force(&ID_SEQ);
            seq.load(Ordering::SeqCst)
        }
        #[cfg(not(feature = "alloc"))]
        {
            ID_SEQ.load(Ordering::SeqCst)
        }
    }
}

impl Display for SlideId {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "{}", self.0)
    }
}
