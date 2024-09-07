use btleplug::api::bleuuid::uuid_from_u16;
use uuid::Uuid;

pub mod service {
    use super::*;

    //pub const GENERIC_ACCESS: Uuid = uuid_from_u16(0x1800);
    pub const JK_BMS: Uuid = uuid_from_u16(0xffe0);
}

pub mod characteristic {
    use super::*;

    //pub const DEVICE_NAME: Uuid = uuid_from_u16(0x2a00);
    pub const JK_BMS: Uuid = uuid_from_u16(0xffe1);
}
