use core::fmt;
use managed::ManagedSlice;

/// CRC-16-CCITT-FALSE, as specified for DroneCAN transfers.
const CRC: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_IBM_3740);

/// Transfer error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    DataLength,
    BufferTooSmall,
    FrameOrder,
    IdMismatch,
    Toggle,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataLength => write!(f, "data length invalid"),
            Self::BufferTooSmall => write!(f, "buffer is too small"),
            Self::FrameOrder => write!(f, "transfer frame out of order"),
            Self::IdMismatch => write!(f, "id mismatch"),
            Self::Toggle => write!(f, "toggle bit incorrect"),
        }
    }
}

impl core::error::Error for Error {}

/// A completed transfer payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Payload<'a> {
    /// The assembled transfer payload.
    pub data: &'a [u8],
    /// Whether the transfer CRC is valid.
    ///
    /// This is always `false` if no data type signature was provided to
    /// [`Transfer::new`]. Single-frame transfers do not carry a CRC, so this is
    /// `true` for them when a signature was provided.
    pub crc_valid: bool,
}

/// Single-frame or multi-frame payload transfer.
///
/// Multi-frame transfers are protected by a CRC-16-CCITT-FALSE checksum
/// computed over the data type signature followed by the transfer payload, as
/// specified by DroneCAN. The checksum result is provided in the [`Payload`]
/// returned when the transfer completes. Single-frame transfers are not
/// protected by a checksum.
///
/// If no data type signature is provided, [`Payload::crc_valid`] will always
/// be `false`.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Transfer<'a> {
    storage: ManagedSlice<'a, u8>,
    length: usize,
    transfer_id: u8,
    toggle: bool,
    signature: Option<u64>,
    expected_crc: Option<u16>,
    started: bool,
}

impl<'a> Transfer<'a> {
    /// Create a new empty transfer.
    ///
    /// Every element in `storage` will be reset.
    ///
    /// `signature` is the 64-bit data type signature of the transferred data
    /// type, used to verify the checksum of multi-frame transfers.
    ///
    /// If `None`, [`Payload::crc_valid`] will always be `false`.
    pub fn new<S>(storage: S, signature: Option<u64>) -> Self
    where
        S: Into<ManagedSlice<'a, u8>>,
    {
        let storage = storage.into();

        #[cfg(feature = "alloc")]
        let mut storage = storage;

        #[cfg(feature = "alloc")]
        if let ManagedSlice::Owned(vec) = &mut storage {
            vec.clear();
        }

        Self {
            storage,
            length: 0,
            transfer_id: 0,
            toggle: false,
            signature,
            expected_crc: None,
            started: false,
        }
    }

    /// Feed data frames to the ongoing transfer.
    ///
    /// `Ok(None)` is returned if the frame was accepted but the transfer is not
    /// complete. `Ok(Some(payload))` is returned when the frame completes the
    /// transfer; the payload contains the assembled data and the CRC result.
    ///
    /// If an [`Error`] is returned, the transfer should probably be abandoned.
    pub fn add_frame(&mut self, data: &[u8]) -> Result<Option<Payload<'_>>, Error> {
        if data.len() > 8 {
            return Err(Error::DataLength);
        }

        let tail = match data.last() {
            Some(d) => Tail(*d),
            None => return Err(Error::DataLength),
        };

        if tail.start() && self.started {
            // this is not the first transfer
            return Err(Error::FrameOrder);
        }

        if tail.start() {
            // the first frame of a transfer must have the toggle bit cleared
            if tail.toggle() {
                return Err(Error::Toggle);
            }

            self.transfer_id = tail.transfer_id();
            self.toggle = tail.toggle();
            self.started = true;
            self.expected_crc = None;
        } else {
            // we cannot start with a frame that isn't the start of a transfer
            if !self.started {
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
            // multi-frame transfers start with a 2-byte CRC
            if data.len() < 4 {
                return Err(Error::DataLength);
            }
            self.expected_crc = Some(u16::from_le_bytes([data[0], data[1]]));
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

        if tail.end() {
            let crc_valid = match self.expected_crc {
                Some(expected_crc) => match self.signature {
                    Some(signature) => {
                        let mut digest = CRC.digest();
                        digest.update(&signature.to_le_bytes());
                        digest.update(&self.storage[..self.length]);
                        digest.finalize() == expected_crc
                    }
                    None => false,
                },
                // Single-frame transfers do not carry a CRC.
                None => self.signature.is_some(),
            };

            Ok(Some(Payload {
                data: &self.storage[..self.length],
                crc_valid,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Newtype for interpreting the tail byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tail(u8);

impl Tail {
    /// Start of transfer.
    fn start(&self) -> bool {
        (self.0 & (1 << 7)) != 0
    }

    /// End of transfer.
    fn end(&self) -> bool {
        (self.0 & (1 << 6)) != 0
    }

    /// Toggle bit (inverts every payload).
    fn toggle(&self) -> bool {
        (self.0 & (1 << 5)) != 0
    }

    /// Transfer identifier.
    fn transfer_id(&self) -> u8 {
        self.0 & 0x1F
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `uavcan.equipment.actuator.ArrayCommand`
    const ARRAY_COMMAND_SIGNATURE: u64 = 0xD8A7486238EC3AF3;

    /// `uavcan.protocol.NodeStatus`
    const NODE_STATUS_SIGNATURE: u64 = 0x0F0868D0C1A7C6F1;

    #[test]
    fn crc_algorithm_check_value() {
        // Check value for CRC-16-CCITT-FALSE from the DroneCAN specification.
        assert_eq!(CRC.checksum(b"123456789"), 0x29B1);
    }

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
        // Single-frame transfers do not use a CRC, so no signature is needed.
        let mut transfer = Transfer::new(vec![], None);
        let payload = transfer.add_frame(&[0x01, 0x02, 0x03, 0x04, 0xDF]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x02, 0x03, 0x04],
                crc_valid: false,
            }))
        );

        let mut transfer = Transfer::new(vec![], None);
        let payload = transfer.add_frame(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0xDF]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07],
                crc_valid: false,
            }))
        );

        let mut transfer = Transfer::new(vec![], None);
        let payload = transfer.add_frame(&[0xDF]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[],
                crc_valid: false,
            }))
        );
    }

    #[test]
    fn transfer_single_with_signature_crc_valid() {
        // Single-frame transfers carry no CRC, so there is nothing to fail.
        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        let payload = transfer.add_frame(&[0x01, 0x02, 0x03, 0x04, 0xDF]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x02, 0x03, 0x04],
                crc_valid: true,
            }))
        );
    }

    #[test]
    fn transfer_single_has_no_crc() {
        // Single-frame transfers carry no CRC, so the first two bytes are
        // ordinary payload even if they happen to look like a checksum.
        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        let payload = transfer.add_frame(&[0x01, 0x98, 0x00, 0xDF]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x98, 0x00],
                crc_valid: true,
            }))
        );
    }

    #[test]
    fn transfer_first_frame_toggle_must_be_zero() {
        // Start frame with the toggle bit set is rejected.
        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        assert_eq!(
            transfer.add_frame(&[0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0xBD]),
            Err(Error::Toggle)
        );

        // Single-frame transfer with the toggle bit set is rejected too.
        let mut transfer = Transfer::new(vec![], None);
        assert_eq!(transfer.add_frame(&[0x01, 0xFD]), Err(Error::Toggle));
    }

    #[test]
    fn transfer_multi_valid_crc() {
        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        assert_eq!(
            transfer.add_frame(&[0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D]),
            Ok(None)
        );
        let payload = transfer.add_frame(&[0x00, 0x7D, 0x33, 0x7D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x00, 0x68, 0xB5, 0x02, 0x00, 0x7D, 0x33],
                crc_valid: true,
            }))
        );
    }

    #[test]
    fn transfer_multi_without_signature_crc_invalid() {
        // Without a signature the checksum cannot be verified.
        let mut transfer = Transfer::new(vec![], None);
        assert_eq!(
            transfer.add_frame(&[0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D]),
            Ok(None)
        );
        let payload = transfer.add_frame(&[0x00, 0x7D, 0x33, 0x7D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x00, 0x68, 0xB5, 0x02, 0x00, 0x7D, 0x33],
                crc_valid: false,
            }))
        );
    }

    #[test]
    fn transfer_multi_invalid_payload_crc() {
        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        assert_eq!(
            transfer.add_frame(&[0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D]),
            Ok(None)
        );
        // Last payload byte changed from 0x33 to 0x34.
        let payload = transfer.add_frame(&[0x00, 0x7D, 0x34, 0x7D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x00, 0x68, 0xB5, 0x02, 0x00, 0x7D, 0x34],
                crc_valid: false,
            }))
        );
    }

    #[test]
    fn transfer_multi_invalid_crc_field() {
        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        // CRC changed from 0x9801 to 0x9802.
        assert_eq!(
            transfer.add_frame(&[0x02, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D]),
            Ok(None)
        );
        let payload = transfer.add_frame(&[0x00, 0x7D, 0x33, 0x7D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x00, 0x68, 0xB5, 0x02, 0x00, 0x7D, 0x33],
                crc_valid: false,
            }))
        );
    }

    #[test]
    fn transfer_multi_crc_is_little_endian() {
        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        // CRC bytes swapped: 0x9801 transmitted as [0x98, 0x01].
        assert_eq!(
            transfer.add_frame(&[0x98, 0x01, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D]),
            Ok(None)
        );
        let payload = transfer.add_frame(&[0x00, 0x7D, 0x33, 0x7D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x00, 0x68, 0xB5, 0x02, 0x00, 0x7D, 0x33],
                crc_valid: false,
            }))
        );
    }

    #[test]
    fn transfer_multi_crc_includes_signature() {
        // The same frames are valid for ArrayCommand but not for NodeStatus.
        let mut transfer = Transfer::new(vec![], Some(NODE_STATUS_SIGNATURE));
        assert_eq!(
            transfer.add_frame(&[0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D]),
            Ok(None)
        );
        let payload = transfer.add_frame(&[0x00, 0x7D, 0x33, 0x7D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x00, 0x68, 0xB5, 0x02, 0x00, 0x7D, 0x33],
                crc_valid: false,
            }))
        );
    }

    #[test]
    fn transfer_multi_crc_across_frames() {
        // 10-byte payload split over two frames.
        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        assert_eq!(
            transfer.add_frame(&[0xA7, 0xA1, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x9D]),
            Ok(None)
        );
        let payload = transfer.add_frame(&[0xFF, 0x11, 0x22, 0x33, 0x44, 0x7D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22, 0x33, 0x44],
                crc_valid: true,
            }))
        );
    }

    #[test]
    fn transfer_multi_crc_across_three_frames() {
        // The same 10-byte payload split over three frames.
        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        assert_eq!(
            transfer.add_frame(&[0xA7, 0xA1, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x9D]),
            Ok(None)
        );
        assert_eq!(transfer.add_frame(&[0xFF, 0x11, 0x22, 0x3D]), Ok(None));
        let payload = transfer.add_frame(&[0x33, 0x44, 0x5D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22, 0x33, 0x44],
                crc_valid: true,
            }))
        );
    }

    #[test]
    fn transfer_multi_crc_short_end_frame() {
        // 6-byte payload split 5/1.
        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        assert_eq!(
            transfer.add_frame(&[0x01, 0x42, 0x01, 0x02, 0x03, 0x04, 0x05, 0x9D]),
            Ok(None)
        );
        let payload = transfer.add_frame(&[0x06, 0x7D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
                crc_valid: true,
            }))
        );
    }

    #[test]
    fn transfer_multi_crc_empty_end_frame() {
        // 5-byte payload followed by an end frame with no payload.
        let mut transfer = Transfer::new(vec![], Some(0x0123456789ABCDEF));
        assert_eq!(
            transfer.add_frame(&[0x4A, 0xD3, 0xDE, 0xAD, 0xBE, 0xEF, 0x42, 0x9D]),
            Ok(None)
        );
        let payload = transfer.add_frame(&[0x7D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0xDE, 0xAD, 0xBE, 0xEF, 0x42],
                crc_valid: true,
            }))
        );
    }

    #[test]
    fn transfer_multi_crc_node_status() {
        // 9-byte payload of zeroes for uavcan.protocol.NodeStatus.
        let mut transfer = Transfer::new(vec![], Some(NODE_STATUS_SIGNATURE));
        assert_eq!(
            transfer.add_frame(&[0x61, 0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x9D]),
            Ok(None)
        );
        let payload = transfer.add_frame(&[0x00, 0x00, 0x00, 0x00, 0x7D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x00; 9],
                crc_valid: true,
            }))
        );
    }

    #[test]
    fn transfer_multi_crc_borrowed_storage() {
        let mut storage = [0; 8];
        let mut transfer = Transfer::new(storage.as_mut_slice(), Some(ARRAY_COMMAND_SIGNATURE));
        assert_eq!(
            transfer.add_frame(&[0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D]),
            Ok(None)
        );
        let payload = transfer.add_frame(&[0x00, 0x7D, 0x33, 0x7D]);
        assert_eq!(
            payload,
            Ok(Some(Payload {
                data: &[0x01, 0x00, 0x68, 0xB5, 0x02, 0x00, 0x7D, 0x33],
                crc_valid: true,
            }))
        );
    }

    #[test]
    fn transfer_multi_short_start_frame() {
        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        assert_eq!(transfer.add_frame(&[0x01, 0x9D]), Err(Error::DataLength));

        let mut transfer = Transfer::new(vec![], Some(ARRAY_COMMAND_SIGNATURE));
        assert_eq!(
            transfer.add_frame(&[0x01, 0x98, 0x9D]),
            Err(Error::DataLength)
        );
    }

    #[test]
    fn transfer_buffer_too_small() {
        let mut storage = [0; 7]; // 1 byte too small
        let mut transfer = Transfer::new(storage.as_mut_slice(), Some(ARRAY_COMMAND_SIGNATURE));
        assert_eq!(
            transfer.add_frame(&[0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D]),
            Ok(None)
        );
        assert_eq!(
            transfer.add_frame(&[0x00, 0x7D, 0x33, 0x7D]),
            Err(Error::BufferTooSmall)
        );
    }
}
