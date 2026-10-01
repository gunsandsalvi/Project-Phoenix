//! The stored log's declared numbers: its segments' and frames' formats and the bounds of a kind's frames.

/// A segment's first bytes, naming its format: "PHXS".
pub const SEGMENT_MAGIC: u32 = 0x5358_4850;
/// The segment format's version, raised with any change of its bytes.
pub const SEGMENT_VERSION: u16 = 1;
/// A segment header's bytes: its magic u32, its version u16, its first and last day u32 each.
pub const SEGMENT_HEADER: usize = 14;
/// A frame header's bytes: its entries' bytes u32, its entries u32, and its first key, two u64.
pub const FRAME_HEADER: usize = 24;
/// The least bytes a kind may declare a frame to hold.
pub const FRAME_LEAST: u32 = 1 << 12;
/// The most bytes a kind may declare a frame to hold.
pub const FRAME_MOST: u32 = 1 << 20;
/// The most words an item holds besides its key.
pub const ITEM_WORDS: usize = 16;
/// The bits of a value a varint's byte carries, its high bit saying more bytes follow.
pub const VARINT_BITS: u32 = 7;
/// A varint byte's continuation bit.
pub const VARINT_MORE: u8 = 0x80;
/// The most bytes a varint of a u64 takes.
pub const VARINT_MOST: usize = 10;
/// The most bytes one entry takes: its key's two words, its day and its item's words, each a varint.
pub const ENTRY_MOST: usize = (3 + ITEM_WORDS) * VARINT_MOST;
/// The key a segment's content is hashed under to its address.
pub const ADDRESS_KEY: [u64; 2] = [0x7068_782d_7265_636f, 0x7264_2d73_6567_6d74];
/// The characters of a segment's address written as its file's name, two a byte.
pub const ADDRESS_CHARS: usize = 32;
/// The digits a segment's address is written in.
pub const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";
/// The bits one digit of an address writes.
pub const NIBBLE_BITS: u32 = 4;
/// One digit's bits of an address.
pub const NIBBLE: u128 = 0xf;
