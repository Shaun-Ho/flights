use std::collections::hash_map::{Entry as HashEntry, HashMap, OccupiedEntry as HashOccupiedEntry};
use std::error::Error;
use std::fmt;
use std::hash::{BuildHasher, Hash};

pub struct TryInsertError<E, V> {
    pub entry: E,
    pub value: V,
}

impl<E: fmt::Debug, V: fmt::Debug> fmt::Debug for TryInsertError<E, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TryInsertError")
            .field("entry", &self.entry)
            .field("value", &self.value)
            .finish()
    }
}

impl<E, V> fmt::Display for TryInsertError<E, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to insert because key is already occupied")
    }
}

impl<E: fmt::Debug, V: fmt::Debug> Error for TryInsertError<E, V> {}

/// Extension trait to add `try_insert` to standard library HashMap.
pub trait TryInsertExt<K, V> {
    type Occupied<'a>
    where
        Self: 'a;

    fn try_insert(
        &mut self,
        key: K,
        value: V,
    ) -> Result<&mut V, TryInsertError<Self::Occupied<'_>, V>>;
}

impl<K, V, S> TryInsertExt<K, V> for HashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    type Occupied<'a>
        = HashOccupiedEntry<'a, K, V>
    where
        Self: 'a;

    fn try_insert(
        &mut self,
        key: K,
        value: V,
    ) -> Result<&mut V, TryInsertError<Self::Occupied<'_>, V>> {
        match self.entry(key) {
            HashEntry::Occupied(entry) => Err(TryInsertError { entry, value }),
            HashEntry::Vacant(entry) => Ok(entry.insert(value)),
        }
    }
}
