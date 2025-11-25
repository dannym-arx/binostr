//! DannyPack: Ultra-compact custom binary format for Nostr events
//!
//! BLAZINGLY FAST - uses unsafe pointer operations for maximum speed.
//!
//! Layout (single event):
//! ```text
//! [fixed: 138 bytes]
//!   - id: 32 bytes
//!   - pubkey: 32 bytes  
//!   - sig: 64 bytes
//!   - created_at: 8 bytes (i64 LE)
//!   - kind: 2 bytes (u16 LE)
//! [tag_len: varint] + [tag_data: variable]
//! [content_header: 1 byte (bit7=is_hex, bits0-6=len or 0x7F for varint)] + [content_data]
//! ```

use crate::event::NostrEvent;
use std::ptr;

const FIXED_SIZE: usize = 138;

#[inline(always)]
unsafe fn write_varint_ptr(mut dst: *mut u8, mut value: u64) -> usize {
    let start = dst;
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        *dst = byte;
        dst = dst.add(1);
        if value == 0 {
            break;
        }
    }
    dst.offset_from(start) as usize
}

#[inline(always)]
unsafe fn read_varint_ptr(src: *const u8, max_len: usize) -> (u64, usize) {
    let mut result: u64 = 0;
    let mut shift = 0;
    let mut pos = 0;

    loop {
        if pos >= max_len {
            return (0, 0);
        }
        let byte = *src.add(pos);
        pos += 1;

        result |= ((byte & 0x7F) as u64) << shift;

        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }

    (result, pos)
}

#[inline(always)]
unsafe fn write_len_flag_ptr(dst: *mut u8, len: usize, is_hex: bool) -> usize {
    let flag = if is_hex { 0x80u8 } else { 0x00u8 };
    if len < 0x7F {
        *dst = flag | (len as u8);
        1
    } else {
        *dst = flag | 0x7F;
        1 + write_varint_ptr(dst.add(1), len as u64)
    }
}

#[inline(always)]
unsafe fn read_len_flag_ptr(src: *const u8, max_len: usize) -> (usize, bool, usize) {
    let header = *src;
    let is_hex = (header & 0x80) != 0;
    let len_or_marker = (header & 0x7F) as usize;

    if len_or_marker < 0x7F {
        (len_or_marker, is_hex, 1)
    } else {
        let (len, varint_bytes) = read_varint_ptr(src.add(1), max_len - 1);
        (len as usize, is_hex, 1 + varint_bytes)
    }
}

#[inline(always)]
const fn is_hex_digit(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')
}

#[inline(always)]
fn is_hex_string(s: &str) -> bool {
    let bytes = s.as_bytes();
    let len = bytes.len();
    if len == 0 || len % 2 != 0 {
        return false;
    }
    let mut i = 0;
    while i + 8 <= len {
        unsafe {
            let b0 = *bytes.get_unchecked(i);
            let b1 = *bytes.get_unchecked(i + 1);
            let b2 = *bytes.get_unchecked(i + 2);
            let b3 = *bytes.get_unchecked(i + 3);
            let b4 = *bytes.get_unchecked(i + 4);
            let b5 = *bytes.get_unchecked(i + 5);
            let b6 = *bytes.get_unchecked(i + 6);
            let b7 = *bytes.get_unchecked(i + 7);
            if !is_hex_digit(b0)
                || !is_hex_digit(b1)
                || !is_hex_digit(b2)
                || !is_hex_digit(b3)
                || !is_hex_digit(b4)
                || !is_hex_digit(b5)
                || !is_hex_digit(b6)
                || !is_hex_digit(b7)
            {
                return false;
            }
        }
        i += 8;
    }
    while i < len {
        unsafe {
            if !is_hex_digit(*bytes.get_unchecked(i)) {
                return false;
            }
        }
        i += 1;
    }
    true
}

#[inline(always)]
unsafe fn hex_decode_fast(src: &[u8], dst: *mut u8) -> usize {
    const HEX_DECODE: [u8; 256] = {
        let mut table = [0u8; 256];
        let mut i = 0;
        while i < 256 {
            table[i] = match i as u8 {
                b'0'..=b'9' => (i as u8) - b'0',
                b'a'..=b'f' => (i as u8) - b'a' + 10,
                b'A'..=b'F' => (i as u8) - b'A' + 10,
                _ => 0,
            };
            i += 1;
        }
        table
    };

    let len = src.len() / 2;
    let mut i = 0;
    while i < len {
        let hi = *HEX_DECODE.get_unchecked(*src.get_unchecked(i * 2) as usize);
        let lo = *HEX_DECODE.get_unchecked(*src.get_unchecked(i * 2 + 1) as usize);
        *dst.add(i) = (hi << 4) | lo;
        i += 1;
    }
    len
}

#[inline(always)]
unsafe fn hex_encode_fast(src: &[u8], dst: *mut u8) -> usize {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    let len = src.len();
    let mut i = 0;
    while i < len {
        let b = *src.get_unchecked(i);
        *dst.add(i * 2) = *HEX_CHARS.get_unchecked((b >> 4) as usize);
        *dst.add(i * 2 + 1) = *HEX_CHARS.get_unchecked((b & 0xF) as usize);
        i += 1;
    }
    len * 2
}

pub fn serialize(event: &NostrEvent) -> Vec<u8> {
    let tags_data_size = calc_tags_data_size(&event.tags);
    let content_len = event.content.len();
    let estimated = FIXED_SIZE + 5 + tags_data_size + 5 + content_len;

    let mut buf = Vec::with_capacity(estimated);

    unsafe {
        buf.set_len(estimated);
        let base = buf.as_mut_ptr();
        let mut ptr = base;

        ptr::copy_nonoverlapping(event.id.as_ptr(), ptr, 32);
        ptr::copy_nonoverlapping(event.pubkey.as_ptr(), ptr.add(32), 32);
        ptr::copy_nonoverlapping(event.sig.as_ptr(), ptr.add(64), 64);
        ptr = ptr.add(128);

        *(ptr as *mut [u8; 8]) = event.created_at.to_le_bytes();
        ptr = ptr.add(8);
        *(ptr as *mut [u8; 2]) = event.kind.to_le_bytes();
        ptr = ptr.add(2);

        let len_bytes = write_varint_ptr(ptr, tags_data_size as u64);
        ptr = ptr.add(len_bytes);

        ptr = pack_tags_raw(ptr, &event.tags);

        let content_bytes = event.content.as_bytes();
        let len = content_bytes.len();
        let header_bytes = write_len_flag_ptr(ptr, len, false);
        ptr = ptr.add(header_bytes);
        ptr::copy_nonoverlapping(content_bytes.as_ptr(), ptr, len);
        ptr = ptr.add(len);

        buf.set_len(ptr.offset_from(base) as usize);
    }

    buf
}

pub fn serialize_compact(event: &NostrEvent) -> Vec<u8> {
    let estimated = FIXED_SIZE + 10 + estimate_tags_size(&event.tags) + event.content.len();
    let mut buf = Vec::with_capacity(estimated);

    unsafe {
        buf.set_len(estimated);
        let base = buf.as_mut_ptr();
        let mut ptr = base;

        ptr::copy_nonoverlapping(event.id.as_ptr(), ptr, 32);
        ptr::copy_nonoverlapping(event.pubkey.as_ptr(), ptr.add(32), 32);
        ptr::copy_nonoverlapping(event.sig.as_ptr(), ptr.add(64), 64);
        ptr = ptr.add(128);
        *(ptr as *mut [u8; 8]) = event.created_at.to_le_bytes();
        ptr = ptr.add(8);
        *(ptr as *mut [u8; 2]) = event.kind.to_le_bytes();
        ptr = ptr.add(2);

        let mut tag_buf = Vec::with_capacity(estimate_tags_size(&event.tags));
        tag_buf.set_len(tag_buf.capacity());
        let tag_end = pack_tags_fast(&event.tags, tag_buf.as_mut_ptr());
        let tags_len = tag_end.offset_from(tag_buf.as_ptr()) as usize;

        let len_bytes = write_varint_ptr(ptr, tags_len as u64);
        ptr = ptr.add(len_bytes);
        ptr::copy_nonoverlapping(tag_buf.as_ptr(), ptr, tags_len);
        ptr = ptr.add(tags_len);

        let content_bytes = event.content.as_bytes();
        if is_hex_string(&event.content) {
            let decoded_len = content_bytes.len() / 2;
            let header_bytes = write_len_flag_ptr(ptr, decoded_len, true);
            ptr = ptr.add(header_bytes);
            hex_decode_fast(content_bytes, ptr);
            ptr = ptr.add(decoded_len);
        } else {
            let len = content_bytes.len();
            let header_bytes = write_len_flag_ptr(ptr, len, false);
            ptr = ptr.add(header_bytes);
            ptr::copy_nonoverlapping(content_bytes.as_ptr(), ptr, len);
            ptr = ptr.add(len);
        }

        buf.set_len(ptr.offset_from(base) as usize);
    }

    buf
}

#[inline(always)]
fn calc_tags_data_size(tags: &[Vec<String>]) -> usize {
    let mut size = varint_size(tags.len() as u64);
    for tag in tags {
        size += 1;
        for value in tag {
            let len = value.len();
            size += if len < 0x7F {
                1
            } else {
                1 + varint_size(len as u64)
            };
            size += len;
        }
    }
    size
}

#[inline(always)]
const fn varint_size(mut value: u64) -> usize {
    let mut size = 1;
    while value >= 0x80 {
        value >>= 7;
        size += 1;
    }
    size
}

#[inline(always)]
unsafe fn pack_tags_raw(mut dst: *mut u8, tags: &[Vec<String>]) -> *mut u8 {
    dst = dst.add(write_varint_ptr(dst, tags.len() as u64));

    for tag in tags {
        *dst = tag.len() as u8;
        dst = dst.add(1);

        for value in tag {
            let bytes = value.as_bytes();
            let len = bytes.len();

            dst = dst.add(write_len_flag_ptr(dst, len, false));
            ptr::copy_nonoverlapping(bytes.as_ptr(), dst, len);
            dst = dst.add(len);
        }
    }

    dst
}

#[inline(always)]
fn estimate_tags_size(tags: &[Vec<String>]) -> usize {
    let mut size = 5;
    for tag in tags {
        size += 1;
        for value in tag {
            size += 3 + value.len();
        }
    }
    size
}

#[inline(always)]
unsafe fn pack_tags_fast(tags: &[Vec<String>], mut dst: *mut u8) -> *mut u8 {
    dst = dst.add(write_varint_ptr(dst, tags.len() as u64));

    for tag in tags {
        *dst = tag.len() as u8;
        dst = dst.add(1);

        for value in tag {
            let value_bytes = value.as_bytes();

            if is_hex_string(value) {
                let decoded_len = value_bytes.len() / 2;
                dst = dst.add(write_len_flag_ptr(dst, decoded_len, true));
                hex_decode_fast(value_bytes, dst);
                dst = dst.add(decoded_len);
            } else {
                let len = value_bytes.len();
                dst = dst.add(write_len_flag_ptr(dst, len, false));
                ptr::copy_nonoverlapping(value_bytes.as_ptr(), dst, len);
                dst = dst.add(len);
            }
        }
    }

    dst
}

pub fn deserialize(data: &[u8]) -> Result<NostrEvent, DannyPackError> {
    let len = data.len();
    if len < FIXED_SIZE + 2 {
        return Err(DannyPackError::TooShort);
    }

    unsafe {
        let base = data.as_ptr();
        let mut ptr = base;

        let mut id = [0u8; 32];
        let mut pubkey = [0u8; 32];
        let mut sig = [0u8; 64];

        ptr::copy_nonoverlapping(ptr, id.as_mut_ptr(), 32);
        ptr = ptr.add(32);
        ptr::copy_nonoverlapping(ptr, pubkey.as_mut_ptr(), 32);
        ptr = ptr.add(32);
        ptr::copy_nonoverlapping(ptr, sig.as_mut_ptr(), 64);
        ptr = ptr.add(64);

        let created_at = i64::from_le_bytes(*(ptr as *const [u8; 8]));
        ptr = ptr.add(8);

        let kind = u16::from_le_bytes(*(ptr as *const [u8; 2]));
        ptr = ptr.add(2);

        let remaining = len - (ptr.offset_from(base) as usize);

        let (tag_len, varint_bytes) = read_varint_ptr(ptr, remaining);
        if varint_bytes == 0 {
            return Err(DannyPackError::TooShort);
        }
        ptr = ptr.add(varint_bytes);
        let tag_len = tag_len as usize;

        let remaining = len - (ptr.offset_from(base) as usize);
        if tag_len > remaining {
            return Err(DannyPackError::TooShort);
        }

        let tags = unpack_tags_fast(ptr, tag_len)?;
        ptr = ptr.add(tag_len);

        let remaining = len - (ptr.offset_from(base) as usize);
        let (content_len, content_is_hex, header_bytes) = read_len_flag_ptr(ptr, remaining);
        ptr = ptr.add(header_bytes);

        let remaining = len - (ptr.offset_from(base) as usize);
        if content_len > remaining {
            return Err(DannyPackError::TooShort);
        }

        let content = if content_is_hex {
            let mut s = String::with_capacity(content_len * 2);
            s.as_mut_vec().set_len(content_len * 2);
            hex_encode_fast(std::slice::from_raw_parts(ptr, content_len), s.as_mut_ptr());
            s
        } else {
            String::from_utf8_unchecked(std::slice::from_raw_parts(ptr, content_len).to_vec())
        };

        Ok(NostrEvent {
            id,
            pubkey,
            created_at,
            kind,
            tags,
            content,
            sig,
        })
    }
}

#[inline(always)]
unsafe fn unpack_tags_fast(
    ptr: *const u8,
    max_len: usize,
) -> Result<Vec<Vec<String>>, DannyPackError> {
    if max_len == 0 {
        return Ok(Vec::new());
    }

    let mut pos = 0;

    let (tag_count, varint_bytes) = read_varint_ptr(ptr, max_len);
    if varint_bytes == 0 {
        return Err(DannyPackError::InvalidTagData);
    }
    pos += varint_bytes;
    let tag_count = tag_count as usize;

    let mut tags = Vec::with_capacity(tag_count);

    for _ in 0..tag_count {
        if pos >= max_len {
            return Err(DannyPackError::InvalidTagData);
        }

        let value_count = *ptr.add(pos) as usize;
        pos += 1;

        let mut values = Vec::with_capacity(value_count);

        for _ in 0..value_count {
            let remaining = max_len - pos;
            let (len, is_hex, header_bytes) = read_len_flag_ptr(ptr.add(pos), remaining);
            pos += header_bytes;

            if pos + len > max_len {
                return Err(DannyPackError::InvalidTagData);
            }

            let value = if is_hex {
                let mut s = String::with_capacity(len * 2);
                s.as_mut_vec().set_len(len * 2);
                hex_encode_fast(
                    std::slice::from_raw_parts(ptr.add(pos), len),
                    s.as_mut_ptr(),
                );
                s
            } else {
                String::from_utf8_unchecked(std::slice::from_raw_parts(ptr.add(pos), len).to_vec())
            };

            values.push(value);
            pos += len;
        }

        tags.push(values);
    }

    Ok(tags)
}

pub fn serialize_batch(events: &[NostrEvent]) -> Vec<u8> {
    let serialized: Vec<Vec<u8>> = events.iter().map(serialize).collect();
    let total_size: usize = 4 + serialized.iter().map(|e| 4 + e.len()).sum::<usize>();

    let mut buf = Vec::with_capacity(total_size);

    unsafe {
        buf.set_len(total_size);
        let mut ptr = buf.as_mut_ptr();

        ptr::copy_nonoverlapping((events.len() as u32).to_le_bytes().as_ptr(), ptr, 4);
        ptr = ptr.add(4);

        for event_data in &serialized {
            ptr::copy_nonoverlapping((event_data.len() as u32).to_le_bytes().as_ptr(), ptr, 4);
            ptr = ptr.add(4);
            ptr::copy_nonoverlapping(event_data.as_ptr(), ptr, event_data.len());
            ptr = ptr.add(event_data.len());
        }
    }

    buf
}

pub fn deserialize_batch(data: &[u8]) -> Result<Vec<NostrEvent>, DannyPackError> {
    let len = data.len();
    if len < 4 {
        return Err(DannyPackError::TooShort);
    }

    unsafe {
        let base = data.as_ptr();
        let mut ptr = base;

        let event_count = u32::from_le_bytes(*(ptr as *const [u8; 4])) as usize;
        ptr = ptr.add(4);

        let mut events = Vec::with_capacity(event_count);

        for _ in 0..event_count {
            let remaining = len - (ptr.offset_from(base) as usize);
            if remaining < 4 {
                return Err(DannyPackError::TooShort);
            }

            let event_len = u32::from_le_bytes(*(ptr as *const [u8; 4])) as usize;
            ptr = ptr.add(4);

            let remaining = len - (ptr.offset_from(base) as usize);
            if event_len > remaining {
                return Err(DannyPackError::TooShort);
            }

            events.push(deserialize(std::slice::from_raw_parts(ptr, event_len))?);
            ptr = ptr.add(event_len);
        }

        Ok(events)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DannyPackError {
    #[error("Data too short")]
    TooShort,

    #[error("Invalid tag data")]
    InvalidTagData,

    #[error("Invalid varint")]
    InvalidVarint,

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> NostrEvent {
        NostrEvent {
            id: [0xab; 32],
            pubkey: [0xcd; 32],
            created_at: 1234567890,
            kind: 1,
            tags: vec![
                vec!["p".to_string(), "abcd1234".to_string()],
                vec!["e".to_string(), "deadbeef".to_string()],
            ],
            content: "Hello, Nostr!".to_string(),
            sig: [0xef; 64],
        }
    }

    fn sample_event_hex_content() -> NostrEvent {
        NostrEvent {
            id: [0xab; 32],
            pubkey: [0xcd; 32],
            created_at: 1234567890,
            kind: 1,
            tags: vec![],
            content: "deadbeefcafe1234".to_string(),
            sig: [0xef; 64],
        }
    }

    #[test]
    fn test_roundtrip() {
        let event = sample_event();
        let bytes = serialize(&event);
        let back = deserialize(&bytes).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_roundtrip_hex_content() {
        let event = sample_event_hex_content();
        let bytes = serialize(&event);
        let back = deserialize(&bytes).unwrap();
        assert_eq!(event, back);

        let non_hex = sample_event();
        let non_hex_bytes = serialize(&non_hex);
        println!("Normal content: {} bytes", non_hex_bytes.len());
        println!("Hex content:    {} bytes", bytes.len());
    }

    #[test]
    fn test_batch_roundtrip() {
        let events = vec![sample_event(), sample_event_hex_content()];
        let bytes = serialize_batch(&events);
        let back = deserialize_batch(&bytes).unwrap();
        assert_eq!(events, back);
    }

    #[test]
    fn test_size_comparison() {
        let event = sample_event();

        let dannypack_size = serialize(&event).len();
        let json_size = crate::json::serialize(&event).len();

        println!("DannyPack: {} bytes", dannypack_size);
        println!("JSON:      {} bytes", json_size);
        println!(
            "Savings:   {:.1}%",
            100.0 * (1.0 - dannypack_size as f64 / json_size as f64)
        );
    }
}
