use crate::intrinsics::assume;

#[derive(Debug, Clone)]
pub(super) struct BufferedHasher<const N: usize, H> {
    // Correctness invariant: buffer[len_unchecked..] are all zero.
    buffer: [u8; N],
    // Safety invariant: `len_unchecked < N`.
    len_unchecked: usize,
    chunk_hasher: H,
}

impl<const N: usize, H: Default> Default for BufferedHasher<N, H> {
    fn default() -> Self {
        BufferedHasher::new(H::default())
    }
}

impl<const N: usize, H> BufferedHasher<N, H> {
    pub fn new(chunk_hasher: H) -> Self {
        BufferedHasher { buffer: [0; N], len_unchecked: 0, chunk_hasher }
    }
}

use details::BufferTools;
mod details {
    pub trait BufferTools {
        fn merge_in(&mut self, other: Self);
        fn shift_array_left(self, n: usize) -> Self;
        fn shift_array_right(self, n: usize) -> Self;
    }
}
// Yes, these "backwards" shifts are correct.  Once llvm-project#55327 is fixed
// we could switch to `{to|from}_be_bytes` for improved clarity.
// Or, even better, find a way to write these generically.
impl BufferTools for [u8; 4] {
    #[inline]
    fn merge_in(&mut self, other: Self) {
        *self = (u32::from_ne_bytes(*self) | u32::from_ne_bytes(other)).to_ne_bytes()
    }
    #[inline]
    fn shift_array_left(self, n: usize) -> Self {
        debug_assert!(n < 4);
        (u32::from_le_bytes(self) >> (n * 8)).to_le_bytes()
    }
    #[inline]
    fn shift_array_right(self, n: usize) -> Self {
        debug_assert!(n < 4);
        (u32::from_le_bytes(self) << (n * 8)).to_le_bytes()
    }
}
impl BufferTools for [u8; 8] {
    #[inline]
    fn merge_in(&mut self, other: Self) {
        *self = (u64::from_ne_bytes(*self) | u64::from_ne_bytes(other)).to_ne_bytes()
    }
    #[inline]
    fn shift_array_left(self, n: usize) -> Self {
        debug_assert!(n < 8);
        (u64::from_le_bytes(self) >> (n * 8)).to_le_bytes()
    }
    #[inline]
    fn shift_array_right(self, n: usize) -> Self {
        debug_assert!(n < 8);
        (u64::from_le_bytes(self) << (n * 8)).to_le_bytes()
    }
}
impl BufferTools for [u8; 16] {
    #[inline]
    fn merge_in(&mut self, other: Self) {
        *self = (u128::from_ne_bytes(*self) | u128::from_ne_bytes(other)).to_ne_bytes()
    }
    #[inline]
    fn shift_array_left(self, n: usize) -> Self {
        debug_assert!(n < 16);
        (u128::from_le_bytes(self) >> (n * 8)).to_le_bytes()
    }
    #[inline]
    fn shift_array_right(self, n: usize) -> Self {
        debug_assert!(n < 16);
        (u128::from_le_bytes(self) << (n * 8)).to_le_bytes()
    }
}

#[inline]
fn short_copy<const MAX: usize>(target: &mut [u8], source: &[u8], mut n: usize) {
    assert!(n <= MAX);

    let (mut target, mut source) = (&mut target[..n], &source[..n]);

    // LLVM doesn't simplify calls to memcpy unless the length is known exactly.
    // So since we know this is always short, emit it as a sequence of known
    // power-of-two-length copies, for which it can just use loads/stores.
    for i in (0..MAX.log2() + 1).rev() {
        let part = 1_usize << i;
        if n >= part {
            n -= part;
            target[n..].copy_from_slice(&source[n..]);
            target = &mut target[..n];
            source = &source[..n];
        }
    }
}

impl<const N: usize, H> BufferedHasher<N, H>
where
    H: super::ChunkHasher<N>,
    [u8; N]: BufferTools,
{
    #[inline]
    fn len(&self) -> usize {
        debug_assert!(self.len_unchecked < N);

        let len = self.len_unchecked;
        // SAFETY: guaranteed by type invariant.
        // (This allows LLVM to remove various checks later.)
        unsafe { assume(len < N) };
        len
    }

    #[inline]
    fn set_len(&mut self, new_len: usize) {
        assert!(new_len < N);

        // SAFETY: we just checked the invariant
        self.len_unchecked = new_len;
    }

    /// Simple version, when there's at most one chunk to hash.
    #[inline]
    fn write_short(&mut self, bytes: &[u8]) {
        assert!(bytes.len() <= N);

        let new = bytes.len();
        let len = self.len();

        let mut padded = [0; N];
        short_copy::<N>(&mut padded, bytes, new);
        let bytes = padded;

        self.buffer.merge_in(bytes.shift_array_right(len));

        let total = len + new;
        if total < N {
            self.set_len(total);
        } else {
            self.chunk_hasher.write_chunk(self.buffer);

            let extra = total - N;
            if extra == 0 {
                self.buffer = [0; N];
            } else {
                self.buffer = bytes.shift_array_left(N - len);
            }
            self.set_len(extra);
        }
    }

    /// Complex version, where we can always do N-byte reads.
    #[inline]
    fn write_long(&mut self, mut bytes: &[u8]) {
        assert!(bytes.len() > N);

        // Because we know `bytes` is long enough, we can read full chunks.
        // That reads more than we might need, but it's actually more efficient
        // than doing shorter, dynamic-length reads.  We'll shift them later
        // to ignore the bytes that are handled by the full chunks loop.
        let (first_chunk, _) = bytes.split_array_ref();
        let first_chunk: [u8; N] = *first_chunk;
        let (_, last_chunk) = bytes.rsplit_array_ref();
        let last_chunk: [u8; N] = *last_chunk;

        let len = self.len();
        if len != 0 {
            self.buffer.merge_in(first_chunk.shift_array_right(len));
            self.chunk_hasher.write_chunk(self.buffer);
            // SAFETY: We asserted that `bytes` is at least `N` long,
            // and `len < N` by invariant, which means that the subtraction
            // cannot overflow, and thus the split must be in-range.
            // (LLVM ought to know this, since we have it an `assume`
            //  about the invariant, but seemingly it doesn't.)
            (_, bytes) = unsafe { bytes.split_at_unchecked(N - len) };
        }

        let (chunks, tail) = bytes.as_chunks();
        for chunk in chunks {
            self.chunk_hasher.write_chunk(*chunk);
        }

        let extra = tail.len();
        if extra == 0 {
            self.buffer = [0; N];
        } else {
            self.buffer = last_chunk.shift_array_left(N - extra);
        }
        self.set_len(extra);
    }
}

impl<const N: usize, H> super::Hasher for BufferedHasher<N, H>
where
    H: super::ChunkHasher<N>,
    [u8; N]: BufferTools,
{
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        debug_assert_eq!(
            self.buffer.shift_array_left(self.len_unchecked), [0; N],
            "set/zero: {:?}", self.buffer.split_at(self.len_unchecked),
        );

        // Most of the `write_*` calls pass a slice of known length,
        // so this check often optimizes away entirely.
        if bytes.len() <= N {
            self.write_short(bytes);
        } else {
            self.write_long(bytes);
        }
    }

    #[inline]
    fn write_str(&mut self, s: &str) {
        // This hasher works byte-wise, and `0xFF` cannot show up in a `str`,
        // so just hashing the one extra byte is enough to be prefix-free.
        self.write(s.as_bytes());
        self.write_u8(0xFF);
    }

    fn finish(&self) -> u64 {
        debug_assert_eq!(
            self.buffer.shift_array_left(self.len_unchecked), [0; N],
            "set/zero: {:?}", self.buffer.split_at(self.len_unchecked),
        );

        let len = self.len();
        self.chunk_hasher.finish(len, self.buffer)
    }
}
