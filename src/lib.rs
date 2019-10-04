use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{
    convert::TryInto,
    io::{self, Cursor, Seek, SeekFrom, Write},
    mem::size_of,
};

#[derive(Debug)]
pub struct Buffo(Vec<u8>);

impl Buffo {
    pub fn str_array<'a, T>(strs: T) -> Buffo
    where
        T: IntoIterator<Item = &'a str>,
    {
        let mut index: Vec<IndexItem> = vec![];
        let mut data = vec![];
        let mut cursor = 0usize; // tracks idx into data buffer
        for s in strs {
            data.extend_from_slice(s.as_bytes());
            data.push(0u8); // NUL-terminate for C string compatibility
            let len = (s.as_bytes().len() + 1)
                .try_into()
                .expect("string too long");
            let idx: u32 = cursor.try_into().expect("too much data");
            index.push(IndexItem { idx, len });
            cursor += len as usize;
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
        let item_idx: usize = (i as usize) * 2 * size_of::<u32>();
        cur.seek(SeekFrom::Current(item_idx as i64)).unwrap();

        // Read out IndexItem
        let data_idx = cur.read_u32::<LittleEndian>().unwrap();
        let data_len = cur.read_u32::<LittleEndian>().unwrap();

        // StrArray buffo layout:
        // [index_count: u32, (idx: u32, len: u32)..., [data: u8, ...]]
        let index_len = index_count as usize * 2 * size_of::<u32>();
        // Skip over the index_count 4-byte hunk, then the index items
        let data_start = size_of::<u32>() + index_len as usize + data_idx as usize;
        let data_len = data_len - 1; // slice off NUL terminal
        let data: &[u8] = &self.as_bytes()[data_start..data_start + data_len as usize];
        std::str::from_utf8(data).ok()
    }

    // TODO: iter_strs
    pub fn collect_strs(&self) -> Vec<&str> {
        // let mut result = vec![];
        // let mut cur = Cursor::new(self.as_bytes());
        // let index_len = cur.read_u32::<LittleEndian>().unwrap();
        // // The nth index item is at i * sizeof(IndexItem)
        // let foo: usize = (i as usize) * 2 * size_of::<u32>();
        // cur.seek(SeekFrom::Current(foo as i64)).unwrap();
        // let data_idx = cur.read_u32::<LittleEndian>().unwrap();
        // let data_len = cur.read_u32::<LittleEndian>().unwrap();

        panic!()
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
}
