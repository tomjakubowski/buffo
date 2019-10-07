//! Implements "Buffo", a binary data format for arrays of UTF-8 strings.
//! Ideas for further work:
//!   * Store a hash map
//!   * Store types of data other than &str
//!   * Use a zero-sized type for reading / slicing buffo (akin to str/String)

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{convert::TryInto, io, mem::size_of};

/// StrArray buffo layout:
///
/// ```text
/// [index_count: u32, [(idx: u32, len: u32)], [data blob] : [u8]]
/// ```
/// Each `idx` is an offset into `[data blob]`
#[derive(Debug)]
pub struct Buffo(Vec<u8>);

// Constants for Buffo's layout
const INDEX_COUNT_SERIAL_SIZE: usize = size_of::<u32>();
const INDEX_ITEMS_BEGIN: usize = INDEX_COUNT_SERIAL_SIZE;
const INDEX_ITEM_SERIAL_SIZE: usize = 2 * size_of::<u32>();
// A delimiter isn't strictly necessary, but this one is nice because it provides C-string
// compatibility.
const DATA_DELIM: &[u8] = b"\0";

impl Buffo {
    pub fn str_array<'a, S, T>(strs: T) -> Buffo
    where
        T: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut index: Vec<IndexItem> = vec![];
        let mut data = vec![];
        for s in strs {
            let s: &str = s.as_ref();
            let bytes = s.as_bytes();
            let idx: u32 = data.len().try_into().expect("too much data");
            data.extend_from_slice(bytes);
            data.extend_from_slice(DATA_DELIM);
            let len = (bytes.len() + DATA_DELIM.len())
                .try_into()
                .expect("string too long");
            index.push(IndexItem { idx, len });
        }

        let mut output = io::Cursor::new(vec![]);
        write_buffo(BuffoParts { index, data }, &mut output).unwrap();
        Buffo(output.into_inner())
    }

    pub fn nth_str(&self, i: u32) -> Option<&str> {
        let item = self.nth_index_item(i)?;
        let datum_idx = item.idx as usize;
        let datum_len = item.len as usize;
        let str_len = datum_len - DATA_DELIM.len();

        // NB: malicious index data (i.e. with a bad idx or len) could make this panic for OOB
        // access
        let datum: &[u8] = &self.data_blob()[datum_idx..datum_idx + str_len];
        std::str::from_utf8(datum).ok()
    }

    pub fn iter_strs(&self) -> impl Iterator<Item = &str> {
        (0..self.index_count()).map(move |i| self.nth_str(i).unwrap())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    fn index_count(&self) -> u32 {
        self.as_bytes().read_u32::<LittleEndian>().unwrap()
    }

    fn nth_index_item(&self, i: u32) -> Option<IndexItem> {
        if i >= self.index_count() {
            return None;
        }
        let pos = i as usize * INDEX_ITEM_SERIAL_SIZE;
        let serialized_item = &self.index_blob()[pos..pos + INDEX_ITEM_SERIAL_SIZE];
        let item = IndexItem::buffo_read(serialized_item).expect("buffo_read error");
        Some(item)
    }

    fn index_blob(&self) -> &[u8] {
        let count = self.index_count() as usize;
        let blob_len = INDEX_ITEM_SERIAL_SIZE * count;
        let begin = INDEX_ITEMS_BEGIN;
        &self.as_bytes()[begin..begin + blob_len]
    }

    fn data_blob(&self) -> &[u8] {
        let count = self.index_count() as usize;
        let data_start = INDEX_ITEMS_BEGIN + count * INDEX_ITEM_SERIAL_SIZE;
        &self.as_bytes()[data_start..]
    }
}

struct BuffoParts {
    index: Vec<IndexItem>,
    data: Vec<u8>,
}

struct IndexItem {
    idx: u32,
    len: u32, // includes NUL-terminator
}

impl IndexItem {
    fn buffo_write<W>(&self, mut wr: W) -> io::Result<()>
    where
        W: io::Write,
    {
        wr.write_u32::<LittleEndian>(self.idx)?;
        wr.write_u32::<LittleEndian>(self.len)?;
        Ok(())
    }

    fn buffo_read<R>(mut r: R) -> io::Result<IndexItem>
    where
        R: io::Read,
    {
        let idx = r.read_u32::<LittleEndian>()?;
        let len = r.read_u32::<LittleEndian>()?;
        Ok(IndexItem { idx, len })
    }
}

// StrArray buffo layout:
// [index_count: u32, [(idx: u32, len: u32)], [data blob]]
// Each idx is an offset into [data blob]
fn write_buffo<W>(parts: BuffoParts, mut cursor: W) -> io::Result<()>
where
    W: io::Write,
{
    let index_count: u32 = parts
        .index
        .len()
        .try_into()
        .expect("Too many items for buffo");
    cursor.write_u32::<LittleEndian>(index_count)?;
    for x in parts.index {
        x.buffo_write(&mut cursor)?;
        // Sanity check bounds
        let x_end = x.idx as usize + x.len as usize;
        assert!(x_end <= parts.data.len())
    }
    cursor.write_all(&parts.data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // debug printing function
    fn xxd(xs: &[u8]) {
        for (i, x) in xs.iter().enumerate() {
            print!("{:02x}", x);
            if i % 4 == 3 {
                println!("");
            }
        }
    }

    #[test]
    fn test_index_item_size() {
        let mut out = vec![];
        let item = IndexItem { idx: 2, len: 9000 };
        item.buffo_write(&mut out).unwrap();
        assert_eq!(INDEX_ITEM_SERIAL_SIZE, out.len());
    }

    #[test]
    fn test_trivial_buffo() {
        let input: Vec<&str> = vec![];
        let buffo = Buffo::str_array(input);
        assert_eq!(None, buffo.nth_str(0));
    }

    #[test]
    fn test_roundtrip() {
        let input = vec!["Foo", "Bar", "Hello world"];
        let buffo = Buffo::str_array(input);
        xxd(buffo.as_bytes());
        assert_eq!("Foo", buffo.nth_str(0).unwrap());
        assert_eq!("Bar", buffo.nth_str(1).unwrap());
        assert_eq!("Hello world", buffo.nth_str(2).unwrap());
        assert_eq!(None, buffo.nth_str(3));
    }

    #[test]
    fn test_collect() {
        let input = vec!["Foo", "Bar", "Hello world"];
        let buffo = Buffo::str_array(&input);
        xxd(buffo.as_bytes());
        let output: Vec<&str> = buffo.iter_strs().collect();
        assert_eq!(input, output);
    }
}
