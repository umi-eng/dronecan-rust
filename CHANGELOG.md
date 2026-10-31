# Changelog

## [Unreleased]

- Fix `clippy::doc_lazy_continuation` warnings in the `Id::anonymous`
  documentation.
- `Id::message`, `Id::anonymous`, and `Id::service` now return
  `Result<Id, InvalidId>` instead of `Option<Id>`. The new `InvalidId` error
  reports which field was invalid. `InvalidId` and `InvalidExtendedId` now
  derive `thiserror::Error`.
- Replace the infallible `From<Id> for embedded_can::ExtendedId` conversion
  with `TryFrom<Id>`, which returns the new `InvalidExtendedId` error instead
  of panicking.
- Reject the first frame of a transfer if its toggle bit is set, returning
  `Error::Toggle`.
- Verify the CRC-16-CCITT-FALSE checksum of multi-frame transfers.
  `Transfer::new` now takes an optional data type signature. `add_frame`
  returns a `Payload` containing the data and CRC result once the transfer
  completes.
- Make `Transfer::add_frame` panic-free for short frames by splitting off the
  tail byte and CRC explicitly instead of indexing with `data.len() - 1`.
  Empty frames are rejected with `Error::DataLength`.

## v0.1.0

- Initial release.
