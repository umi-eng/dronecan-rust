/// 29-bit identifier mask.
const ID_MASK: u32 = 0x1fffffff;

/// DroneCAN identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-1", derive(defmt::Format))]
pub struct Id(u32);

impl Id {
    /// Create a new ['Id'] from a raw identifier value.
    ///
    /// Masked to 29 bits to ensure the id is valid.
    pub fn new(raw: u32) -> Self {
        Self(raw & ID_MASK)
    }

    /// Parse the identifier.
    ///
    /// This will always succeed, however non-DroneCAN message may yield
    /// unexpected results.
    pub fn kind(&self) -> Kind {
        let priority = (self.0 >> 24) as u8;
        let source_node = (self.0 & 0x7F) as u8;
        let service_not_message = (self.0 & (1 << 7)) != 0;

        if service_not_message {
            Kind::Service {
                priority,
                service_type: ((self.0 >> 16) & 0xFF) as u8,
                request: (self.0 & (1 << 15)) != 0,
                destination_node: ((self.0 >> 8) & 0x7F) as u8,
                source_node,
            }
        } else if source_node == 0 {
            Kind::Anonymous {
                priority,
                discriminator: ((self.0 >> 10) & 0x3FFF) as u16,
                type_id: ((self.0 >> 8) & 0x3) as u8,
            }
        } else {
            Kind::Message {
                priority,
                type_id: ((self.0 >> 8) & 0xFFFF) as u16,
                source_node,
            }
        }
    }

    /// Message priority.
    pub fn priority(&self) -> u8 {
        (self.0 >> 24) as u8
    }
}

impl From<embedded_can::ExtendedId> for Id {
    fn from(value: embedded_can::ExtendedId) -> Self {
        Self::new(value.as_raw())
    }
}

/// Identifier message kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt-1", derive(defmt::Format))]
pub enum Kind {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `uavcan.equipment.actuator.ArrayCommand`
    ///
    /// [Reference](https://dronecan.github.io/Specification/7._List_of_standard_data_types/#arraycommand)
    #[test]
    fn uavcan_equipment_actuator_array_command() {
        assert_eq!(
            Id::new(0x0803F20A).kind(),
            Kind::Message {
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
            Id::new(0x184E270A).kind(),
            Kind::Message {
                priority: 24,
                type_id: 20007,
                source_node: 10,
            }
        )
    }
}
