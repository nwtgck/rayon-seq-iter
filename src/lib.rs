use rayon::prelude::*;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::sync::mpsc;


// (base: https://users.rust-lang.org/t/parallel-work-collected-sequentially/13504/3)
#[derive(Debug)]
struct ReverseTuple< T>(usize, T);
impl< T> PartialEq for ReverseTuple< T> {
    fn eq(&self, o: &Self) -> bool { o.0.eq(&self.0) }
}
impl<T> Eq for ReverseTuple<T> {}
impl<T> PartialOrd for ReverseTuple<T> {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> { o.0.partial_cmp(&self.0) }
}
impl<T> Ord for ReverseTuple<T> {
    fn cmp(&self, o: &Self) -> Ordering { o.0.cmp(&self.0) }
}

pub struct IntoSeqIter<I> {
    iter: mpsc::IntoIter<ReverseTuple<I>>,
    idx: usize,
    heap: BinaryHeap<ReverseTuple<I>>
}

impl<I>  Iterator for IntoSeqIter<I> {
    type Item = I;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // NOTE: self.iter.nex() blocks because it is Receiver's iter()
            if let Some(reverse_tuple) = self.iter.next() {
                // Push to new element
                self.heap.push(reverse_tuple);
                // Get the youngest element
                if self.heap.peek().map(|x| x.0) == Some(self.idx) {
                    self.idx += 1;
                    break self.heap.pop().map(|x| x.1);
                }
            } else {
                self.idx += 1;
                break self.heap.pop().map(|x| x.1);
            }
        }
    }
}

pub fn into_seq_iter<I: Send + 'static, P: rayon::iter::IndexedParallelIterator<Item=I> + 'static>(par_iter: P) -> IntoSeqIter<I> {
    // TODO: 1 is OK?
    let (sender, receiver) = mpsc::sync_channel(1);

    rayon::spawn( move || {
        par_iter.enumerate().for_each(|(i, x)| {
            sender.send(ReverseTuple(i, x)).unwrap();
        });
    });

    IntoSeqIter {
        iter: receiver.into_iter(),
        idx: 0,
        heap: BinaryHeap::new()
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
