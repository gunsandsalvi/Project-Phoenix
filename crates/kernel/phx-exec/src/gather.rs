use phx_num::violation;

use crate::consts::GATHER_SERIAL_BELOW;
use crate::pool::Pool;

/// Intents per (chunk, handler), kept from day to day so their capacity is reused.
#[derive(Debug)]
pub struct IntentBuf<T> {
    handlers: usize,
    bufs: Vec<Vec<T>>,
}

impl<T: Copy + Send + Sync> IntentBuf<T> {
    /// Buffers for `handlers` handlers per chunk.
    #[must_use]
    pub fn new(handlers: usize) -> IntentBuf<T> {
        if handlers == 0 {
            violation!(clause = "TIME.6", "intent buffers for no handler");
        }
        IntentBuf { handlers, bufs: Vec::new() }
    }

    /// Empties every buffer, keeping its capacity, and sizes the set for `chunks` chunks.
    pub fn reset(&mut self, chunks: usize) {
        self.bufs.iter_mut().for_each(Vec::clear);
        self.bufs.resize_with(chunks * self.handlers, Vec::new);
    }

    /// Each chunk's buffers, one per handler, which the chunk's task owns alone.
    pub fn chunks_mut(&mut self) -> impl Iterator<Item = &mut [Vec<T>]> {
        self.bufs.chunks_mut(self.handlers)
    }

    /// Intents held across every buffer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bufs.iter().map(Vec::len).sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bufs.iter().all(Vec::is_empty)
    }
}

/// Every intent, in (chunk, handler) order and each buffer's own order, placed by prefix sum; the result never
/// depends on which worker filled which buffer or when.
pub fn gather<T: Copy + Send + Sync>(pool: Option<&Pool>, intents: &IntentBuf<T>, out: &mut Vec<T>) {
    out.clear();
    let total = intents.len();
    let Some(first) = intents.bufs.iter().find_map(|b| b.first().copied()) else {
        return;
    };
    out.resize(total, first);
    let Some(pool) = pool.filter(|_| total >= GATHER_SERIAL_BELOW) else {
        out.clear();
        intents.bufs.iter().for_each(|b| out.extend_from_slice(b));
        return;
    };
    // One task per chunk, copying that chunk's buffers into their places in turn.
    let mut rest = out.as_mut_slice();
    let mut chunks: Vec<Vec<(&mut [T], &[T])>> = Vec::with_capacity(intents.bufs.len() / intents.handlers);
    for chunk in intents.bufs.chunks(intents.handlers) {
        let mut pieces = Vec::with_capacity(chunk.len());
        for b in chunk {
            let (dst, tail) = std::mem::take(&mut rest).split_at_mut(b.len());
            pieces.push((dst, b.as_slice()));
            rest = tail;
        }
        chunks.push(pieces);
    }
    pool.for_each(chunks, |pieces| {
        for (dst, src) in pieces {
            dst.copy_from_slice(src);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{IntentBuf, gather};
    use crate::pool::Pool;
    use crate::spec::PoolSpec;

    #[test]
    fn gather_order_is_canonical() {
        let chunks = 40;
        let mut buf: IntentBuf<(u32, u32, u32)> = IntentBuf::new(3);
        buf.reset(chunks);
        // Filled in a scrambled order of chunks, as workers finishing out of turn would.
        let mut order: Vec<usize> = (0..chunks).collect();
        order.reverse();
        order.swap(3, 17);
        let mut views: Vec<_> = buf.chunks_mut().enumerate().collect();
        for c in order {
            let (chunk, handlers) = &mut views[c];
            for (h, b) in handlers.iter_mut().enumerate() {
                for k in 0..(*chunk + h) % 5 * 3000 {
                    b.push((u32::try_from(*chunk).unwrap(), u32::try_from(h).unwrap(), u32::try_from(k).unwrap()));
                }
            }
        }
        for workers in [1, 8] {
            let pool = Pool::new(&PoolSpec::unpinned(workers)).unwrap();
            let mut out = Vec::new();
            gather(Some(&pool), &buf, &mut out);
            assert!(out.windows(2).all(|w| w[0] < w[1]), "(chunk, handler, position) order");
            assert_eq!(out.len(), buf.len());
        }
        buf.reset(chunks);
        assert!(buf.is_empty());
    }
}
