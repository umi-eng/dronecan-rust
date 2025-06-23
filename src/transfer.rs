use core::fmt;
use managed::ManagedSlice;

/// Transfer error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-1", derive(defmt::Format))]
pub enum Error {
    DataLength,
    BufferTooSmall,
    FrameOrder,
    Crc,
    IdMismatch,
    Toggle,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataLength => write!(f, "Data length invalid"),
            Self::BufferTooSmall => write!(f, "Buffer is too small"),
            Self::FrameOrder => write!(f, "Transfer frame out of order"),
            Self::Crc => write!(f, "CRC check failed"),
            Self::IdMismatch => write!(f, "ID mismatch"),
            Self::Toggle => write!(f, "Toggle bit incorrect"),
        }
    }
}

impl core::error::Error for Error {}

/// Single-frame or multi-frame payload transfer.
///
/// This implementation doesn't yet verify the checksum.
#[derive(Debug)]
#[cfg_attr(feature = "defmt-1", derive(defmt::Format))]
pub struct Transfer<'a> {
    storage: ManagedSlice<'a, u8>,
    length: usize,
    transfer_id: u8,
    toggle: bool,
}

impl<'a> Transfer<'a> {
    /// Create a new empty transfer.
    ///
    /// Every element in `storage` will be reset.
    pub fn new<S>(storage: S) -> Self
    where
        S: Into<ManagedSlice<'a, u8>>,
    {
        let mut storage = storage.into();

        if let ManagedSlice::Owned(vec) = &mut storage {
            vec.clear();
        }

        Self {
            storage,
            length: 0,
            transfer_id: 0,
            toggle: false,
        }
    }

    /// Feed data frames to the ongoing transfer.
    ///
    /// If the frame is accepted `Ok(None)` will be returned or
    /// `Ok(Some(&[u8]))` with the inner data if the last data frame was marked
    /// as the end of the transfer.
    ///
    /// If an [`Error`] is returned, the transfer should probably be abandoned.
    pub fn add_frame(&mut self, data: &[u8]) -> Result<Option<&[u8]>, Error> {
        if data.len() > 8 {
            return Err(Error::DataLength);
        }

        let tail = match data.last() {
            Some(d) => Tail(*d),
            None => return Err(Error::DataLength),
        };

        if tail.start() && self.length != 0 {
            // this is not the first transfer
            return Err(Error::FrameOrder);
        }

        if tail.start() {
            self.transfer_id = tail.transfer_id();
            self.toggle = tail.toggle();
        } else {
            // we cannot start with an end frame
            if self.length == 0 && tail.end() {
                return Err(Error::FrameOrder);
            }

            if self.transfer_id != tail.transfer_id() {
                return Err(Error::IdMismatch);
            }

            if self.toggle == tail.toggle() {
                return Err(Error::Toggle);
            } else {
                self.toggle = tail.toggle();
            }
        }

        let inner_data = if tail.start() && !tail.end() {
            &data[2..data.len() - 1]
        } else {
            // single frame transfers don't start with crc
            &data[..data.len() - 1]
        };

        match &mut self.storage {
            #[cfg(feature = "alloc")]
            ManagedSlice::Owned(vec) => {
                vec.extend_from_slice(inner_data);
            }
            ManagedSlice::Borrowed(slice) => {
                if self.length + inner_data.len() > slice.len() {
                    return Err(Error::BufferTooSmall);
                }
                slice[self.length..self.length + inner_data.len()].copy_from_slice(inner_data)
            }
        }

        self.length += inner_data.len();

        Ok(if tail.end() {
            // todo: crc check
            Some(&self.storage[..self.length])
        } else {
            None
        })
    }
}

/// Newtype for interpreting the tail byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tail(u8);

impl Tail {
    fn start(&self) -> bool {
        (self.0 & (1 << 7)) != 0
    }

    fn end(&self) -> bool {
        (self.0 & (1 << 6)) != 0
    }

    fn toggle(&self) -> bool {
        (self.0 & (1 << 5)) != 0
    }

    fn transfer_id(&self) -> u8 {
        self.0 & 0x1F
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_byte() {
        let tail = Tail(0xFF);
        assert!(tail.start());
        assert!(tail.end());
        assert!(tail.toggle());
        assert!(tail.transfer_id() == 31);

        let tail = Tail(0x7C);
        assert!(!tail.start());
        assert!(tail.end());
        assert!(tail.toggle());
        assert!(tail.transfer_id() == 28)
    }

    #[test]
    fn transfer_single() {
        // 4-byte transfer
        let mut transfer = Transfer::new(vec![]);
        let res = transfer.add_frame(&[0x01, 0x02, 0x03, 0x04, 0xFF]);
        assert_eq!(res, Ok(Some([0x01, 0x02, 0x03, 0x04].as_ref())));

        // 7-byte transfer
        let mut transfer = Transfer::new(vec![]);
        let res = transfer.add_frame(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0xFF]);
        assert_eq!(
            res,
            Ok(Some([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07].as_ref()))
        );
    }

    #[test]
    fn tansfer_multi() {
        let mut transfer = Transfer::new(vec![]);
        let res = transfer.add_frame(&[0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D]);
        assert_eq!(res, Ok(None));
        let res = transfer.add_frame(&[0x00, 0x7D, 0x33, 0x7D]);
        let data = &[0x01, 0x00, 0x68, 0xB5, 0x02, 0x00, 0x7D, 0x33];
        assert_eq!(res, Ok(Some(data.as_ref())));
    }

    #[test]
    fn tansfer_buffer_too_small() {
        let mut storage = [0; 7]; // 1 byte too small
        let mut transfer = Transfer::new(storage.as_mut_slice());
        let res = transfer.add_frame(&[0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D]);
        assert_eq!(res, Ok(None));
        let res = transfer.add_frame(&[0x00, 0x7D, 0x33, 0x7D]);
        assert_eq!(res, Err(Error::BufferTooSmall));
    }
}
