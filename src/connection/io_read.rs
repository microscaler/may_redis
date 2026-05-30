//! Non-blocking read from the TCP socket.
//!
//! Reads raw bytes from the underlying socket into a `BytesMut` buffer,
//! returning whether the read was blocked or the buffer was filled.

use bytes::{BufMut, BytesMut};
use std::io;

/// Read from the inner raw socket into a `BytesMut` buffer.
///
/// # Return value (critical — do not discard)
///
/// - `Ok(true)` — the read was **blocked**: the socket returned
///   `WouldBlock` before `read_buf` was filled. The caller MUST wait
///   for the next epoll readable event (via `stream.wait_io()`)
///   before reading again, otherwise the connection loop will
///   busy-spin and never yield to other coroutines.
/// - `Ok(false)` — the read filled `read_buf` completely. More data
///   may still be available in the kernel buffer, so the caller
///   should re-read immediately without going through epoll.
///
/// # Errors
///
/// Returns the underlying [`io::Error`] for any non-`WouldBlock`
/// failure. A clean half-close (read of 0 bytes) is mapped to
/// [`io::ErrorKind::BrokenPipe`] so the caller treats it as a fatal
/// connection error.
///
/// # History
///
/// The whole reason the return value is documented this loudly is
/// Bug 1 in `llmwiki/topics/connection-loop-pitfalls.md`: the
/// connection loop used to discard this `bool` and hardcode
/// `read_blocked = false`, which caused every integration test to
/// hang. Treat the return value as load-bearing.
pub(super) fn nonblock_read<R: io::Read>(
    stream: &mut R,
    read_buf: &mut BytesMut,
) -> io::Result<bool> {
    // SAFETY: `BytesMut::chunk_mut()` returns a `&mut [u8]` with capacity equal to
    // `remaining_capacity()`. The buffer is uninitialized but guaranteed to hold at
    // least `len` bytes. `stream.read()` writes up to `len` bytes into this space.
    // After `read_cnt` bytes are read, `read_buf.advance_mut(read_cnt)` marks them as
    // initialized. The raw pointer cast is valid because `chunk_mut()` returns a slice
    // with properly initialized capacity metadata.
    let buf: &mut [u8] = unsafe { &mut *(read_buf.chunk_mut() as *mut _ as *mut [u8]) };
    let len = buf.len();
    let mut read_cnt = 0;
    while read_cnt < len {
        match stream.read(unsafe {
            // SAFETY: The `while read_cnt < len` loop invariant guarantees
            // `read_cnt <= buf.len()`. After each successful read, `read_cnt`
            // increases, but never exceeds `len`. Therefore `read_cnt..` is always a
            // valid subslice of `buf`.
            buf.get_unchecked_mut(read_cnt..)
        }) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed")),
            Ok(n) => read_cnt += n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e),
        }
    }
    // SAFETY: `stream.read()` wrote exactly `read_cnt` bytes into the uninitialized
    // buffer starting at position 0. `advance_mut(read_cnt)` transitions those
    // `read_cnt` bytes from uninitialized to initialized. This is correct because
    // `stream.read()` only writes into the capacity portion returned by `chunk_mut()`.
    unsafe { read_buf.advance_mut(read_cnt) };
    Ok(read_cnt < len)
}
