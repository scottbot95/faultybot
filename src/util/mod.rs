mod caching;
mod cascading;
mod cooldowns;

pub use caching::CachingMap;
pub use cascading::CascadingMap;
pub use cooldowns::*;

#[poise::async_trait]
pub trait AsyncSource<K, V> {
    async fn get(&self, key: &K) -> Result<Option<V>, crate::Error>;
}

/// Used to convert a value from an i64. Primarily used for serenity ID types
/// so we can serialize them for storing in Postgres which doesn't support unsigned types
pub trait Fromi64 {
    fn from_i64(int: i64) -> Self;
}

impl<T: From<u64>> Fromi64 for T {
    fn from_i64(int: i64) -> Self {
        let unsigned: u64 = unsafe { std::mem::transmute(int) };
        T::from(unsigned)
    }
}

/// Used to convert a value to an i64. Primarily used for serenity ID types
/// so we can serialize them for storing in Postgres which doesn't support unsigned types
pub trait Toi64 {
    fn to_i64(self) -> i64;
}

impl<T: Into<u64>> Toi64 for T {
    fn to_i64(self) -> i64 {
        unsafe { std::mem::transmute(T::into(self)) }
    }
}