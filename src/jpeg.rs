use std::io::prelude::*;
use std::io::{BufReader, Cursor};
use std::fs::File;
use std::path::PathBuf;
use byteorder::{BigEndian, ReadBytesExt};
use crate::{DecodeError, DecodeErrorKind};

pub const JPEG_SEGMENT_MARKER_START: u8 = 0xFF;
pub const JPEG_SOI_MARKER: [u8; 2] = [0xFF, 0xD8];
pub const JPEG_EOI_MARKER: [u8; 2] = [0xFF, 0xD9];

const JFIF_IDENTIFIER: [u8; 5] = [0x4A, 0x46, 0x49, 0x46, 0];
const JFIF_IDENTIFIER_STRING: &'static str = "JFIF";
const JFXX_IDENTIFIER: [u8; 5] = [0x4A, 0x46, 0x58, 0x58, 0];
const JFXX_IDENTIFIER_STRING: &'static str = "JFXX";

const READ_LEN: u64 = 4096;

#[derive(Debug, Default)]
pub struct APP0 {
    pub length: u16,
    pub identifier: [u8; 5],
    pub version_major: u8,
    pub version_minor: u8,
    pub density_units: u8,
    pub xdensity: u16,
    pub ydensity: u16,
    pub xthumbnail: u8,
    pub ythumbnail: u8,
    pub thumbnail_data: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct APP0Ext {
    pub length: u16,
    pub identifier: [u8; 5],
    pub thumbnail_format: u8,
    pub thumbnail_data: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct JPEGMeta {
    pub app0: APP0,
}

pub fn is_jpeg_file(file: &mut File) -> Result<bool, DecodeError> {
    let mut buf = [0u8; 2];
    file.seek(std::io::SeekFrom::Start(0))?;
    file.read_exact(&mut buf)?;
    if buf != JPEG_SOI_MARKER {
        return Ok(false)
    }

    file.seek(std::io::SeekFrom::End(-2))?;
    file.read_exact(&mut buf)?;
    if buf != JPEG_EOI_MARKER {
        return Ok(false)
    }

    file.rewind()?;
    Ok(true)
}

pub fn extract_metadata(filepath: PathBuf) -> Result<JPEGMeta, DecodeError> {
    let mut file = File::open(&filepath)?;
    if !is_jpeg_file(&mut file)? {
        return Err(DecodeError{
            kind: DecodeErrorKind::InvalidFormat,
            reason: format!("File {} is not a JPEG file", filepath.display()),
        })
    }

    let mut reader = BufReader::new(file);
    let mut buffer = Cursor::new(Vec::with_capacity(READ_LEN as usize));
    let mut metadata = JPEGMeta::default();
    let mut reached_sos_marker = false;

    while !reached_sos_marker {
        if reader.by_ref()
            .take(READ_LEN)
                .read_to_end(buffer.get_mut())? == 0 { break; }
        reached_sos_marker = decode_segments_from_buffer(&mut buffer, &mut metadata)?;
    }

    println!("{:#?}", metadata);

    Ok(metadata)
}

fn decode_segments_from_buffer(buffer: &mut Cursor<Vec<u8>>, 
    metadata: &mut JPEGMeta) -> Result<bool, DecodeError> {
    while buffer.position() < READ_LEN {
        if buffer.read_u8()? != JPEG_SEGMENT_MARKER_START { continue; }

        let marker = buffer.read_u8()?;
        if marker == 0 || marker == 0xFF { continue; }

        log::trace!(" ==> Segment Marker [0x{:02x}]", marker);
        match marker {
            0xDA => return Ok(true), // SOS - Start of Scan
            0xE0 => { // APP0 (JFIF)
                metadata.app0.length = buffer.read_u16::<BigEndian>()?;
                buffer.read_exact(&mut metadata.app0.identifier)?;
                let ident_string = std::str::from_utf8(&metadata.app0.identifier)?;
                if metadata.app0.identifier != JFIF_IDENTIFIER {
                    return Err(DecodeError{
                        kind: DecodeErrorKind::InvalidFormat,
                        reason: format!("Expected identifier {}, got {}",
                            JFIF_IDENTIFIER_STRING, ident_string)
                    });
                }
                metadata.app0.version_major = buffer.read_u8()?;
                metadata.app0.version_minor = buffer.read_u8()?;
                metadata.app0.density_units = buffer.read_u8()?;
                let density_string = match metadata.app0.density_units {
                    0 => "aspect ratio",
                    1 => "pixels per inch",
                    3 => "pixels per centimeter",
                    invalid => { 
                        return Err(DecodeError{
                            kind: DecodeErrorKind::BadData,
                            reason: format!("Invalid density units value; Expected one of 
                                (0x00, 0x01, 0x03), got {:02X} instead", invalid),
                        }); 
                    } 
                };

                metadata.app0.xdensity = buffer.read_u16::<BigEndian>()?;
                metadata.app0.ydensity = buffer.read_u16::<BigEndian>()?;
                metadata.app0.xthumbnail = buffer.read_u8()?;
                metadata.app0.ythumbnail = buffer.read_u8()?;

                let m = &metadata.app0;
                log::trace!("Segment Data ({} bytes): {} version {}.{:02};
                density {}x{} ({}); thumbnail {}x{}", m.length, ident_string, 
                m.version_major, m.version_minor, m.xdensity, m.ydensity, density_string,
                m.xthumbnail, m.ythumbnail);
            },
            0xE1 => { // APP0 (EXIF)
                unimplemented!("EXIF decoding not implemented!");
            },
            _ => continue,
        }
    }

    Ok(false)
}
