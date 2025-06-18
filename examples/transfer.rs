use dronecan::{Kind, Transfer};
use embedded_can::ExtendedId;

fn main() {
    let frames = &[
        // This frame will be ignored
        PretendFrame {
            id: ExtendedId::new(0x1234).unwrap(),
            data: vec![],
        },
        // This frame is a valid start of transfer
        PretendFrame {
            id: ExtendedId::new(0x0803F20A).unwrap(),
            data: vec![0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D],
        },
        // This frame is a valid end of transfer
        PretendFrame {
            id: ExtendedId::new(0x0803F20A).unwrap(),
            data: vec![0x00, 0x7D, 0x33, 0x7D],
        },
        // This frame is invalid because it is an end frame because we expect a
        // begin frame
        PretendFrame {
            id: ExtendedId::new(0x0803F20A).unwrap(),
            data: vec![0x00, 0x7D, 0x33, 0x7D],
        },
        // This frame is a valid start of transfer
        PretendFrame {
            id: ExtendedId::new(0x0803F20A).unwrap(),
            data: vec![0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D],
        },
        // This frame is a valid end of transfer
        PretendFrame {
            id: ExtendedId::new(0x0803F20A).unwrap(),
            data: vec![0x00, 0x7D, 0x33, 0x7D],
        },
        // This frame is a valid start of transfer...
        PretendFrame {
            id: ExtendedId::new(0x0803F20A).unwrap(),
            data: vec![0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D],
        },
        // ...but we weren't expecting another start of transfer
        PretendFrame {
            id: ExtendedId::new(0x0803F20A).unwrap(),
            data: vec![0x01, 0x98, 0x01, 0x00, 0x68, 0xB5, 0x02, 0x9D],
        },
    ];

    let mut transfer = Transfer::new(vec![]);

    for frame in frames {
        match dronecan::Id::from(frame.id).kind() {
            Kind::Message {
                priority: 8,
                type_id: 1010,
                source_node: 10,
            } => match transfer.add_frame(&frame.data) {
                Ok(Some(data)) => {
                    println!("Transfer complete with data: {:?}", data);
                    // restart the transfer
                    transfer = Transfer::new(vec![]);
                }
                Ok(None) => {
                    println!("Ingested some data.");
                }
                Err(err) => {
                    println!("{}, restarting the transfer.", err);
                    transfer = Transfer::new(vec![]);
                }
            },
            _ => println!("Got an id not part of our transfer."),
        }
    }
}

// Your platform will have its own frame type. This is our pretend one just for
// this example
struct PretendFrame {
    id: ExtendedId,
    data: Vec<u8>,
}
