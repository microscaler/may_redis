//! Non-blocking write to the TCP socket.
//!
//! Flushes buffered bytes to the underlying socket without blocking,
//! advancing the write buffer past written bytes.

use bytes::{Buf, BytesMut};
use std::io;

/// Write the front of `write_buf` to the inner raw socket without
/// blocking.
///
/// Writes as many bytes as the socket will accept in one go and then
/// advances `write_buf` past them, so a follow-up call (after a
/// `wait_io` for `WRITABLE`) can pick up where this one left off.
///
/// # Return value
///
/// `Ok(n)` is the number of bytes actually written this call. `n` may
/// be less than `write_buf.len()` — that simply means the kernel
/// buffer is full and the caller should wait for the socket to become
/// writable again. `write_buf` is left containing exactly the unwritten
/// tail.
///
/// # Errors
///
/// Returns the underlying [`io::Error`] for any non-`WouldBlock`
/// failure. A write of 0 bytes is mapped to
/// [`io::ErrorKind::BrokenPipe`] so the caller treats it as fatal.
pub(super) fn nonblock_write<W: io::Write>(
    stream: &mut W,
    write_buf: &mut BytesMut,
) -> io::Result<usize> {
    let buf = write_buf.chunk();
    let len = buf.len();
    let mut write_cnt = 0;
    while write_cnt < len {
        // AC-4.1, FR-033: Use safe bounds-checked indexing instead of `get_unchecked`.
        // The `write_cnt <= len` loop invariant (checked by `while` condition) guarantees
        // this slice is always valid. `buf[write_cnt..]` returns a subslice of `buf`
        // that is at most `buf.len() - write_cnt` bytes — never past the end.
        match stream.write(&buf[write_cnt..]) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed")),
            Ok(n) => write_cnt += n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e),
        }
    }
    write_buf.advance(write_cnt);
    Ok(write_cnt)
}
