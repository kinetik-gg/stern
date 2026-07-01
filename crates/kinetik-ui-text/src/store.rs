use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use kinetik_ui_core::TextLayoutId;

use crate::{CosmicTextEngine, ShapedTextLayout, TextLayoutKey};

/// Persistent shaped text layout cache.
///
/// The store owns the text engine and assigns stable layout handles from
/// layout keys. UI layers can request handles while render backends register
/// the resulting owned layouts as resources.
pub struct TextLayoutStore {
    engine: CosmicTextEngine,
    keys: HashMap<TextLayoutKey, TextLayoutId>,
    pub(crate) layouts: HashMap<TextLayoutId, Arc<ShapedTextLayout>>,
}

impl TextLayoutStore {
    /// Creates an empty shaped text layout store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: CosmicTextEngine::new(),
            keys: HashMap::new(),
            layouts: HashMap::new(),
        }
    }

    /// Returns the number of cached shaped layouts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.layouts.len()
    }

    /// Returns true when no shaped layouts are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }

    /// Clears all cached shaped layouts.
    pub fn clear(&mut self) {
        self.keys.clear();
        self.layouts.clear();
    }

    /// Returns the backing text engine.
    #[must_use]
    pub const fn engine(&self) -> &CosmicTextEngine {
        &self.engine
    }

    /// Returns mutable access to the backing text engine.
    pub fn engine_mut(&mut self) -> &mut CosmicTextEngine {
        &mut self.engine
    }

    /// Returns a stable layout ID for a text layout key, shaping on cache miss.
    pub fn layout_id(&mut self, key: TextLayoutKey) -> TextLayoutId {
        let preferred_id = text_layout_id(&key);
        self.layout_id_with_preferred_id(key, preferred_id)
    }

    pub(crate) fn layout_id_with_preferred_id(
        &mut self,
        key: TextLayoutKey,
        preferred_id: TextLayoutId,
    ) -> TextLayoutId {
        if let Some(id) = self.keys.get(&key) {
            return *id;
        }

        let id = self.available_layout_id(preferred_id);
        let layout = self.engine.shape_text(&key);
        self.keys.insert(key, id);
        self.layouts.insert(id, Arc::new(layout));
        id
    }

    fn available_layout_id(&self, preferred_id: TextLayoutId) -> TextLayoutId {
        let mut raw = preferred_id.raw().max(1);

        loop {
            let id = TextLayoutId::from_raw(raw);
            if !self.layouts.contains_key(&id) {
                return id;
            }

            raw = raw.wrapping_add(1).max(1);
        }
    }

    /// Returns a shaped layout by ID.
    #[must_use]
    pub fn layout(&self, id: TextLayoutId) -> Option<&ShapedTextLayout> {
        self.layouts.get(&id).map(Arc::as_ref)
    }

    /// Iterates cached shaped text layouts.
    pub fn layouts(&self) -> impl Iterator<Item = StoredTextLayout<'_>> {
        self.keys.iter().filter_map(|(key, id)| {
            self.layouts.get(id).map(|layout| StoredTextLayout {
                id: *id,
                key,
                layout: Arc::clone(layout),
            })
        })
    }
}

impl Default for TextLayoutStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Borrowed shaped text layout entry.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredTextLayout<'a> {
    /// Text layout handle.
    pub id: TextLayoutId,
    /// Layout request used to shape the text.
    pub key: &'a TextLayoutKey,
    /// Shaped layout.
    pub layout: Arc<ShapedTextLayout>,
}

fn text_layout_id(key: &TextLayoutKey) -> TextLayoutId {
    let mut hasher = StableHasher::new();
    key.hash(&mut hasher);
    TextLayoutId::from_raw(hasher.finish().max(1))
}

struct StableHasher(u64);

impl StableHasher {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    const fn new() -> Self {
        Self(Self::OFFSET)
    }
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn write_u8(&mut self, i: u8) {
        self.write(&[i]);
    }

    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes());
    }

    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes());
    }

    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes());
    }

    fn write_usize(&mut self, i: usize) {
        self.write_u64(u64::try_from(i).unwrap_or(u64::MAX));
    }
}
