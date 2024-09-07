use crate::{utils::*, CellData, DeviceInfo, Error, Result};
use core::mem::size_of;

pub const HEARTBEAT: [u8; 4] = *b"AT\r\n";
pub const REQUEST_HEADER: [u8; 4] = [0xaa, 0x55, 0x90, 0xeb];
pub const RESPONSE_HEADER: [u8; 4] = [0x55, 0xaa, 0xeb, 0x90];

pub struct MessageIter<'r> {
    raw: &'r [u8],
}

impl<'r> From<&'r [u8]> for MessageIter<'r> {
    fn from(raw: &'r [u8]) -> Self {
        Self { raw }
    }
}

impl<'r> Iterator for MessageIter<'r> {
    type Item = &'r [u8];
    fn next(&mut self) -> Option<Self::Item> {
        let l = self.raw.len();
        if l > 0 {
            for i in 1..l {
                let (item, rest) = self.raw.split_at(i);
                if rest.starts_with(&HEARTBEAT)
                    || rest.starts_with(&REQUEST_HEADER)
                    || rest.starts_with(&RESPONSE_HEADER)
                {
                    self.raw = rest;
                    return Some(item);
                }
            }
            let (item, rest) = self.raw.split_at(self.raw.len());
            self.raw = rest;
            Some(item)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Default)]
#[repr(C, packed)]
pub struct RawRequest {
    pub header: [u8; 4],
    pub command_code: u8,
    pub command_data: [u8; 14],
}

impl AsRef<[u8; size_of::<Self>()]> for RawRequest {
    fn as_ref(&self) -> &[u8; size_of::<Self>()] {
        unsafe { &*(&self.header as *const _ as *const _) }
    }
}

impl AsRef<[u8]> for RawRequest {
    fn as_ref(&self) -> &[u8] {
        let raw: &[u8; size_of::<Self>()] = self.as_ref();
        raw.as_ref()
    }
}

impl From<u8> for RawRequest {
    fn from(command_code: u8) -> Self {
        Self {
            header: REQUEST_HEADER,
            command_code,
            ..Default::default()
        }
    }
}

/*impl RawRequest {
    fn with_val(mut self, val: f32) -> Self {
        self.set_val(val);
        self
    }

    fn set_val(&mut self, val: f32) {
        let val = ((val * 1000.0) as i32).to_le_bytes();
        self.command_data[0..val.len()].copy_from_slice(&val);
    }
}*/

#[derive(Clone, Copy, Default, Debug)]
#[repr(C, packed)]
pub struct RawResponse {
    pub header: [u8; 4],
}

impl From<&'_ [u8; size_of::<RawResponse>()]> for &'_ RawResponse {
    fn from(raw: &[u8; size_of::<RawResponse>()]) -> Self {
        unsafe { &*(raw as *const _ as *const _) }
    }
}

impl TryFrom<&'_ [u8]> for &'_ RawResponse {
    type Error = Error;

    fn try_from(raw: &[u8]) -> Result<Self> {
        let (raw, _) = raw.split_first_chunk().ok_or(Error::NotEnoughData)?;
        Ok(raw.into())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MessageType {
    Request,
    Response,
    Heartbeat,
}

impl RawResponse {
    pub fn message_type(&self) -> Option<MessageType> {
        Some(match self.header {
            REQUEST_HEADER => MessageType::Request,
            RESPONSE_HEADER => MessageType::Response,
            HEARTBEAT => MessageType::Heartbeat,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(C, packed)]
pub struct RawRecord {
    pub response: RawResponse,
    pub record_type: u8,
    pub record_number: u8,
}

impl From<&'_ [u8; size_of::<RawRecord>()]> for &'_ RawRecord {
    fn from(raw: &[u8; size_of::<RawRecord>()]) -> Self {
        unsafe { &*(raw as *const _ as *const _) }
    }
}

impl TryFrom<&'_ [u8]> for &'_ RawRecord {
    type Error = Error;

    fn try_from(raw: &[u8]) -> Result<Self> {
        let (raw, _) = raw.split_first_chunk().ok_or(Error::NotEnoughData)?;
        Ok(raw.into())
    }
}

impl TryFrom<&'_ RawDeviceInfo> for DeviceInfo {
    type Error = Error;

    fn try_from(raw: &'_ RawDeviceInfo) -> Result<Self> {
        if raw.record.record_type != 0x03 {
            return Err(Error::BadRecordType);
        }
        Ok(Self {
            device_model: ascii_to_string_safe("device_model", raw.device_model),
            hardware_version: ascii_to_string_safe("hardware_version", raw.hardware_version),
            software_version: ascii_to_string_safe("software_version", raw.software_version),
            up_time: u32le_to_count(&raw.up_time),
            poweron_times: u32le_to_count(&raw.poweron_times),
            device_name: ascii_to_string_safe("device_name", raw.device_name),
            device_passcode: ascii_to_string_safe("device_passcode", raw.device_passcode),
            manufacturing_date: ascii_to_string_safe("manufacturing_date", raw.manufacturing_date),
            serial_number: ascii_to_string_safe("serial_number", raw.serial_number),
            passcode: ascii_to_string_safe("passcode", raw.passcode),
            userdata: ascii_to_string_safe("userdata", raw.userdata),
            setup_passcode: ascii_to_string_safe("setup_passcode", raw.setup_passcode),
            userdata2: ascii_to_string_safe("userdata2", raw.userdata2),
        })
    }
}

impl TryFrom<&'_ [u8]> for DeviceInfo {
    type Error = Error;

    fn try_from(raw: &'_ [u8]) -> Result<Self> {
        let res: &RawDeviceInfo = raw.try_into()?;
        res.try_into()
    }
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(C, packed)]
struct RawDeviceInfo {
    record: RawRecord,
    device_model: [u8; 16],
    hardware_version: [u8; 8],
    software_version: [u8; 8],
    up_time: [u8; 4],
    poweron_times: [u8; 4],
    device_name: [u8; 16],
    device_passcode: [u8; 16],
    manufacturing_date: [u8; 8],
    serial_number: [u8; 11],
    passcode: [u8; 5],
    userdata: [u8; 16],
    setup_passcode: [u8; 16],
    userdata2: [u8; 16],
}

impl From<&'_ [u8; size_of::<RawDeviceInfo>()]> for &'_ RawDeviceInfo {
    fn from(raw: &[u8; size_of::<RawDeviceInfo>()]) -> Self {
        unsafe { &*(raw as *const _ as *const _) }
    }
}

impl TryFrom<&'_ [u8]> for &'_ RawDeviceInfo {
    type Error = Error;

    fn try_from(raw: &[u8]) -> Result<Self> {
        let (raw, _) = raw.split_first_chunk().ok_or(Error::NotEnoughData)?;
        Ok(raw.into())
    }
}

impl TryFrom<&'_ RawCellData> for CellData {
    type Error = Error;

    fn try_from(raw: &'_ RawCellData) -> Result<Self> {
        if raw.record.record_type != 0x02 {
            return Err(Error::BadRecordType);
        }
        Ok(Self {
            cell_voltage: i16les_to_values(&raw.cell_voltage, 1e-3),
            average_cell_voltage: i16le_to_value(&raw.average_cell_voltage, 1e-3),
            delta_cell_voltage: i16le_to_value(&raw.delta_cell_voltage, 1e-3),
            balance_current: i16le_to_value(&raw.balance_current, 1e-3),
            cell_resistance: i16les_to_values(&raw.cell_resistance, 1e-3),
            battery_voltage: i32le_to_value(&raw.battery_voltage, 1e-3),
            battery_power: i32le_to_value(&raw.battery_power, 1e-3),
            battery_current: i32le_to_value(&raw.battery_current, 1e-3),
            battery_temperature: i16les_to_values(&raw.battery_temperature, 1e-1),
            mosfet_temperature: i16le_to_value(
                if raw.mosfet_temperature != [0u8; 2] {
                    &raw.mosfet_temperature
                } else {
                    &raw.mosfet_temperature2
                },
                1e-1,
            ),
            remain_percent: raw.remain_percent[0],
            remain_capacity: u32le_to_value(&raw.remain_capacity, 1e-3),
            nominal_capacity: u32le_to_value(&raw.nominal_capacity, 1e-3),
            cycle_count: u32le_to_count(&raw.cycle_count),
            cycle_capacity: u32le_to_value(&raw.cycle_capacity, 1e-3),
            up_time: u32le_to_count(&raw.up_time),
        })
    }
}

impl TryFrom<&'_ [u8]> for CellData {
    type Error = Error;

    fn try_from(raw: &'_ [u8]) -> Result<Self> {
        let res: &RawCellData = raw.try_into()?;
        res.try_into()
    }
}

#[derive(Clone, Copy, Default, Debug)]
#[repr(C, packed)]
struct RawCellData {
    record: RawRecord,
    cell_voltage: [[u8; 2]; 32],
    _unknown0: [u8; 4],
    average_cell_voltage: [u8; 2],
    delta_cell_voltage: [u8; 2],
    balance_current: [u8; 2],
    cell_resistance: [[u8; 2]; 32],
    _unknown1: [u8; 6],
    battery_voltage: [u8; 4],
    battery_power: [u8; 4],
    battery_current: [u8; 4],
    battery_temperature: [[u8; 2]; 2],
    mosfet_temperature: [u8; 2],
    _unknown2: [u8; 5],
    remain_percent: [u8; 1],
    remain_capacity: [u8; 4],
    nominal_capacity: [u8; 4],
    cycle_count: [u8; 4],
    cycle_capacity: [u8; 4],
    _unknown3: [u8; 4],
    up_time: [u8; 4],
    _unknown4: [u8; 24],
    _charge_current: [u8; 2],
    _discharge_current: [u8; 2],
    _unknown5: [u8; 28],
    mosfet_temperature2: [u8; 2],
}

impl From<&'_ [u8; size_of::<RawCellData>()]> for &'_ RawCellData {
    fn from(raw: &[u8; size_of::<RawCellData>()]) -> Self {
        unsafe { &*(raw as *const _ as *const _) }
    }
}

impl TryFrom<&'_ [u8]> for &'_ RawCellData {
    type Error = Error;

    fn try_from(raw: &[u8]) -> Result<Self> {
        let (raw, _) = raw.split_first_chunk().ok_or(Error::NotEnoughData)?;
        Ok(raw.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_hex::PrettyHex;

    mod message_iter {
        use super::*;

        #[test]
        fn empty() {
            let mut i = MessageIter::from([].as_slice());

            assert!(i.next().is_none());
        }

        #[test]
        fn single() {
            let p = [1, 2, 3, 4, 5];
            let mut i = MessageIter::from(p.as_slice());

            assert_eq!(i.next(), Some(p.as_slice()));
            assert!(i.next().is_none());
        }

        #[test]
        fn single_header() {
            let mut i = MessageIter::from(HEARTBEAT.as_slice());

            assert_eq!(i.next(), Some(&HEARTBEAT[..]));
            assert!(i.next().is_none());
        }

        #[test]
        fn single_with_payload() {
            let mut m = Vec::default();
            m.extend(&REQUEST_HEADER);
            let p = [1, 2, 3, 4, 5];
            m.extend(&p);

            let mut i = MessageIter::from(m.as_slice());

            let q = i.next().unwrap();
            assert_eq!(&q[..REQUEST_HEADER.len()], &REQUEST_HEADER);
            assert_eq!(&q[REQUEST_HEADER.len()..], &p);

            assert!(i.next().is_none());
        }

        #[test]
        fn multi() {
            let mut m = Vec::default();
            let s = [0xd, 0xf, 0, 7];
            m.extend(&s);
            m.extend(&HEARTBEAT);
            m.extend(&HEARTBEAT);
            m.extend(&REQUEST_HEADER);
            let p = [1, 2, 3, 4, 5];
            m.extend(&p);
            m.extend(&RESPONSE_HEADER);
            let d = [0xa, 0xb, 0xc];
            m.extend(&d);

            let mut i = MessageIter::from(m.as_slice());

            assert_eq!(i.next(), Some(s.as_slice()));
            assert_eq!(i.next(), Some(HEARTBEAT.as_slice()));
            assert_eq!(i.next(), Some(HEARTBEAT.as_slice()));

            let q = i.next().unwrap();
            assert_eq!(&q[..REQUEST_HEADER.len()], &REQUEST_HEADER);
            assert_eq!(&q[REQUEST_HEADER.len()..], &p);

            let r = i.next().unwrap();
            assert_eq!(&r[..RESPONSE_HEADER.len()], &RESPONSE_HEADER);
            assert_eq!(&r[RESPONSE_HEADER.len()..], &d);

            assert!(i.next().is_none());
        }
    }

    mod response_parse {
        use super::*;

        #[test]
        fn device_info() {
            let raw = [
                0x55, 0xaa, 0xeb, 0x90, 0x03, 0x59, 0x4a, 0x4b, 0x5f, 0x42, 0x44, 0x34, 0x41, 0x38,
                0x53, 0x34, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x31, 0x35, 0x41, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x31, 0x35, 0x2e, 0x32, 0x36, 0x00, 0x00, 0x00, 0x7c, 0xe3, 0x18, 0x00,
                0x01, 0x00, 0x00, 0x00, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x31, 0x32, 0x33, 0x34, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x32, 0x34, 0x30, 0x38, 0x31, 0x38,
                0x00, 0x00, 0x34, 0x30, 0x35, 0x33, 0x31, 0x33, 0x31, 0x30, 0x36, 0x32, 0x39, 0x00,
                0x30, 0x30, 0x30, 0x00, 0x4a, 0x4b, 0x2d, 0x42, 0x4d, 0x53, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
                0x39, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4a, 0x4b, 0x2d, 0x42, 0x4d, 0x53,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfe, 0xff, 0xff, 0xff,
                0x1f, 0xe9, 0x05, 0x02, 0x00, 0x00, 0x00, 0x00, 0x90, 0x1f, 0x00, 0x00, 0x00, 0x00,
                0xc0, 0xd8, 0xe7, 0xf7, 0x3c, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0xdf, 0x27, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xdf, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xdf, 0x27, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, 0x08, 0x00, 0x01,
                0x64, 0x00, 0x00, 0x00, 0x5f, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x00, 0x32, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x0e, 0x00, 0x00,
                0x32, 0x32, 0x01, 0x1e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfe, 0x9f, 0x69, 0x9f, 0x0f, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x08,
            ];

            let info = <&RawDeviceInfo>::try_from(raw.as_slice()).unwrap();

            println!("{:?}", raw[..size_of::<RawDeviceInfo>()].hex_dump());
            println!("{info:02x?}");
            println!("{:?}", raw[size_of::<RawDeviceInfo>()..].hex_dump());

            let info = DeviceInfo::try_from(info).unwrap();

            println!("{info:02x?}");

            assert_eq!(info.device_model, "JK_BD4A8S4P");
            assert_eq!(info.hardware_version, "15A");
            assert_eq!(info.software_version, "15.26");
            assert_eq!(info.up_time, 1631100);
            assert_eq!(info.poweron_times, 1);
            assert_eq!(info.device_name, "abcdefgh");
            assert_eq!(info.device_passcode, "1234");
            assert_eq!(info.manufacturing_date, "240818");
            assert_eq!(info.serial_number, "40531310629");
            assert_eq!(info.passcode, "000");
            assert_eq!(info.userdata, "JK-BMS");
            assert_eq!(info.setup_passcode, "123456789");
            assert_eq!(info.userdata2, "JK-BMS");
            //assert!(false);
        }

        #[test]
        fn cell_data() {
            let raw = [
                0x55, 0xaa, 0xeb, 0x90, 0x02, 0x22, 0x50, 0x09, 0x50, 0x09, 0x50, 0x09, 0x50, 0x09,
                0x50, 0x09, 0x4f, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x3f, 0x00, 0x00, 0x00, 0x50, 0x09, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x00, 0x89, 0x00,
                0x8c, 0x00, 0x8a, 0x00, 0x8b, 0x00, 0x8b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0xe2, 0x37, 0x00, 0x00,
                0xb7, 0x08, 0x00, 0x00, 0x9c, 0x00, 0x00, 0x00, 0xee, 0x00, 0xf3, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0xe0, 0x2e, 0x00, 0x00, 0xe0, 0x2e, 0x00, 0x00,
                0x01, 0x00, 0x00, 0x00, 0x06, 0x41, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00, 0x7c, 0x7c,
                0x17, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x01, 0x00, 0x00, 0x00, 0xd2, 0x03, 0x02, 0x00,
                0x01, 0x00, 0xad, 0x69, 0x3e, 0x40, 0x00, 0x00, 0x00, 0x00, 0x96, 0x05, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x01, 0x03, 0x06, 0x01, 0x00, 0xd8, 0xdc, 0xea, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x03, 0x01, 0x30, 0xf8, 0x30, 0xf8, 0xcf, 0x03, 0xda, 0xe2, 0xcc, 0x08,
                0x9e, 0x01, 0x00, 0x00, 0x80, 0x51, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfe, 0xff, 0x7f, 0xdc, 0x0f, 0x01, 0x00,
                0x80, 0x07, 0x00, 0x00, 0x00, 0x26,
            ];

            /*let raw = [
                0x55, 0xaa, 0xeb, 0x90, 0x02, 0x97, 0x50, 0x09, 0x50, 0x09, 0x50, 0x09, 0x50, 0x09,
                0x50, 0x09, 0x50, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x3f, 0x00, 0x00, 0x00, 0x50, 0x09, 0x01, 0x00, 0x00, 0x05, 0x8a, 0x00, 0x89, 0x00,
                0x8c, 0x00, 0x8a, 0x00, 0x8b, 0x00, 0x8b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0xfe, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe3, 0x37, 0x00, 0x00,
                0xb7, 0x08, 0x00, 0x00, 0x9c, 0x00, 0x00, 0x00, 0xe8, 0x00, 0xec, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0xe0, 0x2e, 0x00, 0x00, 0xe0, 0x2e, 0x00, 0x00,
                0x01, 0x00, 0x00, 0x00, 0x64, 0x45, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00, 0xa4, 0x0f,
                0x19, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x3f, 0x00, 0x01, 0x00, 0x00, 0x00, 0xd2, 0x03, 0x02, 0x00,
                0x00, 0x00, 0xad, 0x69, 0x3e, 0x40, 0x00, 0x00, 0x00, 0x00, 0x96, 0x05, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x01, 0x03, 0x06, 0x01, 0x00, 0x6c, 0x9c, 0xfa, 0x00, 0x00, 0x00,
                0x00, 0x00, 0xfe, 0x00, 0x30, 0xf8, 0x30, 0xf8, 0xcf, 0x03, 0x02, 0x76, 0xce, 0x08,
                0xbb, 0x01, 0x00, 0x00, 0x80, 0x51, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfe, 0xff, 0x7f, 0xdc, 0x0f, 0x01, 0x00,
                0x80, 0x07, 0x00, 0x00, 0x00, 0xe7,
            ];*/

            let info = <&RawCellData>::try_from(raw.as_slice()).unwrap();

            println!("{:?}", raw[..size_of::<RawCellData>()].hex_dump());
            println!("{info:02x?}");
            println!("{:?}", raw[size_of::<RawCellData>()..].hex_dump());

            let info = CellData::try_from(info).unwrap();

            println!("{info:02x?}");

            assert_eq!(
                info.cell_voltage,
                [2.384, 2.384, 2.384, 2.384, 2.384, 2.3830001]
            );
            assert_eq!(info.average_cell_voltage, 2.384);
            assert_eq!(info.delta_cell_voltage, 0.0);
            assert_eq!(info.balance_current, 0.0);
            assert_eq!(
                info.cell_resistance,
                [0.13800001, 0.13700001, 0.14, 0.13800001, 0.13900001, 0.13900001]
            );
            assert_eq!(info.battery_voltage, 14.306001);
            assert_eq!(info.battery_power, 2.2310002);
            assert_eq!(info.battery_current, 0.156);
            assert_eq!(info.battery_temperature, [23.800001, 24.300001]);
            assert_eq!(info.mosfet_temperature, 25.9);
            assert_eq!(info.remain_percent, 100);
            assert_eq!(info.remain_capacity, 12.000001);
            assert_eq!(info.nominal_capacity, 12.000001);
            assert_eq!(info.cycle_count, 1);
            assert_eq!(info.cycle_capacity, 16.646);
            assert_eq!(info.up_time, 1539196);
            //assert!(false);
        }
    }
}
