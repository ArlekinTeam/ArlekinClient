pub mod threading;

use std::ops::{Deref, DerefMut};

pub struct UnsafeSync<T>(pub T);

unsafe impl<T> Send for UnsafeSync<T> {}
unsafe impl<T> Sync for UnsafeSync<T> {}

impl<T> Deref for UnsafeSync<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for UnsafeSync<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<T> for UnsafeSync<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T: Clone> Clone for UnsafeSync<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: PartialEq> PartialEq for UnsafeSync<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}
