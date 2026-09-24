// The derive names the storable marker by this crate's path, which must resolve inside the crate too.
extern crate self as phx_store;

pub mod arena;
pub mod backing;
pub mod block_list;
pub mod column;
pub mod consts;
mod convert;
pub mod descriptor;
pub mod encode;
pub mod hash;
pub mod pod;
pub mod region;
pub mod save;
mod save_values;
pub mod table;

pub use arena::{ArenaLists, CellListRef, CellLists, ChunkArena, ListRef, move_list};
pub use backing::{AddressSpace, Backing, HeapBacking, MmapBacking, SystemBacking};
pub use block_list::{BlockBag, BlockList, BlockPool};
pub use column::{ChunkMut, Column};
pub use descriptor::{ColumnDescriptor, FieldDescriptor, FieldTag, Transform};
pub use encode::{DecodeError, decode_column, decode_rows, encode_column, encode_rows, rows_in};
pub use hash::{IdentityHasher, LogicalHasher, Sip128, SlotIdentity};
pub use pod::{__seal, Pod, as_bytes, as_bytes_mut, from_bytes};
pub use region::Region;
pub use save::{LoadError, Reader, Saved, Writer, hash_saved, narrow};
pub use table::{SlotAlloc, Table, TableChunk, TableChunks};
