use dronecan::Id;

fn main() {
    // An example identifier
    let id = Id::new(0x0803F20A);

    // This is how we can match for it
    match id {
        Id::Message {
            priority: 8,
            type_id: 1010,
            source_node: 10,
            ..
        } => println!("Matched!"),

        _ => println!("Got something else..."),
    }
}
