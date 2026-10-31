# Changelog

## [Unreleased]

- Verify the CRC-16-CCITT-FALSE checksum of multi-frame transfers.
  `Transfer::new` now takes an optional data type signature. `add_frame`
  returns a `Payload` containing the data and CRC result once the transfer
  completes.

## v0.1.0

- Initial release.
