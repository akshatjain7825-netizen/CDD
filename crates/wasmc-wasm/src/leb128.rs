#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leb128Error {
    ExceededMaxByteLimit(usize),
    UnexpectedEof,
    NonCanonicalOrOverflow,
}

pub struct Leb128Codec;

impl Leb128Codec {
    pub fn encode_u32(out: &mut Vec<u8>, mut val: u32) -> usize {
        let start = out.len();
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if val == 0 {
                break;
            }
        }
        out.len() - start
    }

    pub fn encode_i32(out: &mut Vec<u8>, mut val: i32) -> usize {
        let start = out.len();
        let mut more = true;
        while more {
            let byte = (val & 0x7F) as u8;
            val >>= 7; // arithmetic shift
            let sign_bit = (byte & 0x40) != 0;
            if (val == 0 && !sign_bit) || (val == -1 && sign_bit) {
                more = false;
                out.push(byte);
            } else {
                out.push(byte | 0x80);
            }
        }
        out.len() - start
    }

    pub fn encode_u64(out: &mut Vec<u8>, mut val: u64) -> usize {
        let start = out.len();
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if val == 0 {
                break;
            }
        }
        out.len() - start
    }

    pub fn encode_i64(out: &mut Vec<u8>, mut val: i64) -> usize {
        let start = out.len();
        let mut more = true;
        while more {
            let byte = (val & 0x7F) as u8;
            val >>= 7;
            let sign_bit = (byte & 0x40) != 0;
            if (val == 0 && !sign_bit) || (val == -1 && sign_bit) {
                more = false;
                out.push(byte);
            } else {
                out.push(byte | 0x80);
            }
        }
        out.len() - start
    }

    pub fn decode_u32(input: &[u8]) -> Result<(u32, usize), Leb128Error> {
        let (val, len) = Self::decode_u64_bounded(input, 32)?;
        Ok((val as u32, len))
    }

    pub fn decode_i32(input: &[u8]) -> Result<(i32, usize), Leb128Error> {
        let mut result = 0i32;
        let mut shift = 0;
        let mut idx = 0;
        let max_bytes = 5;

        loop {
            if idx >= input.len() {
                return Err(Leb128Error::UnexpectedEof);
            }
            if idx >= max_bytes {
                return Err(Leb128Error::ExceededMaxByteLimit(max_bytes));
            }
            let byte = input[idx];
            idx += 1;
            result |= ((byte & 0x7F) as i32) << shift;
            shift += 7;

            if (byte & 0x80) == 0 {
                if shift < 32 && (byte & 0x40) != 0 {
                    result |= !0 << shift;
                }
                return Ok((result, idx));
            }
        }
    }

    pub fn decode_u64(input: &[u8]) -> Result<(u64, usize), Leb128Error> {
        Self::decode_u64_bounded(input, 64)
    }

    pub fn decode_i64(input: &[u8]) -> Result<(i64, usize), Leb128Error> {
        let mut result = 0i64;
        let mut shift = 0;
        let mut idx = 0;
        let max_bytes = 10;

        loop {
            if idx >= input.len() {
                return Err(Leb128Error::UnexpectedEof);
            }
            if idx >= max_bytes {
                return Err(Leb128Error::ExceededMaxByteLimit(max_bytes));
            }
            let byte = input[idx];
            idx += 1;
            result |= ((byte & 0x7F) as i64) << shift;
            shift += 7;

            if (byte & 0x80) == 0 {
                if shift < 64 && (byte & 0x40) != 0 {
                    result |= !0 << shift;
                }
                return Ok((result, idx));
            }
        }
    }

    fn decode_u64_bounded(slice: &[u8], max_bits: u32) -> Result<(u64, usize), Leb128Error> {
        let max_bytes = ((max_bits + 6) / 7) as usize;
        let mut result = 0u64;
        let mut shift = 0u32;
        let mut idx = 0;

        loop {
            if idx >= max_bytes {
                return Err(Leb128Error::ExceededMaxByteLimit(max_bytes));
            }
            if idx >= slice.len() {
                return Err(Leb128Error::UnexpectedEof);
            }
            let byte = slice[idx];
            idx += 1;
            let low_7 = (byte & 0x7F) as u64;

            if idx == max_bytes {
                let rem_bits = max_bits - shift;
                let mask = if rem_bits >= 64 { !0u64 } else { (1u64 << rem_bits) - 1 };
                if (low_7 & !mask) != 0 {
                    return Err(Leb128Error::NonCanonicalOrOverflow);
                }
            }

            result |= low_7 << shift;
            if (byte & 0x80) == 0 {
                return Ok((result, idx));
            }
            shift += 7;
        }
    }
}
