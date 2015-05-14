#![feature(alloc, collections, core, libc)]

extern crate libc;
extern crate image;
extern crate img_hash;

use image::{
    FilterType,
    GrayImage,
    ImageBuffer,
    Luma,
    Rgb,
    Rgba,
    Pixel,
};

use img_hash::{HashType, HashImage, ImageHash};

use libc::{c_int, c_uint, c_uchar, size_t};

use std::{
    boxed,
    ptr,
    slice,
};

#[repr(C)]
#[derive(PartialEq, Eq, Copy, Clone)]
pub enum ExternHashType {
    Mean = 1,
    Gradient = 2,
    DoubleGradient = 3,
    DCT = 4,
}

impl Into<HashType> for ExternHashType {
    fn into(self) -> HashType {
        use ExternHashType::*;

        match self {
            Mean => HashType::Mean,
            Gradient => HashType::Gradient,
            DoubleGradient => HashType::DoubleGradient,
            DCT => HashType::DCT,
        }
    }
}

pub struct ExternHashImage(GrayImage);

impl HashImage for ExternHashImage {
    fn gray_resize_square(&self, size: u32) -> GrayImage {
        image::imageops::resize(&self.0, size, size, FilterType::Nearest)
    }
}

#[no_mangle]
pub extern fn create_hash_image(
    img_data: *const c_uchar, width: c_uint, height: c_uint, channels: c_int
) -> *mut ExternHashImage {
    let data_len = (width * height * (channels.abs() as u32)) as usize;
    let img_data = unsafe { slice::from_raw_parts(img_data as *const u8, data_len) };

    create_extern_hash_image(img_data, width, height, channels)
        .map(Box::new)
        .map_or_else(
            || ptr::null_mut(),
            |hash_image| unsafe { boxed::into_raw(hash_image) }
        )
}

#[no_mangle]
pub extern fn create_hash(
    hash_image: *mut ExternHashImage, 
    hash_type: ExternHashType, 
    hash_size: c_uint, 
    hash_data_out: *mut c_uchar
) -> c_int {   
    if hash_image.is_null() || hash_data_out.is_null() {
        return false as c_int;
    }

    let hash_image = unsafe { Box::from_raw(hash_image) };

    let hash = ImageHash::hash(&*hash_image, hash_size, hash_type.into());

    let hash_data = hash.bitv.to_bytes();
    let hash_data_out = unsafe { 
        slice::from_raw_parts_mut(hash_data_out, (hash_size * hash_size) as usize) 
    };

    slice::bytes::copy_memory(&hash_data, hash_data_out);

    true as c_int
}

#[no_mangle]
pub extern fn get_hash_data_alloc_size(hash_size: c_uint) -> size_t {
    let hash_sq = hash_size * hash_size;

    let mut alloc_size = hash_sq as usize / 8;

    if hash_sq % 8 != 0 {
        alloc_size += 1;
    }

    alloc_size as size_t
}


fn create_extern_hash_image(
    img_data: &[u8], width: u32, height: u32, channels: i32
) -> Option<ExternHashImage> {
    use image::imageops::colorops::grayscale;

    let img_data = img_data.to_owned();   

    let gray_image = match channels {
        1 => ImageBuffer::<Luma<u8>, _>::from_raw(width, height, img_data).unwrap(),
        3 => {
            let rgb_image = ImageBuffer::<Rgb<u8>, _>::from_raw(width, height, img_data).unwrap();
            grayscale(&rgb_image)
        },
        -4 => {
            let mut argb_image = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, img_data).unwrap();
            // Convert ARGB to RGBA
            for px in argb_image.pixels_mut() {
                let (a, r, g, b) = px.channels4();
                *px = Rgba::from_channels(r, g, b, a);
            }

            grayscale(&argb_image)
        },

        _ => return None,
    };

    Some(ExternHashImage(gray_image))
}
