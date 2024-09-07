#![doc = include_str!("../README.md")]
mod format;
mod protocol;
mod result;
mod types;
mod utils;
mod uuids;

#[cfg(feature = "metrics")]
mod metrics;

use btleplug::{
    api::{
        BDAddr, Central, CentralEvent, CharPropFlags, Characteristic, Peripheral, ScanFilter,
        Service, ValueNotification, WriteType,
    },
    platform::{Adapter, Peripheral as Periphery, PeripheralId as PeripheryId},
};
use core::{pin::Pin, time::Duration};
use futures::stream::{Stream, StreamExt};
use pretty_hex::PrettyHex;
use std::sync::Arc;
use tokio::{
    sync::{Mutex, RwLock},
    time::timeout,
};
use tracing as log;
use uuid::Uuid;

pub use format::Format;
pub use macaddr::MacAddr6 as MacAddr;
pub use result::{Error, Result};
pub use types::{CellData, DeviceId, DeviceInfo};

#[cfg(feature = "metrics")]
pub use metrics::{Metrics, Scrapeable};

use protocol::{MessageIter, MessageType, RawRecord, RawRequest, RawResponse};
use utils::checksum;

impl DeviceId {
    pub async fn match_adapter(&self, adapter: &Adapter) -> Result<bool> {
        let info = adapter.adapter_info().await?;

        Ok(match self {
            DeviceId::Mac(mac) => info.contains(&mac.to_string()),
            DeviceId::Name(name) => info.contains(name),
        })
    }

    pub async fn match_periphery(&self, periphery: &Periphery) -> Result<bool> {
        Ok(match self {
            DeviceId::Mac(mac) => periphery.address().as_ref() == mac.as_bytes(),
            DeviceId::Name(name) => {
                periphery
                    .properties()
                    .await?
                    .and_then(|props| props.local_name.map(|local_name| &local_name == name))
                    .unwrap_or(false)
                /*
                if let Some(device_name_characteristic) = find_service_characteristic(
                    periphery,
                    &uuids::service::GENERIC_ACCESS,
                    &uuids::characteristic::DEVICE_NAME,
                    CharPropFlags::READ,
                ) {
                    periphery.read(&device_name_characteristic).await? == name.as_bytes()
                } else {
                    false
                }
                */
            }
        })
    }
}

/// Client
pub struct Client {
    device_id: DeviceId,
    adapter: Adapter,
    periphery_id: Arc<RwLock<Option<PeripheryId>>>,
    data_buffer: Mutex<DataBuffer>,
    options: Options,
}

struct DataBuffer {
    raw: Vec<u8>,
}

impl core::ops::Deref for DataBuffer {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl core::ops::DerefMut for DataBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.raw
    }
}

impl Default for DataBuffer {
    fn default() -> Self {
        let raw = Vec::with_capacity(512);
        Self { raw }
    }
}

impl DataBuffer {
    fn init(&mut self) {
        self.raw.clear();
    }

    fn add_crc(&mut self) {
        let crc = checksum(None, &self.raw);
        self.raw.push(crc);
    }

    fn add_data(&mut self, data: impl AsRef<[u8]>) {
        self.raw.extend(data.as_ref());
    }

    fn check_data_crc(&self) -> bool {
        if self.raw.len() < protocol::RESPONSE_HEADER.len() + 1 {
            return false;
        }

        self.crc() == checksum(None, self.data())
    }

    fn crc(&self) -> u8 {
        let len = self.raw.len() - 1;
        self.raw[len]
    }

    fn data(&self) -> &[u8] {
        let len = self.raw.len() - 1;
        &self.raw[..len]
    }

    fn data_as<'d, T: TryFrom<&'d [u8], Error = Error>>(&'d self) -> Result<T> {
        self.data().try_into()
    }
}

/// Client options
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Options {
    pub scan_timeout: Duration,
    pub request_timeout: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scan_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(5),
        }
    }
}

impl Client {
    /// Create client for BMC device
    pub fn new(adapter: &Adapter, device_id: &DeviceId, options: &Options) -> Self {
        let adapter = adapter.clone();
        let device_id = device_id.clone();
        let periphery_id = Arc::new(RwLock::new(None));
        let options = *options;
        let data_buffer = Mutex::new(DataBuffer::default());
        Self {
            device_id,
            adapter,
            periphery_id,
            data_buffer,
            options,
        }
    }

    /// Connect to device if not connected
    pub async fn open(&self) -> Result<()> {
        let periphery = self.find_periphery().await?;

        if periphery.is_connected().await? {
            log::debug!("Periphery already connected: {periphery:?}");
            //periphery.disconnect().await?;
        } else {
            log::debug!("Connect periphery: {periphery:?}");
            periphery.connect().await?;
        }

        Ok(())
    }

    /// Disconnect from device if connected
    pub async fn close(&self) -> Result<()> {
        if let Some(periphery_id) = self.get_periphery_id().await {
            let periphery = self.adapter.peripheral(&periphery_id).await?;
            {
                if periphery.is_connected().await? {
                    log::debug!("Disconnect periphery: {periphery:?}");
                    periphery.disconnect().await?;
                }
            }
        }
        Ok(())
    }

    /// Get device identifier
    pub fn device_id(&self) -> &DeviceId {
        &self.device_id
    }

    /// Get bluetooth device MAC address
    pub async fn address(&self) -> Result<BDAddr> {
        let periphery = self.get_periphery().await?;

        Ok(periphery.address())
    }

    /// Get bluetooth device MAC address
    pub async fn mac_address(&self) -> Result<MacAddr> {
        self.address()
            .await
            .map(|address| address.into_inner().into())
    }

    /// Get bluetooth device name
    pub async fn device_name(&self) -> Result<String> {
        let periphery = self.get_periphery().await?;

        /*
        let characteristic = if let Some(characteristic) = find_service_characteristic(
            &periphery,
            &uuids::service::GENERIC_ACCESS,
            &uuids::characteristic::DEVICE_NAME,
            CharPropFlags::READ,
        ) {
            characteristic
        } else {
            periphery.discover_services().await?;
            find_service_characteristic(
                &periphery,
                &uuids::service::GENERIC_ACCESS,
                &uuids::characteristic::DEVICE_NAME,
                CharPropFlags::READ,
            )
            .ok_or(Error::NotFound)?
        };

        let device_name = periphery.read(&characteristic).await?;

        Ok(String::from_utf8(device_name)?)
        */

        periphery
            .properties()
            .await?
            .and_then(|props| props.local_name)
            .ok_or(Error::NotFound)
    }

    /// Get device info
    pub async fn device_info(&self) -> Result<DeviceInfo> {
        let mut data_buffer = self.make_request(&0x97.into()).await;

        self.send_request(&mut data_buffer, 0x03.into()).await?;

        data_buffer.data_as::<DeviceInfo>()
    }

    /// Get device data
    pub async fn cell_data(&self) -> Result<CellData> {
        let mut data_buffer = self.make_request(&0x96.into()).await;

        self.send_request(&mut data_buffer, 0x02.into()).await?;

        data_buffer.data_as::<CellData>()
    }

    async fn make_request(&self, cmd: &RawRequest) -> tokio::sync::MutexGuard<'_, DataBuffer> {
        let mut data_buffer = self.data_buffer.lock().await;
        data_buffer.init();
        data_buffer.add_data(cmd);
        data_buffer
    }

    async fn send_request(
        &self,
        data_buffer: &mut DataBuffer,
        record_type: Option<u8>,
    ) -> Result<()> {
        let periphery = self.get_periphery().await?;

        periphery.discover_services().await?;

        let characteristic = find_service_characteristic(
            &periphery,
            &uuids::service::JK_BMS,
            &uuids::characteristic::JK_BMS,
            CharPropFlags::WRITE_WITHOUT_RESPONSE | CharPropFlags::NOTIFY,
        )
        .ok_or(Error::NotFound)?;

        periphery.subscribe(&characteristic).await?;

        let res = timeout(
            self.options.request_timeout,
            self.process_request(&periphery, &characteristic, record_type, data_buffer),
        )
        .await
        .map_err(From::from)
        .unwrap_or_else(Err);

        periphery.unsubscribe(&characteristic).await?;

        if let Err(error) = &res {
            log::error!("Request failed with: {error:?}");
        }

        res
    }

    async fn receive_messages(
        notifications: &mut Pin<Box<dyn Stream<Item = ValueNotification> + Send>>,
        characteristic: &Characteristic,
        message_type: MessageType,
        record_type: Option<u8>,
        mut data_buffer: Option<&mut DataBuffer>,
    ) -> Result<()> {
        let mut msg_count = 0;

        while let Some(data) = notifications.next().await {
            if data.uuid != characteristic.uuid {
                continue;
            }
            log::trace!("Received notification");
            //log::trace!("{:?}", data.value.hex_dump());

            for message in MessageIter::from(data.value.as_slice()) {
                log::trace!("Received message #{msg_count}");
                log::trace!("{:?}", message.hex_dump());

                match <&RawResponse>::try_from(message) {
                    Ok(res) => {
                        if msg_count > 0 {
                            if res.message_type().is_some() {
                                log::trace!("End message");
                                return Ok(());
                            } else {
                                log::trace!("Continue message");
                                if let Some(data_buffer) = &mut data_buffer {
                                    data_buffer.add_data(message);
                                }
                                msg_count += 1;
                            }
                        } else if res
                            .message_type()
                            .map(|msg_type| msg_type == message_type)
                            .unwrap_or(false)
                            && record_type
                                .map(|record_type| {
                                    <&RawRecord>::try_from(message)
                                        .map(|res| res.record_type == record_type)
                                        .unwrap_or(false)
                                })
                                .unwrap_or(true)
                        {
                            log::trace!("Start message");
                            if let Some(data_buffer) = &mut data_buffer {
                                data_buffer.add_data(message);
                            }
                            msg_count += 1;
                        }
                    }
                    Err(error) => {
                        log::warn!("Error while repr message: {error:?}");
                    }
                }
            }
        }

        Ok(())
    }

    async fn process_request(
        &self,
        periphery: &Periphery,
        characteristic: &Characteristic,
        record_type: Option<u8>,
        data_buffer: &mut DataBuffer,
    ) -> Result<()> {
        data_buffer.add_crc();

        let mut notifications = periphery.notifications().await?;

        log::trace!("Send request");
        log::trace!("{:?}", data_buffer.hex_dump());

        periphery
            .write(characteristic, data_buffer, WriteType::WithoutResponse)
            .await?;

        data_buffer.init();

        Self::receive_messages(
            &mut notifications,
            characteristic,
            MessageType::Response,
            record_type,
            Some(data_buffer),
        )
        .await?;

        log::trace!("Received response");
        log::trace!("{:?}", data_buffer.hex_dump());

        if data_buffer
            .data_as::<&RawRecord>()
            .ok()
            .and_then(|res| {
                res.response
                    .message_type()
                    .map(|message_type| message_type == MessageType::Response)
            })
            .unwrap_or(false)
        {
            if !data_buffer.check_data_crc() {
                return Err(Error::BadCrc);
            }
        } else {
            return Err(Error::LostConnection);
        }

        Ok(())
    }

    async fn get_periphery_id(&self) -> Option<PeripheryId> {
        self.periphery_id.read().await.clone()
    }

    async fn set_periphery_id(&self, periphery_id: Option<PeripheryId>) {
        *self.periphery_id.write().await = periphery_id;
    }

    async fn get_periphery(&self) -> Result<Periphery> {
        if let Some(periphery_id) = self.get_periphery_id().await {
            if let Ok(periphery) = self.adapter.peripheral(&periphery_id).await {
                return Ok(periphery);
            }
        }
        self.set_periphery_id(None).await;
        Err(Error::LostConnection)
    }

    async fn find_periphery(&self) -> Result<Periphery> {
        // try use already known
        if let Some(periphery_id) = self.get_periphery_id().await {
            if let Ok(periphery) = self.adapter.peripheral(&periphery_id).await {
                return Ok(periphery);
            }
        }

        // try find by device id
        for periphery in self.adapter.peripherals().await? {
            if self.device_id.match_periphery(&periphery).await? {
                self.set_periphery_id(periphery.id().clone().into()).await;
                return Ok(periphery);
            }
        }

        log::info!("Start scan peripherals");
        self.adapter
            .start_scan(ScanFilter {
                services: vec![uuids::service::JK_BMS],
            })
            .await?;

        let scan_result = timeout(self.options.scan_timeout, self.scan())
            .await
            .map_err(From::from)
            .unwrap_or_else(Err);

        log::info!("Stop scan peripherals");
        if let Err(error) = self.adapter.stop_scan().await {
            log::error!("Error while stopping scan: {error}");
        }

        match &scan_result {
            Ok(periphery) => self.set_periphery_id(periphery.id().clone().into()).await,
            Err(error) => log::error!("Error while scanning peripherals: {error}"),
        }

        scan_result
    }

    async fn scan(&self) -> Result<Periphery> {
        let mut events = self.adapter.events().await?;

        while let Some(event) = events.next().await {
            log::trace!("Adapter event: {event:?}");
            if let CentralEvent::DeviceDiscovered(periphery_id) = event {
                let periphery = self.adapter.peripheral(&periphery_id).await?;
                if check_service(&periphery, &uuids::service::JK_BMS).await?
                    && self.device_id.match_periphery(&periphery).await?
                {
                    log::info!("Found peripheral: {periphery:?}");
                    return Ok(periphery);
                }
            }
        }

        Err(Error::NotFound)
    }

    /// Find BMC devices
    pub async fn find(adapter: &Adapter, options: &Options) -> Result<Vec<DeviceId>> {
        log::info!("Start scan peripherals");
        adapter
            .start_scan(ScanFilter {
                services: vec![uuids::service::JK_BMS],
            })
            .await?;

        let mut found_peripheries = Vec::default();

        let scan_result = timeout(
            options.scan_timeout,
            Self::scan_all(adapter, &mut found_peripheries),
        )
        .await
        .or_else(|_| Ok(Ok(()))) // ignore timeout
        .unwrap_or_else(Err);

        log::info!("Stop scan peripherals");
        if let Err(error) = adapter.stop_scan().await {
            log::error!("Error while stopping scan: {error}");
        }

        if let Err(error) = &scan_result {
            log::error!("Error while scanning peripherals: {error}");
        }

        scan_result?;

        Ok(found_peripheries)
    }

    async fn scan_all(adapter: &Adapter, found_peripheries: &mut Vec<DeviceId>) -> Result<()> {
        let mut events = adapter.events().await?;

        while let Some(event) = events.next().await {
            log::trace!("Adapter event: {event:?}");
            if let CentralEvent::DeviceDiscovered(periphery_id) = event {
                let periphery = adapter.peripheral(&periphery_id).await?;
                if check_service(&periphery, &uuids::service::JK_BMS).await? {
                    log::info!("Found peripheral: {periphery:?}");
                    found_peripheries.push(DeviceId::Mac(periphery.address().into_inner().into()));
                }
            }
        }

        Err(Error::NotFound)
    }
}

async fn check_service(periphery: &Periphery, service_uuid: &Uuid) -> Result<bool> {
    Ok(periphery
        .properties()
        .await?
        .map(|props| props.services.iter().any(|uuid| uuid == service_uuid))
        .unwrap_or(false))
}

fn find_service(periphery: &Periphery, service_uuid: &Uuid) -> Option<Service> {
    log::trace!("Services: {:?}", periphery.services());
    periphery
        .services()
        .iter()
        .find(|service| &service.uuid == service_uuid)
        .cloned()
}

fn find_service_characteristic(
    periphery: &Periphery,
    service_uuid: &Uuid,
    characteristic_uuid: &Uuid,
    characteristic_properties: CharPropFlags,
) -> Option<Characteristic> {
    find_service(periphery, service_uuid).and_then(|service| {
        service
            .characteristics
            .iter()
            .find(|characteristic| {
                &characteristic.uuid == characteristic_uuid
                    && characteristic
                        .properties
                        .contains(characteristic_properties)
            })
            .cloned()
    })
}
