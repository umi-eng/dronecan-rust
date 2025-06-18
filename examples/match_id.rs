fn main() {
    // An example identifier
    let id = dronecan::Id::new(0x0803F20A);

    // This is how we can match for it
    match id.kind() {
        dronecan::Kind::Message {
            priority: 8,
            type_id: 1010,
            source_node: 10,
        } => println!("Matched!"),

        _ => println!("Got something else..."),
    }
}
