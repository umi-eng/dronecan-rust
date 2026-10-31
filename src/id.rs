use thiserror::Error;

/// Error returned when an [`Id`] cannot be constructed from the given fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum InvalidId {
    /// The priority exceeds the maximum of 31.
    #[error("priority {0} exceeds the maximum of 31")]
    Priority(u8),
    /// The source node ID is out of range.
    #[error("source node id {0} is out of range")]
    SourceNode(u8),
    /// The destination node ID is out of range.
    #[error("destination node id {0} is out of range")]
    DestinationNode(u8),
    /// The discriminator exceeds the maximum of 16383.
    #[error("discriminator {0} exceeds the maximum of 16383")]
    Discriminator(u16),
}

/// DroneCAN identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Id {
    #[non_exhaustive]
    Message {
        /// Message priority.
        priority: u8,
        /// Message type ID.
        type_id: u16,
        /// Source node ID.
        source_node: u8,
    },
    #[non_exhaustive]
    Anonymous {
        /// Message priority.
        priority: u8,
        /// Discrinimator value.
        discriminator: u16,
        /// Message type ID (lower bits).
        type_id: u8,
    },
    #[non_exhaustive]
    Service {
        /// Message priority.
        priority: u8,
        /// Service type ID.
        service_type: u8,
        /// Is the message a request?
        request: bool,
        /// Destination node ID.
        destination_node: u8,
        /// Source node ID.
        source_node: u8,
    },
}

impl Id {
    /// Create a new ['Id'] from a raw identifier value.
    ///
    /// Masked to 29 bits to ensure the id is valid.
    pub fn new(raw: u32) -> Self {
        let raw = raw & embedded_can::ExtendedId::MAX.as_raw();

        let priority = (raw >> 24) as u8;
        let source_node = (raw & 0x7F) as u8;
        let service_not_message = (raw & (1 << 7)) != 0;

        if service_not_message {
            Self::Service {
                priority,
                service_type: ((raw >> 16) & 0xFF) as u8,
                request: (raw & (1 << 15)) != 0,
                destination_node: ((raw >> 8) & 0x7F) as u8,
                source_node,
            }
        } else if source_node == 0 {
            Self::Anonymous {
                priority,
                discriminator: ((raw >> 10) & 0x3FFF) as u16,
                type_id: ((raw >> 8) & 0x3) as u8,
            }
        } else {
            Self::Message {
                priority,
                type_id: ((raw >> 8) & 0xFFFF) as u16,
                source_node,
            }
        }
    }

    /// Create a message identifier.
    ///
    /// - `source_node` source node identifier `1..=127`
    /// - `type_id` message type identifier
    /// - `priority` message priority `1..=31`
    pub fn message(source_node: u8, type_id: u16, priority: u8) -> Result<Self, InvalidId> {
        if priority > 0x1F {
            return Err(InvalidId::Priority(priority));
        }

        // cannot be an anonymous message
        if source_node == 0 || source_node > 0x7F {
            return Err(InvalidId::SourceNode(source_node));
        }

        Ok(Self::Message {
            priority,
            type_id,
            source_node,
        })
    }

    /// Create an anonymous identifier.
    ///
    /// - `type_id` message type identifier masked to the two lowest bits -
    ///   `discriminator` a field which should be filled with random data to make
    ///   anonymous frames unique. Valid values `1..=16383`
    /// - `priority` message priority `1..=31`
    pub fn anonymous(type_id: u16, discriminator: u16, priority: u8) -> Result<Self, InvalidId> {
        if priority > 0x1F {
            return Err(InvalidId::Priority(priority));
        }

        if discriminator > 0x3FFF {
            return Err(InvalidId::Discriminator(discriminator));
        }

        let type_id = (type_id & 0x3) as u8;

        Ok(Self::Anonymous {
            priority,
            discriminator,
            type_id,
        })
    }

    /// Create a service identifier.
    ///
    /// - `source_node` source node identifier `1..=127`
    /// - `destination_node` destination node identifier `1..=127`
    /// - `service_type` data type identifier of the encoded service request or response
    /// - `request` request `true` or response `false`
    /// - `priority` message priority `1..=31`
    pub fn service(
        source_node: u8,
        destination_node: u8,
        service_type: u8,
        request: bool,
        priority: u8,
    ) -> Result<Self, InvalidId> {
        if priority > 0x1F {
            return Err(InvalidId::Priority(priority));
        }

        if source_node == 0 || source_node > 0x7F {
            return Err(InvalidId::SourceNode(source_node));
        }

        if destination_node == 0 || destination_node > 0x7F {
            return Err(InvalidId::DestinationNode(destination_node));
        }

        Ok(Self::Service {
            priority,
            service_type,
            request,
            destination_node,
            source_node,
        })
    }

    pub fn as_raw(&self) -> u32 {
        let mut raw = 0_u32;

        match *self {
            Self::Message {
                priority,
                type_id,
                source_node,
            } => {
                raw |= (priority as u32 & 0x1F) << 24;
                raw |= (type_id as u32) << 8;
                raw |= (source_node as u32) & 0x7F;
            }
            Self::Anonymous {
                priority,
                discriminator,
                type_id,
            } => {
                raw |= (priority as u32 & 0x1F) << 24;
                raw |= (discriminator as u32 & 0x3FFF) << 10;
                raw |= (type_id as u32 & 0x3) << 8;
            }
            Self::Service {
                priority,
                service_type,
                request,
                destination_node,
                source_node,
            } => {
                raw |= (priority as u32 & 0x1F) << 24;
                raw |= (service_type as u32) << 16;
                raw |= (request as u32) << 15;
                raw |= (destination_node as u32 & 0x7F) << 8;
                raw |= 1 << 7; // is service
                raw |= source_node as u32 & 0x7F;
            }
        }

        raw
    }

    /// Message priority.
    pub fn priority(&self) -> u8 {
        match self {
            Self::Message { priority, .. } => *priority,
            Self::Anonymous { priority, .. } => *priority,
            Self::Service { priority, .. } => *priority,
        }
    }
}

impl From<embedded_can::ExtendedId> for Id {
    fn from(value: embedded_can::ExtendedId) -> Self {
        Self::new(value.as_raw())
    }
}

/// Error returned when an [`Id`] cannot be represented as an
/// [`embedded_can::ExtendedId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[error("id cannot be represented as an extended CAN id")]
pub struct InvalidExtendedId;

impl TryFrom<Id> for embedded_can::ExtendedId {
    type Error = InvalidExtendedId;

    fn try_from(value: Id) -> Result<Self, Self::Error> {
        Self::new(value.as_raw()).ok_or(InvalidExtendedId)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_ids() {
        assert!(Id::message(123, 123, 31).is_ok());
        assert_eq!(Id::message(123, 123, 32), Err(InvalidId::Priority(32)));
        assert_eq!(Id::message(0, 123, 31), Err(InvalidId::SourceNode(0)));
        assert_eq!(Id::message(128, 123, 31), Err(InvalidId::SourceNode(128)));

        assert!(Id::anonymous(123, 123, 31).is_ok());
        assert_eq!(Id::anonymous(123, 123, 32), Err(InvalidId::Priority(32)));
        assert_eq!(
            Id::anonymous(123, 0x4000, 31),
            Err(InvalidId::Discriminator(0x4000))
        );

        assert!(Id::service(123, 123, 123, false, 31).is_ok());
        assert_eq!(
            Id::service(123, 123, 123, false, 32),
            Err(InvalidId::Priority(32))
        );
        assert_eq!(
            Id::service(0, 123, 123, false, 31),
            Err(InvalidId::SourceNode(0))
        );
        assert_eq!(
            Id::service(128, 123, 123, false, 31),
            Err(InvalidId::SourceNode(128))
        );
        assert_eq!(
            Id::service(123, 0, 123, false, 31),
            Err(InvalidId::DestinationNode(0))
        );
        assert_eq!(
            Id::service(123, 128, 123, false, 31),
            Err(InvalidId::DestinationNode(128))
        );
        assert_eq!(
            Id::service(0, 0, 123, false, 31),
            Err(InvalidId::SourceNode(0))
        );
    }

    #[test]
    fn to_extended_id() {
        assert_eq!(
            embedded_can::ExtendedId::try_from(Id::new(0x1F0155FA)).ok(),
            embedded_can::ExtendedId::new(0x1F0155FA)
        );
    }

    #[test]
    fn to_from_raw() {
        assert_eq!(Id::new(0x0803F20A).as_raw(), 0x0803F20A); // message
        assert_eq!(Id::new(0x184E270A).as_raw(), 0x184E270A); // message
        assert_eq!(Id::new(0x104E2D00).as_raw(), 0x104E2D00); // anonymous
        assert_eq!(Id::new(0x1F0155FA).as_raw(), 0x1F0155FA); // service
    }

    /// `uavcan.equipment.actuator.ArrayCommand`
    ///
    /// [Reference](https://dronecan.github.io/Specification/7._List_of_standard_data_types/#arraycommand)
    #[test]
    fn uavcan_equipment_actuator_array_command() {
        assert_eq!(
            Id::new(0x0803F20A),
            Id::Message {
                priority: 8,
                type_id: 1010,
                source_node: 10,
            }
        )
    }

    /// `ardupilot.indication.NotifyState`
    ///
    /// [Reference](https://dronecan.github.io/Specification/7._List_of_standard_data_types/#notifystate)
    #[test]
    fn ardupilot_indication_notify_state() {
        assert_eq!(
            Id::new(0x184E270A),
            Id::Message {
                priority: 24,
                type_id: 20007,
                source_node: 10,
            }
        )
    }
}
