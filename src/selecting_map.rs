use std::collections::BTreeMap;
use std::ops::Bound;

pub struct SelectingMap<K, V> {
    contents: BTreeMap<K, V>,
    selected_index: Option<K>,
    changed: bool,
}

impl<K: std::cmp::Ord + std::marker::Copy, V> SelectingMap<K, V> {
    pub fn new() -> SelectingMap<K, V> {
        SelectingMap {
            contents: BTreeMap::new(),
            selected_index: None,
            changed: false,
        }
    }

    pub fn update(&mut self, index: K, info: V) {
        self.contents.insert(index, info);

        if self.selected_index.is_none() {
            assert_ne!(self.contents.len(), 0);
            self.selected_index = Some(*self.contents.keys().next().expect("No key in SelectingMap after inserting"));
        }

        self.changed = true;
    }

    // pub fn next_key_filtered<F>(&self, key: K, filter: Option<F>) -> Option<K>
    //     where F: Fn(&(&K, &V)) -> bool {
    //     let filter: F = filter.unwrap_or(|x| true);
    //     if let Some((next_key, _)) = self.contents.range((Bound::Excluded(key), Bound::Unbounded)).filter(filter).next() {
    //         return Some(*next_key);
    //     } else {
    //         return None;
    //     }
    // }

    pub fn next_key(&self, key: K) -> Option<K> {
        if let Some((next_key, _)) = self.contents.range((Bound::Excluded(key), Bound::Unbounded)).next() {
            return Some(*next_key);
        } else {
            return None;
        }
    }

    pub fn prev_key(&self, key: K) -> Option<K> {
        if let Some((prev_key, _)) = self.contents.range((Bound::Unbounded, Bound::Excluded(key))).next_back() {
            return Some(*prev_key);
        } else {
            return None;
        }
    }

    pub fn select_next(&mut self) {
        if let Some(current) = self.selected_index {
            if let Some(next_key) = self.next_key(current) {
                self.selected_index = Some(next_key);
                self.changed = true;
            }
        }
    }

    pub fn select_prev(&mut self) {
        if let Some(current) = self.selected_index {
            if let Some(prev_key) = self.prev_key(current) {
                self.selected_index = Some(prev_key);
                self.changed = true;
            }
        }
    }

    pub fn get_selected(&self) -> Option<&V> {
        let index = self.selected_index?;
        return Some(self.contents.get(&index).expect("Selected key is not in contents list"));
    }

    pub fn get_selected_mut(&mut self) -> Option<&mut V> {
        let index = self.selected_index?;
        self.changed = true;
        return Some(self.contents.get_mut(&index).expect("Selected key is not in contents list"));
    }

    pub fn len(&self) -> usize {
        return self.contents.len();
    }

    pub fn values(&self) -> std::collections::btree_map::Values<K, V> {
        return self.contents.values();
    }

    pub fn get(&self, index: K) -> Option<&V> {
        return self.contents.get(&index);
    }

    pub fn remove(&mut self, index: K) {
        // set selected_index to a value that will still be there
        // an entry will be removed, so contents shouldn't be empty, so there should be a selection
        if index == self.selected_index.expect("No selected entry while removing one") {
            if let Some(new_index) = self.next_key(index) {
                // take the next one
                self.selected_index = Some(new_index);
            } else if let Some(new_index) = self.prev_key(index) {
                // take the previous one
                self.selected_index = Some(new_index);
            } else {
                // ok, there are none
                assert_eq!(self.contents.len(), 1);
                self.selected_index = None;
            }
        }

        self.contents.remove(&index);
        self.changed = true;
    }

    pub fn get_changed(&self) -> bool {
        return self.changed;
    }

    pub fn reset_changed(&mut self) -> bool {
        let t = self.changed;
        self.changed = false;
        return t;
    }


    pub fn filtered_next_key<F>(&self, key: K, mut filter: F) -> Option<K>
        where F: FnMut(&V) -> bool {
        if let Some((next_key, _)) = self.contents.range((Bound::Excluded(key), Bound::Unbounded)).filter(|(k,v)| filter(v)).next() {
            return Some(*next_key);
        } else {
            return None;
        }
    }

    pub fn filtered_prev_key<F>(&self, key: K, mut filter: F) -> Option<K>
        where F: FnMut(&V) -> bool {
        if let Some((prev_key, _)) = self.contents.range((Bound::Unbounded, Bound::Excluded(key))).filter(|(k,v)| filter(v)).next_back() {
            return Some(*prev_key);
        } else {
            return None;
        }
    }

    pub fn filtered_select_next<F>(&mut self, filter: F)
        where F: FnMut(&V) -> bool {
        if let Some(current) = self.selected_index {
            if let Some(next_key) = self.filtered_next_key(current, filter) {
                self.selected_index = Some(next_key);
                self.changed = true;
            }
        }
    }

    pub fn filtered_select_prev<F>(&mut self, filter: F)
        where F: FnMut(&V) -> bool {
        if let Some(current) = self.selected_index {
            if let Some(prev_key) = self.filtered_prev_key(current, filter) {
                self.selected_index = Some(prev_key);
                self.changed = true;
            }
        }
    }

    pub fn filtered_len<F>(&self, filter: F) -> usize
        where for<'r> F: FnMut(&'r &V) -> bool { // XXX
        return self.contents.values().filter(filter).count();
    }

    pub fn filtered_values<F>(&self, filter: F) -> std::iter::Filter<std::collections::btree_map::Values<K, V>, F>
        where F: FnMut(&&V) -> bool { // XXX
        return self.contents.values().filter(filter);
    }

    pub fn filtered_select_next_else_prev<F>(&mut self, filter: F)
        where F: Fn(&V) -> bool {
        if let Some(index) = self.selected_index {
            if let Some(new_index) = self.filtered_next_key(index, &filter) {
                // take the next one
                self.selected_index = Some(new_index);
            } else if let Some(new_index) = self.filtered_prev_key(index, &filter) {
                // take the previous one
                self.selected_index = Some(new_index);
            }
        }
    }
}
