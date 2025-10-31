use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct KvStore {
    inner: Arc<RwLock<HashMap<String, String>>>,
}

impl KvStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn put(&self, key: String, value: String) {
        let mut map = self.inner.write().expect("rwlock poisoned");
        map.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let map = self.inner.read().expect("rwlock poisoned");
        map.get(key).cloned()
    }

    pub fn delete(&self, key: &str) -> bool {
        let mut map = self.inner.write().expect("rwlock poisoned");
        map.remove(key).is_some()
    }
}
