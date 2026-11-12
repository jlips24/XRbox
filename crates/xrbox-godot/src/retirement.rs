//! Bounded GPU lease retirement; injectable fence operations permit host tests.
pub struct Retired<T> {
    items: Vec<T>,
}
impl<T> Default for Retired<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}
impl<T> Retired<T> {
    pub fn push(
        &mut self,
        item: T,
        ready: impl Fn(&T) -> bool,
        wait_all: impl FnOnce(),
        mut release: impl FnMut(T),
    ) {
        self.items.push(item);
        if self.items.len() >= 4 {
            self.clear(wait_all, release);
            return;
        }
        let mut i = 0;
        while i < self.items.len() {
            if ready(&self.items[i]) {
                release(self.items.remove(i));
            } else {
                i += 1;
            }
        }
    }
    pub fn clear(&mut self, wait_all: impl FnOnce(), mut release: impl FnMut(T)) {
        wait_all();
        for item in self.items.drain(..) {
            release(item);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    #[test]
    fn pending_leases_wait_for_fences_and_full_queue_waits_before_release() {
        let events = RefCell::new(Vec::new());
        let mut retired = Retired::default();
        for n in 0..3 {
            retired.push(
                n,
                |_| false,
                || events.borrow_mut().push("wait"),
                |_| events.borrow_mut().push("release"),
            );
        }
        assert!(events.borrow().is_empty());
        retired.push(
            3,
            |_| false,
            || events.borrow_mut().push("wait"),
            |_| events.borrow_mut().push("release"),
        );
        assert_eq!(
            &*events.borrow(),
            &["wait", "release", "release", "release", "release"]
        );
        assert!(retired.items.is_empty());
    }
    #[test]
    fn only_completed_fences_release_until_explicit_shutdown() {
        let mut retired = Retired::default();
        let released = RefCell::new(Vec::new());
        retired.push(
            1,
            |_| false,
            || panic!("not full"),
            |n| released.borrow_mut().push(n),
        );
        retired.push(
            2,
            |n| *n == 1,
            || panic!("not full"),
            |n| released.borrow_mut().push(n),
        );
        assert_eq!(&*released.borrow(), &[1]);
        assert_eq!(retired.items, vec![2]);
        retired.clear(
            || released.borrow_mut().push(0),
            |n| released.borrow_mut().push(n),
        );
        assert_eq!(&*released.borrow(), &[1, 0, 2]);
    }
}
