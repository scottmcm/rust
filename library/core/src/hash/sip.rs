//! An implementation of SipHash.

#![allow(deprecated)] // the types in this module are deprecated

use crate::marker::PhantomData;

use super::BufferedHasher;

/// An implementation of SipHash 1-3.
///
/// This is currently the default hashing function used by standard library
/// (e.g., `collections::HashMap` uses it by default).
///
/// See: <https://131002.net/siphash>
#[unstable(feature = "hashmap_internals", issue = "none")]
#[rustc_deprecated(
    since = "1.13.0",
    reason = "use `std::collections::hash_map::DefaultHasher` instead"
)]
#[derive(Debug, Clone, Default)]
#[doc(hidden)]
pub struct SipHasher13 {
    hasher: BufferedHasher<8, ChunkHasher<Sip13Rounds>>,
}

/// An implementation of SipHash 2-4.
///
/// See: <https://131002.net/siphash/>
#[unstable(feature = "hashmap_internals", issue = "none")]
#[rustc_deprecated(
    since = "1.13.0",
    reason = "use `std::collections::hash_map::DefaultHasher` instead"
)]
#[derive(Debug, Clone, Default)]
struct SipHasher24 {
    hasher: BufferedHasher<8, ChunkHasher<Sip24Rounds>>,
}

/// An implementation of SipHash 2-4.
///
/// See: <https://131002.net/siphash/>
///
/// SipHash is a general-purpose hashing function: it runs at a good
/// speed (competitive with Spooky and City) and permits strong _keyed_
/// hashing. This lets you key your hash tables from a strong RNG, such as
/// [`rand::os::OsRng`](https://doc.rust-lang.org/rand/rand/os/struct.OsRng.html).
///
/// Although the SipHash algorithm is considered to be generally strong,
/// it is not intended for cryptographic purposes. As such, all
/// cryptographic uses of this implementation are _strongly discouraged_.
#[stable(feature = "rust1", since = "1.0.0")]
#[rustc_deprecated(
    since = "1.13.0",
    reason = "use `std::collections::hash_map::DefaultHasher` instead"
)]
#[derive(Debug, Clone, Default)]
pub struct SipHasher(SipHasher24);

#[derive(Debug)]
struct ChunkHasher<S: Sip> {
    chunks: usize, // how many chunks we've processed
    state: State,  // hash State
    _marker: PhantomData<S>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct State {
    // v0, v2 and v1, v3 show up in pairs in the algorithm,
    // and simd implementations of SipHash will use vectors
    // of v02 and v13. By placing them in this order in the struct,
    // the compiler can pick up on just a few simd optimizations by itself.
    v0: u64,
    v2: u64,
    v1: u64,
    v3: u64,
}

impl State {
    #[inline]
    fn new(k0: u64, k1: u64) -> Self {
        let mut state = State { v0: 0, v1: 0, v2: 0, v3: 0 };
        state.v0 = k0 ^ 0x736f6d6570736575;
        state.v1 = k1 ^ 0x646f72616e646f6d;
        state.v2 = k0 ^ 0x6c7967656e657261;
        state.v3 = k1 ^ 0x7465646279746573;
        state
    }
}

macro_rules! compress {
    ($state:expr) => {{ compress!($state.v0, $state.v1, $state.v2, $state.v3) }};
    ($v0:expr, $v1:expr, $v2:expr, $v3:expr) => {{
        $v0 = $v0.wrapping_add($v1);
        $v1 = $v1.rotate_left(13);
        $v1 ^= $v0;
        $v0 = $v0.rotate_left(32);
        $v2 = $v2.wrapping_add($v3);
        $v3 = $v3.rotate_left(16);
        $v3 ^= $v2;
        $v0 = $v0.wrapping_add($v3);
        $v3 = $v3.rotate_left(21);
        $v3 ^= $v0;
        $v2 = $v2.wrapping_add($v1);
        $v1 = $v1.rotate_left(17);
        $v1 ^= $v2;
        $v2 = $v2.rotate_left(32);
    }};
}

impl SipHasher {
    /// Creates a new `SipHasher` with the two initial keys set to 0.
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    #[rustc_deprecated(
        since = "1.13.0",
        reason = "use `std::collections::hash_map::DefaultHasher` instead"
    )]
    #[must_use]
    pub fn new() -> SipHasher {
        SipHasher::new_with_keys(0, 0)
    }

    /// Creates a `SipHasher` that is keyed off the provided keys.
    #[inline]
    #[stable(feature = "rust1", since = "1.0.0")]
    #[rustc_deprecated(
        since = "1.13.0",
        reason = "use `std::collections::hash_map::DefaultHasher` instead"
    )]
    #[must_use]
    pub fn new_with_keys(key0: u64, key1: u64) -> SipHasher {
        let chunk_hasher = ChunkHasher::new_with_keys(key0, key1);
        SipHasher(SipHasher24 { hasher: BufferedHasher::new(chunk_hasher) })
    }
}

impl SipHasher13 {
    /// Creates a new `SipHasher13` with the two initial keys set to 0.
    #[inline]
    #[unstable(feature = "hashmap_internals", issue = "none")]
    #[rustc_deprecated(
        since = "1.13.0",
        reason = "use `std::collections::hash_map::DefaultHasher` instead"
    )]
    pub fn new() -> SipHasher13 {
        SipHasher13::new_with_keys(0, 0)
    }

    /// Creates a `SipHasher13` that is keyed off the provided keys.
    #[inline]
    #[unstable(feature = "hashmap_internals", issue = "none")]
    #[rustc_deprecated(
        since = "1.13.0",
        reason = "use `std::collections::hash_map::DefaultHasher` instead"
    )]
    pub fn new_with_keys(key0: u64, key1: u64) -> SipHasher13 {
        let chunk_hasher = ChunkHasher::new_with_keys(key0, key1);
        SipHasher13 { hasher: BufferedHasher::new(chunk_hasher) }
    }
}

impl<S: Sip> ChunkHasher<S> {
    #[inline]
    fn new_with_keys(key0: u64, key1: u64) -> ChunkHasher<S> {
        let state = ChunkHasher {
            chunks: 0,
            state: State::new(key0, key1),
            _marker: PhantomData,
        };
        state
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl super::Hasher for SipHasher {
    #[inline]
    fn write(&mut self, msg: &[u8]) {
        self.0.hasher.write(msg)
    }

    #[inline]
    fn write_str(&mut self, s: &str) {
        self.0.hasher.write_str(s);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0.hasher.finish()
    }
}

#[unstable(feature = "hashmap_internals", issue = "none")]
impl super::Hasher for SipHasher13 {
    #[inline]
    fn write(&mut self, msg: &[u8]) {
        self.hasher.write(msg)
    }

    #[inline]
    fn write_str(&mut self, s: &str) {
        self.hasher.write_str(s);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hasher.finish()
    }
}

impl<S: Sip> super::ChunkHasher<8> for ChunkHasher<S> {
    #[inline]
    fn write_chunk(&mut self, chunk: [u8; 8]) {
        self.chunks += 1;
        let mi = u64::from_le_bytes(chunk);

        self.state.v3 ^= mi;
        S::c_rounds(&mut self.state);
        self.state.v0 ^= mi;
    }

    #[inline]
    fn finish(&self, tail_len: usize, mut tail_chunk: [u8; 8]) -> u64 {
        assert!(tail_len < 8);
        debug_assert!(tail_chunk[tail_len..].iter().all(|x| *x == 0));

        let mut state = self.state;

        // Because this isn't a full chunk, the last byte is always zero
        debug_assert_eq!(tail_chunk[7], 0);
        // And thus sip can put the (low byte of the) length in there.
        // It's cheaper to bit-or it in than to directly assign,
        // as that way the compiler knows it doesn't have to mask the rest.
        let length = self.chunks * 8 + tail_len;
        tail_chunk[7] |= length as u8;
        let b = u64::from_le_bytes(tail_chunk);

        state.v3 ^= b;
        S::c_rounds(&mut state);
        state.v0 ^= b;

        state.v2 ^= 0xff;
        S::d_rounds(&mut state);

        state.v0 ^ state.v1 ^ state.v2 ^ state.v3
    }
}

impl<S: Sip> Clone for ChunkHasher<S> {
    #[inline]
    fn clone(&self) -> ChunkHasher<S> {
        ChunkHasher {
            chunks: self.chunks,
            state: self.state,
            _marker: self._marker,
        }
    }
}

impl<S: Sip> Default for ChunkHasher<S> {
    /// Creates a `ChunkHasher<S>` with the two initial keys set to 0.
    #[inline]
    fn default() -> ChunkHasher<S> {
        ChunkHasher::new_with_keys(0, 0)
    }
}

#[doc(hidden)]
trait Sip {
    fn c_rounds(_: &mut State);
    fn d_rounds(_: &mut State);
}

#[derive(Debug, Clone, Default)]
struct Sip13Rounds;

impl Sip for Sip13Rounds {
    #[inline]
    fn c_rounds(state: &mut State) {
        compress!(state);
    }

    #[inline]
    fn d_rounds(state: &mut State) {
        compress!(state);
        compress!(state);
        compress!(state);
    }
}

#[derive(Debug, Clone, Default)]
struct Sip24Rounds;

impl Sip for Sip24Rounds {
    #[inline]
    fn c_rounds(state: &mut State) {
        compress!(state);
        compress!(state);
    }

    #[inline]
    fn d_rounds(state: &mut State) {
        compress!(state);
        compress!(state);
        compress!(state);
        compress!(state);
    }
}
