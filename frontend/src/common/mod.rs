use std::ops::Deref;

pub struct UnsafeSync<T>(pub T);

unsafe impl<T> Send for UnsafeSync<T> {}
unsafe impl<T> Sync for UnsafeSync<T> {}

impl<T> Deref for UnsafeSync<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for UnsafeSync<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}
