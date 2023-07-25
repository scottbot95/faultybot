use std::pin::Pin;

pub struct CascadingMap<K, V> {
    // cache: moka::sync::Cache<SettingsContext2<'static>, serde_json::Value>,
    // db: crate::database::Database,
    sources: Vec<Pin<Box<dyn crate::util::AsyncSource<K, V> + Send + Sync + 'static>>>,
    merge_fn: Pin<Box<dyn Fn(&K, V, V) -> V + Send + Sync + 'static>>,
}

impl<K, V> CascadingMap<K, V> {
    pub fn new(sources: Vec<Pin<Box<dyn crate::util::AsyncSource<K, V> + Send + Sync>>>) -> Self {
        Self {
            sources,
            merge_fn: Box::pin(|_k, _old, new| new),
        }
    }

    pub fn new_with_merge(
        sources: Vec<Pin<Box<dyn crate::util::AsyncSource<K, V> + Send + Sync>>>,
        merge: impl Fn(&K, V, V) -> V + Send + Sync + 'static,
    ) -> Self {
        Self {
            sources,
            merge_fn: Box::pin(merge),
        }
    }

    pub async fn get(&self, key: &K) -> Result<Option<V>, crate::Error> {
        let mut value: Option<V> = None;
        for source in &self.sources {
            let new_val = source.get(key).await?;
            if let Some(new_val) = new_val {
                value = Some(match value {
                    Some(old_val) => (self.merge_fn)(key, old_val, new_val),
                    None => new_val,
                })
            }
        }

        Ok(value)
    }

    pub async fn get_with(&self, key: &K, default: impl FnOnce() -> V) -> Result<V, crate::Error> {
        let value = self.get(key).await?.unwrap_or_else(default);

        Ok(value)
    }
}
