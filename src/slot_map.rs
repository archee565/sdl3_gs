/// A compact free-list–backed container that hands out stable integer indices.
///
/// Reuses slots from removed entries before growing the backing `Vec`.
pub struct SlotMap<T> {
    slots: Vec<SlotEntry<T>>,
    first_free: i32,
}

enum SlotEntry<T> {
    Occupied(T),
    Free { next_free: i32 },
}

impl<T> SlotMap<T> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            first_free: -1,
        }
    }

    /// Insert a value and return its stable index.
    pub fn insert(&mut self, value: T) -> i32 {
        if self.first_free >= 0 {
            let idx = self.first_free;
            let entry = &mut self.slots[idx as usize];
            match entry {
                SlotEntry::Free { next_free } => self.first_free = *next_free,
                SlotEntry::Occupied(_) => unreachable!(),
            }
            *entry = SlotEntry::Occupied(value);
            idx
        } else {
            let idx = self.slots.len() as i32;
            self.slots.push(SlotEntry::Occupied(value));
            idx
        }
    }

    /// Remove the entry at `index`, returning the value.
    ///
    /// # Panics
    /// Panics if the slot is already free.
    pub fn remove(&mut self, index: i32) -> T {
        let entry = &mut self.slots[index as usize];
        let old = std::mem::replace(entry, SlotEntry::Free { next_free: self.first_free });
        self.first_free = index;
        match old {
            SlotEntry::Occupied(v) => v,
            SlotEntry::Free { .. } => panic!("slot already free"),
        }
    }

    pub fn get(&self, index: i32) -> &T {
        match &self.slots[index as usize] {
            SlotEntry::Occupied(v) => v,
            SlotEntry::Free { .. } => panic!("slot is free"),
        }
    }

    pub fn get_mut(&mut self, index: i32) -> &mut T {
        match &mut self.slots[index as usize] {
            SlotEntry::Occupied(v) => v,
            SlotEntry::Free { .. } => panic!("slot is free"),
        }
    }

    /// Iterate over all occupied entries, yielding `(index, &T)`.
    pub fn iter(&self) -> impl Iterator<Item = (i32, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, entry)| match entry {
            SlotEntry::Occupied(v) => Some((i as i32, v)),
            SlotEntry::Free { .. } => None,
        })
    }
}
