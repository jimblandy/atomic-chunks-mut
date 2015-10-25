extern crate crossbeam;
use crossbeam::scope;

mod atomic_counter {
    use crossbeam::scope;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn bang_on_counter() {
        let n = AtomicUsize::new(0);

        scope(|scope| {
            for _ in 0..100 {
                scope.spawn(|| {
                    for _ in 0..100_000 {
                        n.fetch_add(1, Ordering::Relaxed);
                    }
                });
            }
        });

        assert_eq!(n.load(Ordering::SeqCst), 10_000_000);
    }
}

#[cfg(test)]
mod atomic_iterator {
    use crossbeam::scope;
    mod counter {
        use std::sync::atomic::{AtomicUsize, Ordering};
        pub struct Counter {
            count: AtomicUsize
        }

        impl Counter {
            pub fn new(count: usize) -> Counter {
                Counter { count: AtomicUsize::new(count) }
            }

            fn next(&self) -> Option<usize> {
                let mut current;
                loop {
                    current = self.count.load(Ordering::SeqCst);
                    if current == 0 {
                        return None;
                    }
                    if self.count.compare_and_swap(current, current - 1, Ordering::SeqCst) == current {
                        return Some(current - 1);
                    }
                }
            }
        }

        impl<'a> Iterator for &'a Counter {
            type Item = usize;
            fn next(&mut self) -> Option<usize> { (*self).next() }
        }
    }

    #[test]
    fn test_ai() {
        for _ in 0..100 {
            let c = counter::Counter::new(10000);
            let mut threads = vec![];
            scope(|scope| {
                for _ in 0..100 {
                    threads.push(scope.spawn(|| { c.collect::<Vec<_>>() }));
                }
            });

            let mut seen = [false; 10000];
            for thread in threads {
                for i in thread.join() {
                    assert!(!seen[i]);
                    seen[i] = true;
                }
            }
        }
    }
}

mod atomic_chunks_mut {
    use crossbeam::scope;
    use std;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub struct AtomicChunksMut<'a, T: 'a> {
        slice: &'a [T],
        step: usize,
        next: AtomicUsize
    }

    impl<'a, T> AtomicChunksMut<'a, T> {
        pub fn new(slice: &'a mut [T], step: usize) -> AtomicChunksMut<'a, T> {
            AtomicChunksMut {
                slice: slice,
                step: step,
                next: AtomicUsize::new(0)
            }
        }

        #[allow(mutable_transmutes)]
        unsafe fn next(&self) -> Option<&'a mut [T]> {
            let mut current;
            loop {
                current = self.next.load(Ordering::SeqCst);
                assert!(current <= self.slice.len());
                if current == self.slice.len() {
                    return None;
                }
                let end = std::cmp::min(current + self.step, self.slice.len());
                if self.next.compare_and_swap(current, end, Ordering::SeqCst) == current {
                    return Some(std::mem::transmute(&self.slice[current..end]));
                }
            }
        }
    }

    impl<'a, 'b, T> Iterator for &'b AtomicChunksMut<'a, T> {
        type Item = &'a mut [T];
        fn next(&mut self) -> Option<Self::Item> { unsafe { (*self).next() } }
    }
}

pub use atomic_chunks_mut::AtomicChunksMut;

#[test]
fn test_ait() {
    let mut v = vec![0,1,2,3,4,5,6,7,8,9,10];
    let c : Vec<_> = (&AtomicChunksMut::new(&mut v[..], 3)).collect();

    let v2 = vec![0,1,2,3,4,5,6,7,8,9,10];
    assert_eq!(c, vec![&v2[0..3], &v2[3..6], &v2[6..9], &v2[9..11]]);
}

#[test]
fn stress_test_ait() {
    let mut v : Vec<usize> = (0..10000).collect();
    let it = AtomicChunksMut::new(&mut v[..], 3);

    scope(|scope| {
        let mut threads = vec![];
        for _ in 0..10 {
            threads.push(scope.spawn(|| {
                let mut v = vec![];
                for chunk in &it { v.push(chunk[0]); }
                v
            }));
        }

        let mut seen = vec![false; 10000];
        for thread in threads {
            for first in thread.join() {
                assert!(first % 3 == 0);
                assert!(!seen[first]);
                seen[first] = true;
            }
        }
    });
}

fn main() {
    println!("Hello, world!");
}