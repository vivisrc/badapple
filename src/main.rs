#![no_main]
#![no_std]
#![feature(abi_efiapi)]
#![feature(alloc_error_handler)]
#![feature(negative_impls)]

extern crate alloc;

mod timestamp;

use alloc::vec;
use core::{alloc::Layout, arch::asm, ffi::c_void, fmt::Write, panic::PanicInfo, ptr::NonNull};

use uefi::{
    prelude::{cstr16, entry},
    proto::{
        console::gop::{GraphicsOutput, Mode, PixelFormat},
        media::file::{File, FileAttribute, FileInfo, FileMode},
    },
    table::{
        boot::{EventType, OpenProtocolAttributes, OpenProtocolParams, SearchType, Tpl},
        Boot, SystemTable,
    },
    Event, Handle, Identify, Status,
};

use crate::timestamp::Timestamp;

static mut SYSTEM_TABLE: Option<SystemTable<Boot>> = None;

pub const FRAME_WIDTH: usize = 480;
pub const FRAME_HEIGHT: usize = 360;
pub const FRAME_RATE: usize = 30;

pub const MICROS_PER_SECOND: usize = 1_000_000;
pub const MICROS_PER_FRAME: usize = MICROS_PER_SECOND / FRAME_RATE;

#[entry]
pub fn main(image: Handle, st: SystemTable<Boot>) -> Status {
    unsafe {
        SYSTEM_TABLE = Some(st.unsafe_clone());
        ::uefi::alloc::init(st.boot_services());
        st.boot_services()
            .create_event(
                EventType::SIGNAL_EXIT_BOOT_SERVICES,
                Tpl::NOTIFY,
                Some(exit_boot_services),
                None,
            )
            .unwrap();
    }

    let timestamp_hb = st
        .boot_services()
        .locate_handle_buffer(SearchType::ByProtocol(&Timestamp::GUID))
        .ok()
        .and_then(|hb| hb.handles().first().copied());
    if timestamp_hb.is_none() {
        writeln!(stderr(), "timestamp protocol not present").unwrap();
    }
    let mut timestamp = timestamp_hb.map(|timestamp_hb| unsafe {
        st.boot_services()
            .open_protocol::<Timestamp>(
                OpenProtocolParams {
                    handle: timestamp_hb,
                    agent: image,
                    controller: None,
                },
                OpenProtocolAttributes::GetProtocol,
            )
            .expect("failed to open timstamp protocol")
    });
    let (ts_frequency, ts_end_value) = match timestamp.as_deref_mut() {
        Some(timestamp) => {
            let ts_properties = timestamp
                .get_properties()
                .expect("couldn't get timestamp properties");
            (
                ts_properties.frequency() as usize,
                ts_properties.end_value(),
            )
        }
        None => (0, 0),
    };

    let graphics_hb = st
        .boot_services()
        .locate_handle_buffer(SearchType::ByProtocol(&GraphicsOutput::GUID))
        .ok()
        .and_then(|hb| hb.handles().first().copied())
        .expect("failed to locate graphics handle buffer");
    let mut graphics = unsafe {
        st.boot_services()
            .open_protocol::<GraphicsOutput>(
                OpenProtocolParams {
                    handle: graphics_hb,
                    agent: image,
                    controller: None,
                },
                OpenProtocolAttributes::GetProtocol,
            )
            .expect("failed to open graphics output protocol")
    };
    writeln!(stdout(), "opened graphics output protocol").unwrap();
    let mode = graphics
        .modes()
        .fold(None::<Mode>, |best, mode| {
            if mode.info().pixel_format() == PixelFormat::BltOnly {
                return best;
            }
            match best {
                Some(best) => {
                    let (bw, bh) = best.info().resolution();
                    let (mw, mh) = mode.info().resolution();
                    if mw >= FRAME_WIDTH && mh >= FRAME_HEIGHT && (mw * mh < bw * bh) {
                        Some(mode)
                    } else {
                        Some(best)
                    }
                }
                None => Some(mode),
            }
        })
        .expect("graphics output has no modes with pixel format available");
    writeln!(
        stdout(),
        "using graphics mode {}x{}",
        mode.info().resolution().0,
        mode.info().resolution().1,
    )
    .unwrap();
    graphics
        .set_mode(&mode)
        .expect("failed to set graphics output mode");

    let mut fs = st
        .boot_services()
        .get_image_file_system(image)
        .expect("couldn't open simple file system protocol");
    let mut root = fs.open_volume().expect("couldn't open root directory");
    let mut file = root
        .open(cstr16!("VIDEO.BIN"), FileMode::Read, FileAttribute::empty())
        .expect("couldn't open video")
        .into_regular_file()
        .expect("video is not a regular file");

    let mut info_buf = vec![0u8; 1024];
    let info = file
        .get_info::<FileInfo>(&mut info_buf)
        .expect("failed to get file info");
    let frame_count = (info.file_size() / (FRAME_WIDTH * FRAME_HEIGHT) as u64) as usize;

    let stride = mode.info().stride();
    let (width, height) = mode.info().resolution();
    let x_offset = (width - FRAME_WIDTH) / 2;
    let y_offset = (height - FRAME_HEIGHT) / 2;
    let skip_bytes = ((stride - width) + x_offset * 2) * 4;

    let mut frame_buffer = graphics.frame_buffer();
    let frame_buffer_ptr = frame_buffer.as_mut_ptr();
    let mut frame = vec![0u8; FRAME_WIDTH * FRAME_HEIGHT];

    for _ in 0..frame_count {
        let time = match timestamp.as_deref_mut() {
            Some(timestamp) => timestamp.get_timestamp(),
            None => 0,
        };

        let mut read = 0;
        loop {
            read += file.read(&mut frame[read..]).expect("failed to read frame");
            if read == FRAME_WIDTH * FRAME_HEIGHT {
                break;
            }
        }

        unsafe {
            let mut frame_buffer_ptr = frame_buffer_ptr.add((x_offset + y_offset * stride) * 4);
            for y in 0..FRAME_HEIGHT {
                for x in 0..FRAME_WIDTH {
                    let pixel = frame[y * FRAME_WIDTH + x];
                    for _ in 0..4 {
                        *frame_buffer_ptr = pixel;
                        frame_buffer_ptr = frame_buffer_ptr.add(1);
                    }
                }

                frame_buffer_ptr = frame_buffer_ptr.add(skip_bytes);
            }
        };

        if let Some(timestamp) = timestamp.as_deref_mut() {
            let end_time = timestamp.get_timestamp();
            let elapsed = if end_time >= time {
                time - end_time
            } else {
                (ts_end_value - time) + end_time
            } as usize;

            let micros_elapsed = (elapsed * MICROS_PER_SECOND) / (ts_frequency * MICROS_PER_SECOND);
            if MICROS_PER_FRAME > micros_elapsed {
                st.boot_services().stall(MICROS_PER_FRAME - micros_elapsed);
            }
        }
    }

    loop {
        unsafe { asm!("hlt") };
    }
}

unsafe extern "efiapi" fn exit_boot_services(_: Event, _: Option<NonNull<c_void>>) {
    SYSTEM_TABLE = None;
    ::uefi::alloc::exit_boot_services();
}

#[panic_handler]
fn handle_panic(info: &PanicInfo) -> ! {
    _ = write!(stderr(), "\nfuck\n{}", info);

    loop {
        unsafe { asm!("hlt") };
    }
}

#[alloc_error_handler]
fn handle_alloc_error(layout: Layout) -> ! {
    panic!("error trying to alloc {:?}", layout);
}

pub fn stdout() -> impl core::fmt::Write {
    unsafe { SYSTEM_TABLE.as_mut().unwrap().stdout() }
}

pub fn stderr() -> impl core::fmt::Write {
    unsafe { SYSTEM_TABLE.as_mut().unwrap().stderr() }
}
