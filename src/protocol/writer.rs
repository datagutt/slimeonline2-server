//! Binary message writer for Slime Online 2 protocol
//!
//! Writes little-endian values and null-terminated strings to byte buffers.

/// Binary message writer that builds byte buffers.
///
/// All multi-byte integers are written in little-endian format.
/// Strings are null-terminated (C-style).
pub struct MessageWriter {
    buffer: Vec<u8>,
}

impl MessageWriter {
    /// Create a new writer with default capacity.
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(256),
        }
    }

    /// Create a new writer with specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Get the current length of the buffer.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clear the buffer.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Write a single unsigned byte (u8).
    pub fn write_u8(&mut self, value: u8) -> &mut Self {
        self.buffer.push(value);
        self
    }

    /// Write an unsigned 16-bit integer (little-endian).
    pub fn write_u16(&mut self, value: u16) -> &mut Self {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// Write an unsigned 32-bit integer (little-endian).
    pub fn write_u32(&mut self, value: u32) -> &mut Self {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// Write a signed 16-bit integer (little-endian).
    pub fn write_i16(&mut self, value: i16) -> &mut Self {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// Write a signed 32-bit integer (little-endian).
    pub fn write_i32(&mut self, value: i32) -> &mut Self {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// Write a 32-bit floating point number (little-endian).
    pub fn write_f32(&mut self, value: f32) -> &mut Self {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// Write a null-terminated string.
    ///
    /// Appends the string bytes followed by a null byte (0x00).
    pub fn write_string(&mut self, value: &str) -> &mut Self {
        self.buffer.extend_from_slice(value.as_bytes());
        self.buffer.push(0); // Null terminator
        self
    }

    /// Write raw bytes.
    pub fn write_bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.buffer.extend_from_slice(bytes);
        self
    }

    /// Write a boolean as a single byte (0 or 1).
    pub fn write_bool(&mut self, value: bool) -> &mut Self {
        self.buffer.push(if value { 1 } else { 0 });
        self
    }

    /// Consume the writer and return the built buffer.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    /// Get a reference to the internal buffer.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Get a mutable reference to the internal buffer.
    pub fn as_bytes_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }
}

impl Default for MessageWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl From<MessageWriter> for Vec<u8> {
    fn from(writer: MessageWriter) -> Self {
        writer.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_u8() {
        let mut writer = MessageWriter::new();
        writer.write_u8(0x42);
        writer.write_u8(0xFF);

        assert_eq!(writer.as_bytes(), &[0x42, 0xFF]);
    }

    #[test]
    fn test_write_u16_little_endian() {
        let mut writer = MessageWriter::new();
        writer.write_u16(0x1234);

        // Little endian: least significant byte first
        assert_eq!(writer.as_bytes(), &[0x34, 0x12]);
    }

    #[test]
    fn test_write_u32_little_endian() {
        let mut writer = MessageWriter::new();
        writer.write_u32(0x12345678);

        // Little endian: least significant byte first
        assert_eq!(writer.as_bytes(), &[0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn test_write_string() {
        let mut writer = MessageWriter::new();
        writer.write_string("Hello");

        // String bytes + null terminator
        assert_eq!(writer.as_bytes(), b"Hello\x00");
    }

    #[test]
    fn test_write_empty_string() {
        let mut writer = MessageWriter::new();
        writer.write_string("");

        // Just null terminator
        assert_eq!(writer.as_bytes(), &[0x00]);
    }

    #[test]
    fn test_write_multiple_strings() {
        let mut writer = MessageWriter::new();
        writer.write_string("Hello");
        writer.write_string("World");

        assert_eq!(writer.as_bytes(), b"Hello\x00World\x00");
    }

    #[test]
    fn test_chained_writes() {
        let mut writer = MessageWriter::new();
        writer.write_u16(10).write_string("test").write_u32(12345);

        let bytes = writer.into_bytes();
        assert_eq!(bytes.len(), 2 + 5 + 4); // u16 + "test\0" + u32
    }

    #[test]
    fn test_write_login_response() {
        let mut writer = MessageWriter::new();
        writer
            .write_u16(10) // MSG_LOGIN
            .write_u8(1) // Success case
            .write_u16(42) // Player ID
            .write_u32(1234567890) // Server time
            .write_string("Welcome!");

        let bytes = writer.into_bytes();

        // Verify structure
        assert_eq!(bytes[0..2], [10, 0]); // MSG_LOGIN (little endian)
        assert_eq!(bytes[2], 1); // Success case
        assert_eq!(bytes[3..5], [42, 0]); // Player ID (little endian)
    }

    #[test]
    fn test_write_bool() {
        let mut writer = MessageWriter::new();
        writer.write_bool(true);
        writer.write_bool(false);

        assert_eq!(writer.as_bytes(), &[1, 0]);
    }

    #[test]
    fn test_clear() {
        let mut writer = MessageWriter::new();
        writer.write_u32(0xDEADBEEF);
        assert_eq!(writer.len(), 4);

        writer.clear();
        assert_eq!(writer.len(), 0);
        assert!(writer.is_empty());
    }
}
