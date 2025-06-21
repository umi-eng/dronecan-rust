/// DroneCAN identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-1", derive(defmt::Format))]
pub enum Id {
    Message {
        /// Message priority.
        priority: u8,
        /// Message type ID.
        type_id: u16,
        /// Source node ID.
        source_node: u8,
    },
    Anonymous {
        /// Message priority.
        priority: u8,
        /// Discrinimator value.
        discriminator: u16,
        /// Message type ID (lower bits).
        type_id: u8,
    },
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

impl From<Id> for embedded_can::ExtendedId {
    fn from(value: Id) -> Self {
        Self::new(value.as_raw()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
