use crate::util::cascading::CascadingMap;
use std::hash::Hash;

pub struct CachingMap<K, V> {
    cache: moka::future::Cache<K, V>,
    delegate: CascadingMap<K, V>,
}

impl<K, V> CachingMap<K, V>
where
    K: Eq + Hash + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(capacity: u64, delegate: CascadingMap<K, V>) -> Self {
        Self {
            cache: moka::future::Cache::builder()
                .max_capacity(capacity)
                .support_invalidation_closures()
                .build(),
            delegate,
        }
    }

    pub async fn get(&self, key: K) -> Result<Option<V>, crate::Error> {
        if let Some(cached) = self.cache.get(&key) {
            return Ok(Some(cached));
        }

        let new_value = self.delegate.get(&key).await?;
        if let Some(new_value) = &new_value {
            self.cache.insert(key, new_value.clone()).await;
        }

        Ok(new_value)
    }

    pub async fn get_with(&self, key: K, default: impl FnOnce() -> V) -> Result<V, crate::Error> {
        let value = self.get(key).await?.unwrap_or_else(default);

        Ok(value)
    }

    pub fn invalidate_entries_if<F>(&self, predicate: F)
    where
        F: Fn(&K, &V) -> bool + Send + Sync + 'static,
    {
        self.cache.invalidate_entries_if(predicate).unwrap();
    }
}
