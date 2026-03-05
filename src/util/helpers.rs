// Put individual functions with widespread utility in here.
// For more complicated structures, consider making your own file in the /util directory.

use tf_demo_parser::demo::message::packetentities::EntityId;

// Since TF2 has an object limit of 2048, the lowest 11 bits of the handle ID represent the entity ID.
// Source: https://developer.valvesoftware.com/wiki/CHandle
#[allow(dead_code)]
pub fn handle_to_entid(handle: u32) -> EntityId {
    let entid = handle & 0x7FF;
    EntityId::from(entid)
}
