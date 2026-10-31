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
- `Id::service` now rejects node ID `0` for both `source_node` and
  `destination_node`, returning `InvalidId::SourceNode` and
  `InvalidId::DestinationNode` respectively. The DroneCAN spec requires both
  node IDs to be in `1..=127`; previously only values above `127` were
  rejected. Decoding raw IDs via `Id::new` is unchanged.
- Reject the first frame of a transfer if its toggle bit is set, returning
  `Error::Toggle`.
- Verify the CRC-16-CCITT-FALSE checksum of multi-frame transfers.
  `Transfer::new` now takes an optional data type signature. `add_frame`
  returns a `Payload` containing the data and CRC result once the transfer
  completes.
- Make `Transfer::add_frame` panic-free for short frames by splitting off the
  tail byte and CRC explicitly instead of indexing with `data.len() - 1`.
  Empty frames are rejected with `Error::DataLength`.
- Fix `Transfer::add_frame` committing transfer state before a frame was fully
  validated. A start frame that failed with `Error::DataLength` or
  `Error::BufferTooSmall` previously left the transfer marked as started, so a
  subsequent valid start frame was rejected with `Error::FrameOrder`. Frames are
  now validated before any state is mutated.

## v0.1.0

- Initial release.
