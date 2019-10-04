//! Implements "Buffo", a binary data format for arrays of UTF-8 strings.
//! Ideas for further work:
//!   * Store types of data other than &str
//!   * Use a zero-sized type for reading / slicing buffo (akin to str/String)
//!   * Property testing with proptest: https://github.com/AltSysrq/proptest

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{
    convert::TryInto,
    io::{self, Cursor, Seek, SeekFrom, Write},
    mem::size_of,
};

#[derive(Debug)]
/// StrArray buffo layout:
///
/// ```text
/// [index_count: u32, [(idx: u32, len: u32)], [data blob] : [u8]]
/// ```
/// Each `idx` is an offset into `[data blob]`
pub struct Buffo(Vec<u8>);

const INDEX_COUNT_SERIAL_SIZE: usize = size_of::<u32>();
const INDEX_ITEM_SERIAL_SIZE: usize = 2 * size_of::<u32>();

impl Buffo {
    pub fn str_array<'a, T>(strs: T) -> Buffo
    where
        T: IntoIterator<Item = &'a str>,
    {
        let mut index: Vec<IndexItem> = vec![];
        let mut data = vec![];
        for s in strs {
            // This datum starts after the last datum ended
            let idx: u32 = data.len().try_into().expect("too much data");
            data.extend_from_slice(s.as_bytes());
            data.push(0u8); // NUL-terminate for C string compatibility
            let len = (s.as_bytes().len() + 1)
                .try_into()
                .expect("string too long");
            index.push(IndexItem { idx, len });
        }

        let mut output = io::Cursor::new(vec![]);
        write_buffo(BuffoParts { index, data }, &mut output).unwrap();
        Buffo(output.into_inner())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn nth_str(&self, i: u32) -> Option<&str> {
        let mut cur = Cursor::new(self.as_bytes());
        let index_count = cur.read_u32::<LittleEndian>().unwrap();
        if i >= index_count {
            return None;
        }
        let item_idx: usize = (i as usize) * INDEX_ITEM_SERIAL_SIZE;
        cur.seek(SeekFrom::Current(item_idx as i64)).unwrap();

        // Read out IndexItem
        let data_idx = cur.read_u32::<LittleEndian>().unwrap();
        let data_len = cur.read_u32::<LittleEndian>().unwrap();

        let index_len = index_count as usize * INDEX_ITEM_SERIAL_SIZE;
        // Skip over the index_count 4-byte hunk and the index items
        let data_start = INDEX_COUNT_SERIAL_SIZE + index_len + data_idx as usize;
        let str_len = data_len - 1; // slice off NUL terminal

        // NB: malicious index data (i.e. with a bad idx or len) could make this panic for OOB
        // access
        let data: &[u8] = &self.as_bytes()[data_start..data_start + str_len as usize];
        std::str::from_utf8(data).ok()
    }

    // TODO: iter_strs
    pub fn iter_strs(&self) -> impl Iterator<Item = &str> {
        // Advanced in the loop below
        let mut index_cursor = Cursor::new(self.as_bytes());
        let index_count = index_cursor.read_u32::<LittleEndian>().unwrap() as usize;

        let index_len = index_count as usize * INDEX_ITEM_SERIAL_SIZE;
        let blob_start = INDEX_COUNT_SERIAL_SIZE + index_len;
        let blob = &self.as_bytes()[blob_start..];

        (0..index_count).map(move |_| {
            let data_idx = index_cursor.read_u32::<LittleEndian>().unwrap() as usize;
            let data_len = index_cursor.read_u32::<LittleEndian>().unwrap() as usize;
            let str_len = data_len - 1; // slice off NUL terminal
            std::str::from_utf8(&blob[data_idx..data_idx + str_len]).expect("invalid UTF-8")
        })
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

// StrArray buffo layout:
// [index_count: u32, [(idx: u32, len: u32)], [data blob]]
// Each idx is an offset into [data blob]
fn write_buffo<W>(parts: BuffoParts, mut cursor: W) -> io::Result<()>
where
    W: Write,
{
    let index_count: u32 = parts
        .index
        .len()
        .try_into()
        .expect("Too many items for buffo");
    cursor.write_u32::<LittleEndian>(index_count)?;
    for x in parts.index {
        cursor.write_u32::<LittleEndian>(x.idx)?;
        cursor.write_u32::<LittleEndian>(x.len)?;
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

    // debug function
    fn xxd(xs: &[u8]) {
        for (i, x) in xs.iter().enumerate() {
            print!("{:02x}", x);
            if i % 4 == 3 {
                println!("");
            }
        }
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
        let buffo = Buffo::str_array(input.clone());
        xxd(buffo.as_bytes());
        let output: Vec<&str> = buffo.iter_strs().collect();
        assert_eq!(input, output);
    }
}
