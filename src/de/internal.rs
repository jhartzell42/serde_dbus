use crate::align::align;
use crate::error::{Error, Result};
use crate::message::Message;
use byteorder::ByteOrder;
use log::{error, trace};
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::str::from_utf8;

pub(super) struct DataBuffer<'de> {
    pub(super) data: &'de [u8],
    pub(super) data_ix: usize,
}

impl<'de> DataBuffer<'de> {
    pub(super) fn from_message(mesg: &'de Message) -> Self {
        Self {
            data: &mesg.data,
            data_ix: 0,
        }
    }

    pub(super) fn complete(self) -> Result<()> {
        let leftover_data = self.data.len() - self.data_ix;
        if leftover_data != 0 {
            return Err(Error::LeftoverData(leftover_data));
        }

        Ok(())
    }
}

pub(super) struct Deserializer<'a, 'de, B: ByteOrder> {
    data_buffer: &'a mut DataBuffer<'de>,
    sig: &'de [u8],
    sig_ix: usize,
    phantom: PhantomData<B>,
}

pub(super) struct ArrayDeserializer<'a, 'de, B: ByteOrder> {
    pub(super) data_buffer: &'a mut DataBuffer<'de>,
    pub(super) end_ix: usize,
    pub(super) item_sig: &'de [u8],
    pub(super) phantom: PhantomData<B>,
}

impl<'a, 'de, B: ByteOrder> ArrayDeserializer<'a, 'de, B> {
    pub(super) fn next_item_deserializer<'b>(
        &'b mut self,
    ) -> Result<Option<Deserializer<'b, 'de, B>>> {
        match self.data_buffer.data_ix.cmp(&self.end_ix) {
            Ordering::Greater => Err(Error::ArrayElementOverrun(
                self.data_buffer.data_ix,
                self.end_ix,
            )),
            Ordering::Equal => Ok(None),
            Ordering::Less => {
                let sig = self.item_sig;
                Ok(Some(Deserializer {
                    data_buffer: self.data_buffer,
                    sig,
                    sig_ix: 0,
                    phantom: PhantomData,
                }))
            }
        }
    }

    pub(super) fn next_key_deserializer<'b>(
        &'b mut self,
    ) -> Result<Option<Deserializer<'b, 'de, B>>> {
        if let Some(mut de) = self.next_item_deserializer()? {
            de.align_reader(8)?;
            de.expect_signature_byte(b'{')?;
            let sig = de.grab_single_sig()?;
            Ok(Some(Deserializer {
                data_buffer: de.data_buffer,
                sig,
                sig_ix: 0,
                phantom: PhantomData,
            }))
        } else {
            Ok(None)
        }
    }

    pub(super) fn next_value_deserializer<'b>(
        &'b mut self,
    ) -> Result<Option<Deserializer<'b, 'de, B>>> {
        if let Some(mut de) = self.next_item_deserializer()? {
            de.expect_signature_byte(b'{')?;
            let _ = de.grab_single_sig()?;
            let sig = de.grab_single_sig()?;
            de.expect_signature_byte(b'}')?;
            Ok(Some(Deserializer {
                data_buffer: de.data_buffer,
                sig,
                sig_ix: 0,
                phantom: PhantomData,
            }))
        } else {
            Ok(None)
        }
    }
}

// TODO: This information is repeated in too many places
// Clean that up.
fn sig_alignment(item_sig: u8) -> Result<usize> {
    match item_sig {
        b'y' => Ok(1), // BYTE
        b'b' => Ok(4), // BOOLEAN
        b'n' => Ok(2), // INT16
        b'q' => Ok(2), // UINT16
        b'i' => Ok(4), // INT32
        b'u' => Ok(4), // UINT32
        b'x' => Ok(8), // INT64
        b't' => Ok(8), // UINT64
        b'd' => Ok(8), // DOUBLE
        b's' => Ok(4), // STRING
        b'o' => Ok(4), // OBJECT_PATH
        b'g' => Ok(1), // SIGNATURE
        b'a' => Ok(4), // ARRAY
        b'(' => Ok(8), // STRUCT
        b'v' => Ok(1), // VARIANT
        b'{' => Ok(8), // DICT_ENTRY
        b'h' => Ok(4), // UNIX_FD
        _ => Err(Error::UnrecognizedSignatureCharacter(item_sig)),
    }
}

impl<'a, 'de, B: ByteOrder> Deserializer<'a, 'de, B> {
    // Must have already processed the 'a' sig side, and the size data side.
    pub(super) fn array_deserializer(
        mut self,
        array_size: usize,
    ) -> Result<ArrayDeserializer<'a, 'de, B>> {
        let item_sig = self.grab_single_sig()?;
        self.align_reader(sig_alignment(item_sig[0])?)?;
        let end_ix = self.data_buffer.data_ix + array_size;
        Ok(ArrayDeserializer {
            data_buffer: &mut *self.data_buffer,
            end_ix,
            item_sig,
            phantom: PhantomData,
        })
    }

    pub(super) fn subsidiary<'b>(&'b mut self) -> Result<Deserializer<'b, 'de, B>> {
        let item_sig = self.grab_single_sig()?;
        Ok(Deserializer {
            data_buffer: &mut *self.data_buffer,
            sig: item_sig,
            sig_ix: 0,
            phantom: PhantomData,
        })
    }

    pub(super) fn from_message_and_buffer(
        buff: &'a mut DataBuffer<'de>,
        mesg: &'de Message,
    ) -> Self {
        let sig = &mesg.signature;
        let sig_ix = 0;
        Self {
            data_buffer: buff,
            sig,
            sig_ix,
            phantom: PhantomData,
        }
    }

    pub(super) fn possible_variant<'b>(&'b mut self) -> Result<Deserializer<'b, 'de, B>> {
        let mut sig;
        let sig_ix;
        if self.probe_signature_bytes("v".as_bytes()) {
            let sig_len = self.read(1)?[0] as usize;
            sig = self.read(sig_len + 1)?;
            sig = &sig[..sig_len]; // cut terminating nul byte
            sig_ix = 0;
        } else {
            sig = self.sig;
            sig_ix = self.sig_ix;
        }

        Ok(Deserializer {
            data_buffer: &mut *self.data_buffer,
            sig,
            sig_ix,
            phantom: PhantomData,
        })
    }

    pub(super) fn peek_single_sig(&mut self) -> Result<&'de [u8]> {
        let start = self.sig_ix;
        let res = self.grab_single_sig()?;
        self.sig_ix = start;
        Ok(res)
    }

    pub(super) fn grab_single_sig(&mut self) -> Result<&'de [u8]> {
        let start = self.sig_ix;
        let mut nesting = 0;
        for i in self.sig_ix..self.sig.len() {
            match self.sig[i] {
                b'(' | b'[' | b'{' => {
                    nesting += 1;
                }
                b')' | b']' | b'}' => {
                    nesting -= 1;
                }
                b'a' => continue,
                _ => (),
            }

            if nesting == 0 {
                self.sig_ix = i + 1;
                return Ok(&self.sig[start..=i]);
            }
        }

        Err(Error::MismatchedSignatureBracketing(start))
    }

    pub(super) fn probe_signature_bytes(&mut self, expected: &[u8]) -> bool {
        let len = expected.len();
        if self.sig_ix + len > self.sig.len() {
            return false;
        }
        let ix = self.sig_ix;

        let sig_slice = &self.sig[ix..ix + len];
        if sig_slice == expected {
            self.sig_ix = ix + len;
            true
        } else {
            false
        }
    }

    pub(super) fn expect_signature_byte(&mut self, expected: u8) -> Result<()> {
        if self.sig_ix >= self.sig.len() {
            return Err(Error::SignatureExhausted);
        }

        let got = self.sig[self.sig_ix];
        if got != expected {
            return Err(Error::SignatureError(expected, got));
        }

        self.sig_ix += 1;
        Ok(())
    }

    pub(super) fn expect_signature_str(&mut self, expected: &[u8]) -> Result<()> {
        if self.probe_signature_bytes(expected) {
            Ok(())
        } else {
            Err(Error::SignatureErrorIx(expected.to_vec(), self.sig_ix))
        }
    }

    // Index after read must be valid for read to be valid
    fn validate_ix(&mut self) -> Result<()> {
        // Strict inequality
        if self.data_buffer.data_ix > self.data_buffer.data.len() {
            error!("Index out of bounds");
            return Err(Error::IndexOutOfBounds(self.data_buffer.data_ix));
        }

        Ok(())
    }

    pub(super) fn align_reader(&mut self, alignment: usize) -> Result<()> {
        self.data_buffer.data_ix = align(self.data_buffer.data_ix, alignment);
        self.validate_ix()?;
        Ok(())
    }

    pub(super) fn read(&mut self, len: usize) -> Result<&'de [u8]> {
        let old_ix = self.data_buffer.data_ix;
        let new_ix = old_ix + len;
        self.data_buffer.data_ix = new_ix;
        self.validate_ix()?;
        Ok(&self.data_buffer.data[old_ix..new_ix])
    }

    pub(super) fn read_align_signature_value<const S: usize>(
        &mut self,
        signature: u8,
        alignment: usize,
    ) -> Result<[u8; S]> {
        self.expect_signature_byte(signature)?;
        self.align_reader(alignment)?;
        trace!(
            "Read signature '{}', {} bytes, at {}",
            signature as char,
            S,
            self.data_buffer.data_ix
        );
        let slice = self.read(S)?;
        let mut array = [0u8; S];
        array.copy_from_slice(slice);
        Ok(array)
    }

    pub(super) fn deserialize_bytes_basic(&mut self) -> Result<&[u8]> {
        trace!("read string at {}", self.data_buffer.data_ix);
        let size = u32::from_le_bytes(self.read_align_signature_value(b's', 4)?);
        trace!("size is {}", size);
        let res = self.read((size as usize) + 1)?;
        Ok(&res[..size as usize])
    }

    pub(super) fn deserialize_str_basic(&mut self) -> Result<&str> {
        let str = from_utf8(self.deserialize_bytes_basic()?)?;
        trace!("string is {}", str);
        Ok(str)
    }
}
