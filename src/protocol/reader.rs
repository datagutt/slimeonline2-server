//! Binary message reader for Slime Online 2 protocol
//!
//! Reads little-endian values and null-terminated strings from byte buffers.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReadError {
    #[error("Unexpected end of message: expected {expected} bytes, only {available} available")]
    UnexpectedEnd { expected: usize, available: usize },

    #[error("String not null-terminated")]
    StringNotTerminated,

    #[error("Invalid UTF-8 in string: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    #[error("Message too short: minimum {minimum} bytes required, got {actual}")]
    MessageTooShort { minimum: usize, actual: usize },
}

pub type ReadResult<T> = Result<T, ReadError>;

/// Binary message reader that tracks position through a byte buffer.
///
/// All multi-byte integers are read in little-endian format.
/// Strings are null-terminated (C-style).
pub struct MessageReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> MessageReader<'a> {
    /// Create a new reader from a byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Get the current read position.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Get the number of bytes remaining.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Check if we've reached the end of the message.
    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Get the total length of the data.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Skip a number of bytes.
    pub fn skip(&mut self, count: usize) -> ReadResult<()> {
        if self.pos + count > self.data.len() {
            return Err(ReadError::UnexpectedEnd {
                expected: count,
                available: self.remaining(),
            });
        }
        self.pos += count;
        Ok(())
    }

    /// Read a single unsigned byte (u8).
    pub fn read_u8(&mut self) -> ReadResult<u8> {
        if self.pos >= self.data.len() {
            return Err(ReadError::UnexpectedEnd {
                expected: 1,
                available: 0,
            });
        }
        let value = self.data[self.pos];
        self.pos += 1;
        Ok(value)
    }

    /// Read an unsigned 16-bit integer (little-endian).
    pub fn read_u16(&mut self) -> ReadResult<u16> {
        if self.pos + 2 > self.data.len() {
            return Err(ReadError::UnexpectedEnd {
                expected: 2,
                available: self.remaining(),
            });
        }
        let bytes = [self.data[self.pos], self.data[self.pos + 1]];
        self.pos += 2;
        Ok(u16::from_le_bytes(bytes))
    }

    /// Read an unsigned 32-bit integer (little-endian).
    pub fn read_u32(&mut self) -> ReadResult<u32> {
        if self.pos + 4 > self.data.len() {
            return Err(ReadError::UnexpectedEnd {
                expected: 4,
                available: self.remaining(),
            });
        }
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Read a signed 16-bit integer (little-endian).
    pub fn read_i16(&mut self) -> ReadResult<i16> {
        if self.pos + 2 > self.data.len() {
            return Err(ReadError::UnexpectedEnd {
                expected: 2,
                available: self.remaining(),
            });
        }
        let bytes = [self.data[self.pos], self.data[self.pos + 1]];
        self.pos += 2;
        Ok(i16::from_le_bytes(bytes))
    }

    /// Read a signed 32-bit integer (little-endian).
    pub fn read_i32(&mut self) -> ReadResult<i32> {
        if self.pos + 4 > self.data.len() {
            return Err(ReadError::UnexpectedEnd {
                expected: 4,
                available: self.remaining(),
            });
        }
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        Ok(i32::from_le_bytes(bytes))
    }

    /// Read a 32-bit floating point number (little-endian).
    pub fn read_f32(&mut self) -> ReadResult<f32> {
        if self.pos + 4 > self.data.len() {
            return Err(ReadError::UnexpectedEnd {
                expected: 4,
                available: self.remaining(),
            });
        }
        let bytes = [
            self.data[self.pos],
            self.data[self.pos + 1],
            self.data[self.pos + 2],
            self.data[self.pos + 3],
        ];
        self.pos += 4;
        Ok(f32::from_le_bytes(bytes))
    }

    /// Read a null-terminated string.
    ///
    /// The string is expected to be UTF-8 encoded (or ASCII).
    /// Advances position past the null terminator.
    pub fn read_string(&mut self) -> ReadResult<String> {
        let start = self.pos;

        // Find null terminator
        while self.pos < self.data.len() && self.data[self.pos] != 0 {
            self.pos += 1;
        }

        if self.pos >= self.data.len() {
            return Err(ReadError::StringNotTerminated);
        }

        let string_bytes = &self.data[start..self.pos];
        self.pos += 1; // Skip null terminator

        String::from_utf8(string_bytes.to_vec()).map_err(ReadError::InvalidUtf8)
    }

    /// Read a fixed-length byte array.
    pub fn read_bytes(&mut self, len: usize) -> ReadResult<Vec<u8>> {
        if self.pos + len > self.data.len() {
            return Err(ReadError::UnexpectedEnd {
                expected: len,
                available: self.remaining(),
            });
        }
        let bytes = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Ok(bytes)
    }

    /// Peek at the next byte without consuming it.
    pub fn peek_u8(&self) -> ReadResult<u8> {
        if self.pos >= self.data.len() {
            return Err(ReadError::UnexpectedEnd {
                expected: 1,
                available: 0,
            });
        }
        Ok(self.data[self.pos])
    }

    /// Peek at the next u16 without consuming it.
    pub fn peek_u16(&self) -> ReadResult<u16> {
        if self.pos + 2 > self.data.len() {
            return Err(ReadError::UnexpectedEnd {
                expected: 2,
                available: self.remaining(),
            });
        }
        let bytes = [self.data[self.pos], self.data[self.pos + 1]];
        Ok(u16::from_le_bytes(bytes))
    }

    /// Read the message type (first u16) from a buffer.
    ///
    /// This is a convenience method that reads the message type
    /// which is always the first field in any message.
    pub fn read_message_type(&mut self) -> ReadResult<u16> {
        self.read_u16()
    }

    /// Get a slice of the remaining data.
    pub fn remaining_data(&self) -> &[u8] {
        &self.data[self.pos..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u8() {
        let data = [0x42, 0xFF, 0x00];
        let mut reader = MessageReader::new(&data);

        assert_eq!(reader.read_u8().unwrap(), 0x42);
        assert_eq!(reader.read_u8().unwrap(), 0xFF);
        assert_eq!(reader.read_u8().unwrap(), 0x00);
        assert!(reader.read_u8().is_err());
    }

    #[test]
    fn test_read_u16_little_endian() {
        // 0x1234 in little endian is [0x34, 0x12]
        let data = [0x34, 0x12];
        let mut reader = MessageReader::new(&data);

        assert_eq!(reader.read_u16().unwrap(), 0x1234);
    }

    #[test]
    fn test_read_u32_little_endian() {
        // 0x12345678 in little endian is [0x78, 0x56, 0x34, 0x12]
        let data = [0x78, 0x56, 0x34, 0x12];
        let mut reader = MessageReader::new(&data);

        assert_eq!(reader.read_u32().unwrap(), 0x12345678);
    }

    #[test]
    fn test_read_string() {
        let data = b"Hello\x00World\x00";
        let mut reader = MessageReader::new(data);

        assert_eq!(reader.read_string().unwrap(), "Hello");
        assert_eq!(reader.read_string().unwrap(), "World");
    }

    #[test]
    fn test_read_empty_string() {
        let data = b"\x00";
        let mut reader = MessageReader::new(data);

        assert_eq!(reader.read_string().unwrap(), "");
    }

    #[test]
    fn test_string_not_terminated() {
        let data = b"Hello";
        let mut reader = MessageReader::new(data);

        assert!(matches!(
            reader.read_string(),
            Err(ReadError::StringNotTerminated)
        ));
    }

    #[test]
    fn test_read_login_message() {
        // Simulate MSG_LOGIN structure (after type is already read)
        let mut data = Vec::new();
        data.extend_from_slice(b"0.106\x00"); // version
        data.extend_from_slice(b"Player1\x00"); // username
        data.extend_from_slice(b"pass123\x00"); // password
        data.extend_from_slice(b"00-11-22-33-44-55\x00"); // mac

        let mut reader = MessageReader::new(&data);

        assert_eq!(reader.read_string().unwrap(), "0.106");
        assert_eq!(reader.read_string().unwrap(), "Player1");
        assert_eq!(reader.read_string().unwrap(), "pass123");
        assert_eq!(reader.read_string().unwrap(), "00-11-22-33-44-55");
        assert!(reader.is_empty());
    }

    #[test]
    fn test_remaining() {
        let data = [1, 2, 3, 4, 5];
        let mut reader = MessageReader::new(&data);

        assert_eq!(reader.remaining(), 5);
        reader.read_u8().unwrap();
        assert_eq!(reader.remaining(), 4);
        reader.read_u16().unwrap();
        assert_eq!(reader.remaining(), 2);
    }

    #[test]
    fn test_peek() {
        let data = [0x10, 0x00, 0xFF];
        let reader = MessageReader::new(&data);

        assert_eq!(reader.peek_u8().unwrap(), 0x10);
        assert_eq!(reader.peek_u16().unwrap(), 0x0010);
        assert_eq!(reader.position(), 0); // Position unchanged
    }
}
