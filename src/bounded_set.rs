use std::collections::HashSet;

pub struct BoundedSet<T> {
    set: HashSet<T>,
    insertion_order: Vec<T>,
    max_size: usize,
    ring_index: usize,
}

impl<T> BoundedSet<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    // Create a new set that is bounded to have at most max_size elements
    // Once new elements are inserted, the oldest one is removed
    // However, if the same elment is inserted another time, it is still
    // considered to be as old as the initial insertion was.
    pub fn new(max_size: usize) -> Self {
        BoundedSet {
            set: HashSet::new(),
            insertion_order: Vec::new(),
            max_size,
            ring_index: 0usize,
        }
    }

    pub fn insert(&mut self, value: T) {
        if self.set.insert(value.clone()) {
            if self.insertion_order.len() == self.max_size {
                self.set.remove(&self.insertion_order[self.ring_index]);
                self.insertion_order[self.ring_index] = value;
                self.ring_index = (self.ring_index + 1) % self.max_size;
            } else {
                self.insertion_order.push(value);
            }
        }
    }

    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        T: std::borrow::Borrow<Q>, // T (e.g., String) must be able to borrow as Q (e.g., str)
        Q: std::hash::Hash + Eq + ?Sized, // Q (e.g., str) must be hashable and comparable
    {
        self.set.contains(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty_set() {
        let set: BoundedSet<i32> = BoundedSet::new(5);
        // Assertions on internal state are for detailed testing, normally you'd use public methods
        assert_eq!(set.set.len(), 0);
        assert_eq!(set.insertion_order.len(), 0);
        assert_eq!(set.max_size, 5);
        assert_eq!(set.ring_index, 0);
    }

    #[test]
    fn test_insert_below_max_size() {
        let mut set = BoundedSet::new(3);

        // Insert first element
        set.insert(10);
        assert!(set.contains(&10));
        assert_eq!(set.set.len(), 1);
        assert_eq!(set.insertion_order.len(), 1);
        assert_eq!(set.insertion_order[0], 10); // Internal check for order

        // Insert second element
        set.insert(20);
        assert!(set.contains(&10));
        assert!(set.contains(&20));
        assert_eq!(set.set.len(), 2);
        assert_eq!(set.insertion_order.len(), 2);
        assert_eq!(set.insertion_order[1], 20); // Internal check for order

        // Insert third element (reaching max_size)
        set.insert(30);
        assert!(set.contains(&10));
        assert!(set.contains(&20));
        assert!(set.contains(&30));
        assert_eq!(set.set.len(), 3);
        assert_eq!(set.insertion_order.len(), 3);
        assert_eq!(set.insertion_order[2], 30); // Internal check for order
    }

    #[test]
    fn test_insert_beyond_max_size_removes_oldest() {
        let mut set = BoundedSet::new(3);
        set.insert(1); // Oldest: 1
        set.insert(2); // Oldest: 1
        set.insert(3); // Oldest: 1, Set: {1, 2, 3}, order: [1, 2, 3], ring_idx: 0

        // Insert 4: 1 should be removed
        set.insert(4); // Oldest: 2, Set: {2, 3, 4}, order: [4, 2, 3], ring_idx: 1
        assert!(!set.contains(&1));
        assert!(set.contains(&2));
        assert!(set.contains(&3));
        assert!(set.contains(&4));
        assert_eq!(set.set.len(), 3);
        assert_eq!(set.insertion_order[0], 4); // New value replaced oldest slot

        // Insert 5: 2 should be removed
        set.insert(5); // Oldest: 3, Set: {3, 4, 5}, order: [4, 5, 3], ring_idx: 2
        assert!(!set.contains(&2));
        assert!(set.contains(&3));
        assert!(set.contains(&4));
        assert!(set.contains(&5));
        assert_eq!(set.set.len(), 3);
        assert_eq!(set.insertion_order[1], 5); // New value replaced next oldest slot

        // Insert 6: 3 should be removed (ring_index wraps around)
        set.insert(6); // Oldest: 4, Set: {4, 5, 6}, order: [4, 5, 6], ring_idx: 0
        assert!(!set.contains(&3));
        assert!(set.contains(&4));
        assert!(set.contains(&5));
        assert!(set.contains(&6));
        assert_eq!(set.set.len(), 3);
        assert_eq!(set.insertion_order[2], 6); // New value replaced next oldest slot
    }

    #[test]
    fn test_reinsert_existing_element_does_not_change_age() {
        let mut set = BoundedSet::new(3);
        set.insert(10); // Oldest: 10
        set.insert(20); // Oldest: 10
        set.insert(30); // Oldest: 10, Set: {10, 20, 30}, order: [10, 20, 30], ring_idx: 0

        // Re-insert 20. It should not be considered "newest".
        set.insert(20);
        assert!(set.contains(&10));
        assert!(set.contains(&20));
        assert!(set.contains(&30));
        assert_eq!(set.set.len(), 3);
        assert_eq!(set.insertion_order, vec![10, 20, 30]); // Order unchanged
        assert_eq!(set.ring_index, 0); // Ring index unchanged

        // Insert a new element (40). The *original* oldest (10) should be removed.
        set.insert(40); // Oldest: 20, Set: {20, 30, 40}, order: [40, 20, 30], ring_idx: 1
        assert!(!set.contains(&10)); // 10 is removed
        assert!(set.contains(&20));
        assert!(set.contains(&30));
        assert!(set.contains(&40));
        assert_eq!(set.set.len(), 3);
        assert_eq!(set.insertion_order[0], 40); // 40 took 10's slot
    }

    #[test]
    fn test_contains_method() {
        let mut set = BoundedSet::new(2);
        set.insert(100);
        set.insert(200);

        assert!(set.contains(&100));
        assert!(set.contains(&200));
        assert!(!set.contains(&300)); // Not in set

        set.insert(300); // 100 removed
        assert!(!set.contains(&100)); // Should no longer contain 100
        assert!(set.contains(&200));
        assert!(set.contains(&300));
    }

    #[test]
    fn test_max_size_one() {
        let mut set = BoundedSet::new(1);

        set.insert(10);
        assert!(set.contains(&10));
        assert_eq!(set.set.len(), 1);
        assert_eq!(set.insertion_order, vec![10]);
        assert_eq!(set.ring_index, 0);

        set.insert(20); // 10 removed
        assert!(!set.contains(&10));
        assert!(set.contains(&20));
        assert_eq!(set.set.len(), 1);
        assert_eq!(set.insertion_order, vec![20]);
        assert_eq!(set.ring_index, 0); // (0+1)%1 = 0

        set.insert(10); // 20 removed
        assert!(set.contains(&10));
        assert!(!set.contains(&20));
        assert_eq!(set.set.len(), 1);
        assert_eq!(set.insertion_order, vec![10]);
        assert_eq!(set.ring_index, 0);
    }

    #[test]
    fn test_different_types() {
        let mut set: BoundedSet<String> = BoundedSet::new(2);
        set.insert("apple".to_string());
        set.insert("banana".to_string());

        assert!(set.contains(&"apple".to_string()));
        assert!(set.contains(&"banana".to_string()));

        set.insert("orange".to_string()); // "apple" removed
        assert!(!set.contains(&"apple".to_string()));
        assert!(set.contains(&"banana".to_string()));
        assert!(set.contains(&"orange".to_string()));
    }

    #[test]
    #[should_panic] // As discussed, this behavior is accepted.
    fn test_max_size_zero_panic() {
        let mut set = BoundedSet::new(0);
        // This will panic inside the insert method because `max_size` is 0.
        // Specifically, it will try to access `self.insertion_order[self.ring_index]`
        // where `insertion_order` is empty, or perform a modulo by zero if ring_index
        // was non-zero and max_size was 0 (though ring_index starts at 0).
        set.insert(1);
    }
}
