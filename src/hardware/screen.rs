use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Renderer, Texture, TextureAccess};
use sdl2::VideoSubsystem;
use sdl2;
use sdl2_sys::render::{SDL_LockTexture, SDL_UnlockTexture};
use std::mem;
use std::ptr;
use util::color::Color;

const DISPLAY_WIDTH: u32 = 512;
const DISPLAY_HEIGHT: u32 = 478;

const TEXTURE_WIDTH: u32 = 512;
const TEXTURE_HEIGHT: u32 = 512;

pub struct Screen {
    renderer: Renderer<'static>,
    texture: Texture,
    mode: ScreenMode,
    overscan: bool,
    overscan_buffer: bool,
    brightness: u8,
    ptr: *mut u8,
    row_length: isize
}

pub enum ScreenMode {
    Standard,
    Interlace(InterlaceFrame)
}

pub enum InterlaceFrame {
    Even,
    Odd
}

impl Screen {
    pub fn new(video_subsystem: &VideoSubsystem) -> Screen {
        let window = video_subsystem
            .window("SNAIL", DISPLAY_WIDTH, DISPLAY_HEIGHT)
            .position_centered()
            .build()
            .unwrap();

        let renderer = window.renderer()
            .accelerated()
            .build()
            .unwrap();

        let mut texture = renderer
            .create_texture(
                PixelFormatEnum::ARGB8888,
                TextureAccess::Streaming,
                TEXTURE_WIDTH,
                TEXTURE_HEIGHT
            )
            .unwrap();

        texture.set_blend_mode(BlendMode::Blend);

        Screen {
            renderer: renderer,
            texture: texture,
            mode: ScreenMode::Standard,
            overscan: false,
            overscan_buffer: false,
            brightness: 0xFF,
            ptr: ptr::null_mut(),
            row_length: 0
        }
    }

    pub fn overscan(&self) -> bool {
        self.overscan
    }

    pub fn set_overscan(&mut self, overscan: bool) {
        // Wait until next frame to switch to overscan mode
        self.overscan_buffer = overscan;
    }

    pub fn set_brightness(&mut self, brightness: u8) {
        self.brightness = brightness;
    }

    pub fn begin_frame(&mut self) {
        self.renderer.clear();

        self.overscan = self.overscan_buffer;

        let mut row_length = 0;

        let ret = unsafe {
            SDL_LockTexture(
                self.texture.raw(),
                ptr::null(),
                mem::transmute(&mut self.ptr),
                &mut row_length
            )
        };

        if ret != 0 {
            panic!(sdl2::get_error());
        }

        self.row_length = row_length as isize;

        match self.mode {
            ScreenMode::Interlace(InterlaceFrame::Odd) => {
                // Skip the first row so we only render odd-numbered rows
                unsafe { self.ptr = self.ptr.offset(self.row_length); }
            },
            _ => ()
        };
    }

    pub fn end_frame(&mut self) {
        self.fill_non_interlace();

        unsafe { 
            SDL_UnlockTexture(self.texture.raw());
            self.ptr = ptr::null_mut();
        }

        let (src_rect, dst_rect) = if self.overscan {
            (Rect::new(0, 0, 512, 478), Rect::new(0, 0, 512, 478))
        } else {
            (Rect::new(0, 0, 512, 448), Rect::new(0, 15, 512, 448))
        };

        self.renderer.copy(&self.texture, Some(src_rect), Some(dst_rect)).unwrap();

        self.renderer.present();
    }

    pub fn blit(&mut self, color: Color) {
        unsafe {
            *self.ptr = color.blue() << 3;
            self.ptr = self.ptr.offset(1);
            *self.ptr = color.green() << 3;
            self.ptr = self.ptr.offset(1);
            *self.ptr = color.red() << 3;
            self.ptr = self.ptr.offset(1);
            *self.ptr = self.brightness;
            self.ptr = self.ptr.offset(1);
        }
    }

    pub fn next_line(&mut self) {
        self.fill_non_interlace();
        unsafe { self.ptr = self.ptr.offset(self.row_length); }
    }

    fn fill_non_interlace(&mut self) {
        match self.mode {
            ScreenMode::Standard => {
                // Duplicate the previous row
                unsafe {
                    ptr::copy(
                        self.ptr.offset(-self.row_length),
                        self.ptr,
                        self.row_length as usize
                    );
                }
            },
            ScreenMode::Interlace(..) => {
                // Skip the next row, so nothing to do here
            }
        }
    }
}
