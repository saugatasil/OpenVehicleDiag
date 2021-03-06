use byteorder::{BigEndian, ByteOrder, LittleEndian};
use std::io::Read;

/// Random Access file
///
/// Represents a stream of bytes
/// that can be read in order
/// or read data at specific offsets
#[derive(Debug)]
pub struct Raf {
    /// Data in bytes
    data: Vec<u8>,
    /// Max size of buffer
    size: usize,
    /// Current pos in buffer
    pub pos: usize,
    /// Byte order
    bo: RafByteOrder,
}

pub type Result<T> = std::result::Result<T, RafError>;

/// Errors that can be returned during reading of data
#[derive(Debug)]
pub enum RafError {
    /// End index requested exceeds the size of the data stored
    BufferOverflow,
    /// Start index of requested data is more than the max data stored
    StartOutOfRange,
    /// String parse failed. Due to invalid UTF8 Characters
    StrParseError,
}

/// Byte order representation struct
#[derive(Debug)]
pub enum RafByteOrder {
    /// Big endian
    BE,
    /// Little endian
    LE,
}

impl Raf {
    /// Creates a [Raf] struct from anything implimenting the [Read]
    /// trait
    ///
    /// # Params
    /// * reader - implimentor of the [Read] trait to be read into a [Raf]
    /// * bo - Byte order of the source data
    ///
    /// # Returns
    /// * Result, Raf is returned if read was successful, else Err(x) is returned
    pub fn from_read<R: Read>(reader: &mut R, bo: RafByteOrder) -> std::io::Result<Self> {
        let mut data: Vec<u8> = Vec::new();
        reader.read_to_end(&mut data).map(|size| Raf {
            data,
            size,
            pos: 0,
            bo,
        })
    }

    /// Creates a [Raf] struct from a Vector of bytes
    /// 
    /// # Params
    /// * data - Original source data - Will be cloned
    /// * bo - Byte order of the source data
    pub fn from_bytes(data: &Vec<u8>, bo: RafByteOrder) -> Self {
        Raf {
            data: data.clone(),
            size: data.len(),
            pos: 0,
            bo,
        }
    }


    pub fn read_bytes(&mut self, num_bytes: usize) -> Result<Vec<u8>> {
        if self.pos+num_bytes-1 > self.size {
            return Err(RafError::BufferOverflow);
        }
        let res = Vec::from(&self.data[self.pos..self.pos + num_bytes]);
        self.pos += num_bytes;
        Ok(res)
    }

    /// Seeks to location within the data stored
    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn adv(&mut self, pos: usize) -> Result<()> {
        match pos {
            x if self.pos + x > self.size => Err(RafError::StartOutOfRange),
            _ => Ok(self.pos += pos),
        }
    }

    /// Seeks to a position within the file prior to running [func].
    ///
    /// The position in the buffer will be subsequently set to the location
    /// where reading completed.
    /// 
    /// # Example
    /// ```
    /// let data: Vec<u8> = (0x00..0xFF).collect();
    /// let mut reader: Raf = Raf::from_bytes(&data, RafByteOrder::BE);
    /// reader.seek_read(2, Raf::read_i32); // Seeks to position 2 and reads i32
    /// ```
    ///
    /// # Params
    /// * pos - Position in file to start reading from
    /// * func - Function to run to read data
    pub fn seek_read<R>(&mut self, pos: usize, func: fn(&mut Self) -> Result<R>) -> Result<R> {
        self.seek(pos);
        func(self)
    }

    #[inline]
    fn read_primitive<T>(
        &mut self,
        size: usize,
        func_le: fn(&[u8]) -> T,
        func_be: fn(&[u8]) -> T,
    ) -> Result<T> {
        match self.bo {
            RafByteOrder::BE => self.read_bytes(size).map(|r| func_be(&r)),
            RafByteOrder::LE => self.read_bytes(size).map(|r| func_le(&r)),
        }
    }

    /// Reads a C String (Ends in 0x00)
    pub fn read_cstr(&mut self) -> Result<String> {
        let mut bytes: Vec<u8> = Vec::new();
        loop {
            let nextByte = self.read_u8().expect("Read string error");
            if nextByte == 0 {
                return match String::from_utf8(bytes) {
                    Err(_) => Err(RafError::StrParseError),
                    Ok(s) => Ok(s)
                }
            } else {
                bytes.push(nextByte);
            }
        }
    }

    /// Reads f32 from data at current position in buffer
    pub fn read_f32(&mut self) -> Result<f32> {
        self.read_primitive(4, LittleEndian::read_f32, BigEndian::read_f32)
    }

    /// Reads u64 from data at current position in buffer
    pub fn read_u64(&mut self) -> Result<u64> {
        self.read_primitive(8, LittleEndian::read_u64, BigEndian::read_u64)
    }

    /// Reads i64 from data at current position in buffer
    pub fn read_i64(&mut self) -> Result<i64> {
        self.read_primitive(8, LittleEndian::read_i64, BigEndian::read_i64)
    }

    /// Reads u32 from data at current position in buffer
    pub fn read_u32(&mut self) -> Result<u32> {
        self.read_primitive(4, LittleEndian::read_u32, BigEndian::read_u32)
    }

    /// Reads i32 from data at current position in buffer
    pub fn read_i32(&mut self) -> Result<i32> {
        self.read_primitive(4, LittleEndian::read_i32, BigEndian::read_i32)
    }

    /// Reads u16 from data at current position in buffer
    pub fn read_u16(&mut self) -> Result<u16> {
        self.read_primitive(2, LittleEndian::read_u16, BigEndian::read_u16)
    }

    /// Reads i16 from data at current position in buffer
    pub fn read_i16(&mut self) -> Result<i16> {
        self.read_primitive(2, LittleEndian::read_i16, BigEndian::read_i16)
    }

    /// Reads a single byte from data at current position in buffer
    pub fn read_u8(&mut self) -> Result<u8> {
        self.read_byte()
    }

    /// Reads a single byte from data at current position in buffer
    pub fn read_i8(&mut self) -> Result<i8> {
        self.read_byte().map(|x| x as i8)
    }

    pub fn read_byte(&mut self) -> Result<u8> {
        if self.pos > self.size {
            return Err(RafError::StartOutOfRange);
        }
        let res = self.data[self.pos];
        self.pos += 1;
        Ok(res)
    }

    /// Reads utf8 string from data at current position in buffer
    pub fn read_string(&mut self, len: usize) -> Result<String> {
        match String::from_utf8(self.read_bytes(len)?) {
            Err(_) => Err(RafError::StrParseError),
            Ok(s) => Ok(s),
        }
    }
}

#[test]
fn test_seek() {
    let data: Vec<u8> = (0x00..0xFF).collect();

    let mut reader: Raf = Raf::from_bytes(&data, RafByteOrder::BE);
    println!("{}", reader.seek_read(0, Raf::read_i32).unwrap());
}
