use core::mem;

use uefi::{proto::Protocol, unsafe_guid, Error, Result, Status};

#[repr(C)]
#[unsafe_guid("afbfde41-2e6e-4262-ba65-62b9236e5495")]
#[derive(Protocol)]
pub struct Timestamp {
    get_timestamp: extern "efiapi" fn() -> u64,
    get_properties: extern "efiapi" fn(*mut TimestampProperties) -> Status,
}

impl Timestamp {
    pub fn get_timestamp(&mut self) -> u64 {
        (self.get_timestamp)()
    }

    pub fn get_properties(&mut self) -> Result<TimestampProperties> {
        let mut properties = unsafe { mem::zeroed::<TimestampProperties>() };
        let status = (self.get_properties)(&mut properties);
        if status == Status::SUCCESS {
            Ok(properties)
        } else {
            Err(Error::new(status, ()))
        }
    }
}

#[repr(C)]
pub struct TimestampProperties {
    frequency: u64,
    end_value: u64,
}

impl TimestampProperties {
    pub fn frequency(&self) -> u64 {
        self.frequency
    }

    pub fn end_value(&self) -> u64 {
        self.end_value
    }
}
