use rayon::prelude::*;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};


// (base: https://users.rust-lang.org/t/parallel-work-collected-sequentially/13504/3)
#[derive(Debug)]
struct ReverseTuple< T>(usize, T);
impl< T> PartialEq for ReverseTuple< T> {
    fn eq(&self, o: &Self) -> bool { o.0.eq(&self.0) }
}
impl< T> Eq for ReverseTuple< T> {}
impl< T> PartialOrd for ReverseTuple< T> {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> { o.0.partial_cmp(&self.0) }
}
impl< T> Ord for ReverseTuple< T> {
    fn cmp(&self, o: &Self) -> Ordering { o.0.cmp(&self.0) }
}

pub struct IntoSeqIter<I: Sync + Send> {
    iter: mpsc::IntoIter<I>
}

impl <I: Sync + Send>  Iterator for IntoSeqIter<I> {
    type Item = I;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

pub fn into_seq_iter<I: Sync + Send + 'static, P: rayon::iter::IndexedParallelIterator<Item=I> + Sync + 'static>(par_iter: P) -> IntoSeqIter<I> {
    // TODO: 1 is OK?
    let (sender, receiver) = mpsc::sync_channel(1);
    let heap = Arc::new(Mutex::new(BinaryHeap::new()));
    let idx = Arc::new(Mutex::new(0));

    let heap_clone = Arc::clone(&heap);
    let index_clone = Arc::clone(&idx);
    rayon::spawn( move || {
        par_iter.enumerate().for_each(|(i, x)| {
            let mut heap_guard = heap_clone.lock().unwrap();
            let mut idx_guard = index_clone.lock().unwrap();
            heap_guard.push(ReverseTuple(i, x));
            loop {
                if let Some(i) = heap_guard.peek().map(|r| r.0) {
                    if i == *idx_guard {
                        let x = heap_guard.pop().unwrap().1;
                        sender.send(x).unwrap();
                        *idx_guard += 1;
                    } else {
                        break
                    }
                } else {
                    break;
                }
            }
        });
    });

    IntoSeqIter {
        iter: receiver.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_be_sequential() {
        let par_iter = (10..20).collect::<Vec<i32>>().into_par_iter().map(|x| x * 2);
        let vec: Vec<_> = into_seq_iter(par_iter).collect();
        assert_eq!(vec, vec![20, 22, 24, 26, 28, 30, 32, 34, 36, 38]);
    }
}
