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
