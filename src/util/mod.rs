mod caching;
mod cascading;

pub use caching::CachingMap;
pub use cascading::CascadingMap;

#[poise::async_trait]
pub trait AsyncSource<K, V> {
    async fn get(&self, key: &K) -> Result<Option<V>, crate::Error>;
}
