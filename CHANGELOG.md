# Changelog

## [Unreleased]

- Replace the infallible `From<Id> for embedded_can::ExtendedId` conversion
  with `TryFrom<Id>`, which returns the new `InvalidExtendedId` error instead
  of panicking.
- Reject the first frame of a transfer if its toggle bit is set, returning
  `Error::Toggle`.
- Verify the CRC-16-CCITT-FALSE checksum of multi-frame transfers.
  `Transfer::new` now takes an optional data type signature. `add_frame`
  returns a `Payload` containing the data and CRC result once the transfer
  completes.

## v0.1.0

- Initial release.
