use log::trace;

use crate::interrupt::{InterruptHandler, InterruptFlag};
use crate::region::*;

use super::{FetchState, Pipeline, Pixel, Sprite};

//
// Frame configuration
//
pub const FRAME_WIDTH: usize            = 160;
pub const FRAME_HEIGHT: usize           = 144;

//
// Default register values
//
const DEFAULT_REG_DMG_LCDC: u8          = 0x91;
const DEFAULT_REG_DMG_STAT: u8          = 0x81;
const DEFAULT_REG_DMG_SCY: u8           = 0x00;
const DEFAULT_REG_DMG_SCX: u8           = 0x00;
const DEFAULT_REG_DMG_LY: u8            = 0x91;
const DEFAULT_REG_DMG_LYC: u8           = 0x00;
const DEFAULT_REG_DMG_DMA: u8           = 0xFF;
const DEFAULT_REG_DMG_BGP: u8           = 0xFC;
const DEFAULT_REG_DMG_WY: u8            = 0x00;
const DEFAULT_REG_DMG_WX: u8            = 0x00;
const DEFAULT_REG_DMG_OBP0: u8          = 0xFF;
const DEFAULT_REG_DMG_OBP1: u8          = 0xFF;

//
// Tile regions
//
const TILE_DATA_0_START_ADDR: u16       = VRAM_REGION_START;
const TILE_DATA_1_START_ADDR: u16       = 0x8800;

const TILE_MAP_0_START_ADDR: u16        = 0x9800;
const TILE_MAP_1_START_ADDR: u16        = 0x9C00;

//
// LCD status flags
//
const FLAG_STAT_IT_LYC: u8              = 0b01000000;
const FLAG_STAT_IT_OAM: u8              = 0b00100000;
const FLAG_STAT_IT_VBLANK: u8           = 0b00010000;
const FLAG_STAT_IT_HBLANK: u8           = 0b00001000;
const FLAG_STAT_LYC: u8                 = 0b00000100;
const FLAG_STAT_MODE: u8                = 0b00000011;

//
// LCD status modes
//
const LCD_STATUS_MODE_HBLANK: u8        = 0;
const LCD_STATUS_MODE_VBLANK: u8        = 1;
const LCD_STATUS_MODE_OAM: u8           = 2;
const LCD_STATUS_MODE_XFER: u8          = 3;

//
// LCD control flags
//
const FLAG_LCDC_LCD_ENABLE: u8          = 0b10000000;
const FLAG_LCDC_WIN_TMAP_AREA: u8       = 0b01000000;
const FLAG_LCDC_WIN_ENABLE: u8          = 0b00100000;
const FLAG_LCDC_BGWIN_TDATA_AREA: u8    = 0b00010000;
const FLAG_LCDC_BG_TMAP_AREA: u8        = 0b00001000;
const FLAG_LCDC_OBJ_SIZE: u8            = 0b00000100;
const FLAG_LCDC_OBJ_ENABLE: u8          = 0b00000010;
const FLAG_LCDC_BG_WIN_ENABLE: u8       = 0b00000001;

//
// Modes
//
const OAM_LIMIT_PERIOD: u32             = 80;
const XFER_LIMIT_PERIOD: u32            = OAM_LIMIT_PERIOD + 172;
const HBLANK_LIMIT_PERIOD: u32          = 456;
const FRAME_LIMIT_PERIOD: u32           = HBLANK_LIMIT_PERIOD * (FRAME_HEIGHT as u32);
const VBLANK_LIMIT_PERIOD: u32          = FRAME_LIMIT_PERIOD + HBLANK_LIMIT_PERIOD * 10;

//
// Default pixels
//
// This white is slightly less white than pixel used during disabled screen
const PIXEL_COLOR_WHITE: Pixel          = Pixel { r: 0xFE, g: 0xFE, b: 0xFE, a: 0xFE };
const PIXEL_COLOR_LIGHTGRAY: Pixel      = Pixel { r: 0xC0, g: 0xC0, b: 0xC0, a: 0xFF };
const PIXEL_COLOR_DARKGRAY: Pixel       = Pixel { r: 0x60, g: 0x60, b: 0x60, a: 0xFF };
const PIXEL_COLOR_BLACK: Pixel          = Pixel { r: 0x00, g: 0x00, b: 0x00, a: 0xFF };

// Debug functions
macro_rules! trace_mode {
    ($mode: expr) => {
        trace!("pixel mode: {}", $mode)
    }
}

/// This represents a Screen surface
/// # Example
///
/// ```
/// use padme_core::{FRAME_HEIGHT, FRAME_WIDTH, Pixel, Screen};
///
/// struct Canvas {
///     pixels: [u32; FRAME_HEIGHT * FRAME_WIDTH],
/// }
///
/// impl Screen for Canvas {
///     fn set_pixel(&mut self, px: &Pixel, x: u8, y: u8) {
///         self.pixels[x as usize * FRAME_WIDTH + y as usize] = px.argb();
///     }
///
///     fn update(&mut self) {
///     }
/// }
/// ```
pub trait Screen {
    /// Set a single pixel on a screen
    /// This could be used to either store the pixel in a buffer
    /// or draw directly (in this case, the draw method can be empty)
    fn set_pixel(&mut self, px: &Pixel, x: u8, y: u8);
    /// Notify the screen of a new frame
    /// This is dependent on the FPS
    fn update(&mut self);
}

pub struct Ppu {
    /// Video ram
    vram: [u8; VRAM_REGION_SIZE],
    /// Object Attribute Table
    oam: [u8; OAM_REGION_SIZE],
    /// LCD control register
    reg_lcdc: u8,
    /// LCD status register
    reg_stat: u8,
    /// Scroll Y register
    reg_scy: u8,
    /// Scroll X register
    reg_scx: u8,
    /// LCD Y register
    reg_ly: u8,
    /// LCD Y Compare register
    reg_lyc: u8,
    /// Window Y register
    reg_wy: u8,
    /// Window X register
    reg_wx: u8,
    /// Dma transfer register
    reg_dma: u8,
    /// Background Palette
    reg_bgp: u8,
    /// Obj palettes 0 & 1
    reg_obp0: u8,
    reg_obp1: u8,
    /// Keep tracks of horizontal dots (max = 456)
    hdots: u32,
    /// Pixel pipeline
    pipeline: Pipeline,
    /// Dma
    dma_active: bool,
    dma_idx: u8,
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            vram: [0x00u8; VRAM_REGION_SIZE],
            oam: [0x00u8; OAM_REGION_SIZE],
            reg_lcdc: DEFAULT_REG_DMG_LCDC,
            reg_stat: DEFAULT_REG_DMG_STAT,
            reg_scy: DEFAULT_REG_DMG_SCY,
            reg_scx: DEFAULT_REG_DMG_SCX,
            reg_ly: DEFAULT_REG_DMG_LY,
            reg_lyc: DEFAULT_REG_DMG_LYC,
            reg_wy: DEFAULT_REG_DMG_WY,
            reg_wx: DEFAULT_REG_DMG_WX,
            reg_dma: DEFAULT_REG_DMG_DMA,
            reg_bgp: DEFAULT_REG_DMG_BGP,
            reg_obp0: DEFAULT_REG_DMG_OBP0,
            reg_obp1: DEFAULT_REG_DMG_OBP1,
            hdots: 0,
            pipeline: Pipeline::new(),
            dma_active: false,
            dma_idx: 0,
        }
    }

    /// Reset all registers and state
    pub fn reset(&mut self) {
        self.reg_lcdc = DEFAULT_REG_DMG_LCDC;
        self.reg_stat = DEFAULT_REG_DMG_STAT;
        self.reg_scy = DEFAULT_REG_DMG_SCY;
        self.reg_scx = DEFAULT_REG_DMG_SCX;
        self.reg_ly = DEFAULT_REG_DMG_LY;
        self.reg_lyc = DEFAULT_REG_DMG_LYC;
        self.reg_wy = DEFAULT_REG_DMG_WY;
        self.reg_wx = DEFAULT_REG_DMG_WX;
        self.reg_dma = DEFAULT_REG_DMG_DMA;
        self.reg_bgp = DEFAULT_REG_DMG_BGP;
        self.reg_obp0 = DEFAULT_REG_DMG_OBP0;
        self.reg_obp1 = DEFAULT_REG_DMG_OBP1;
        self.hdots = 0;
        self.pipeline = Pipeline::new();
        self.dma_active = false;
        self.dma_idx = 0;
        self.vram.iter_mut().for_each(| byte | *byte = 0);
        self.oam.iter_mut().for_each(| byte | *byte = 0);
    }

    /// Starts a DMA transfer
    pub fn dma_start(&mut self, source: u8) {
        self.reg_dma = source;
        self.dma_active = true;
        self.dma_idx = 0;
        trace!("dma start with source = 0x{:04X}, destination = 0x{:04X}",
               self.dma_source(), OAM_REGION_START);
    }

    /// Checks whether DMA transfer is still pending
    #[inline]
    pub fn is_dma_active(&self) -> bool {
        self.dma_active
    }

    /// Retrieve current DMA source address
    #[inline]
    pub fn dma_source(&self) -> u16 {
        self.reg_dma as u16 * 0x100 + self.dma_idx as u16
    }

    /// Write a byte retrieved from source directly to oam memory
    #[inline]
    pub fn dma_write(&mut self, byte: u8) {
        self.oam[self.dma_idx as usize] = byte;
        self.dma_idx += 1;
        if self.dma_idx as usize >= OAM_REGION_SIZE {
            self.dma_active = false;
        }
    }

    /// Sets the new line y coordinate which eventually triggers some exceptions
    fn set_ly(&mut self, value: u8, it: &mut InterruptHandler) {
        self.reg_ly = value;
        if self.reg_ly == self.reg_lyc {
            self.reg_stat |= FLAG_STAT_LYC;
            if is_set!(self.reg_stat, FLAG_STAT_IT_LYC) {
                it.request(InterruptFlag::Lcdc);
            }
        } else {
            self.reg_stat &= !FLAG_STAT_LYC;
        }
    }

    #[inline]
    fn inc_ly(&mut self, it: &mut InterruptHandler) {
        self.set_ly(self.reg_ly + 1, it);
    }

    /// Retrieve pixel color from color id
    fn pixel_from_id(pal: u8, color_id: u8) -> Pixel {
        match (pal >> (color_id * 2)) & 0x3 {
            0 => PIXEL_COLOR_WHITE,
            1 => PIXEL_COLOR_LIGHTGRAY,
            2 => PIXEL_COLOR_DARKGRAY,
            3 => PIXEL_COLOR_BLACK,
            _ => unreachable!(),
        }
    }

    /// Sets pixel mode
    #[inline]
    fn set_mode(&mut self, mode: u8) {
        self.reg_stat = (self.reg_stat & !FLAG_STAT_MODE) | mode;
    }

    /// Retrieve whether background/window is enabled
    #[inline]
    fn is_bgwin_enabled(&self) -> bool {
        is_set!(self.reg_lcdc, FLAG_LCDC_BG_WIN_ENABLE)
    }

    /// Retrieve whether sprites are enabled
    #[inline]
    fn is_obj_enabled(&self) -> bool {
        is_set!(self.reg_lcdc, FLAG_LCDC_OBJ_ENABLE)
    }

    /// Retrieve sprite size for the current row
    #[inline]
    fn obj_size(&self) -> u8 {
        if is_set!(self.reg_lcdc, FLAG_LCDC_OBJ_SIZE) { 16 } else { 8 }
    }

    /// Retrieve background tile map
    #[inline]
    fn bg_map_area(&self) -> u16 {
        if is_set!(self.reg_lcdc, FLAG_LCDC_BG_TMAP_AREA) {
            TILE_MAP_1_START_ADDR
        } else {
            TILE_MAP_0_START_ADDR
        }
    }

    /// Retrieve background/windows tile data
    #[inline]
    fn bgwin_data_area(&self) -> u16 {
        if is_set!(self.reg_lcdc, FLAG_LCDC_BGWIN_TDATA_AREA) {
            TILE_DATA_0_START_ADDR
        } else {
            TILE_DATA_1_START_ADDR
        }
    }

    /// Retrieve whether window is enabled
    #[inline]
    fn is_win_enabled(&self) -> bool {
        is_set!(self.reg_lcdc, FLAG_LCDC_WIN_ENABLE)
    }

    /// Retrieve window tile map area
    #[inline]
    fn win_map_area(&self) -> u16 {
        if is_set!(self.reg_lcdc, FLAG_LCDC_WIN_TMAP_AREA) {
            TILE_MAP_1_START_ADDR
        } else {
            TILE_MAP_0_START_ADDR
        }
    }

    /// Retrieve whether lcd is enabled at all
    #[inline]
    fn is_lcd_enabled(&self) -> bool {
        is_set!(self.reg_lcdc, FLAG_LCDC_LCD_ENABLE)
    }

    /// Used to advance the PPU mode after some CPU cycles
    pub fn step<S: Screen>(&mut self, screen: &mut S, it: &mut InterruptHandler) {
        // Dots counter is reset during hblank
        self.hdots += 1;

        match self.reg_stat & FLAG_STAT_MODE {
            LCD_STATUS_MODE_OAM => self.handle_mode_oam(),
            LCD_STATUS_MODE_XFER => self.handle_mode_xfer(screen, it),
            LCD_STATUS_MODE_HBLANK => self.handle_mode_hblank(it),
            LCD_STATUS_MODE_VBLANK => self.handle_mode_vblank(screen, it),
            _ => unreachable!(),
        }
    }

    /// Mode 2: OAM scanning
    fn handle_mode_oam(&mut self) {
        trace_mode!("oam");
        if self.hdots == 1 {
            self.scan_sprites();
            // check if this line is a window_y trigger
            if self.is_win_enabled() &&
                self.reg_wx < (FRAME_WIDTH as u8) &&
                self.reg_wy < (FRAME_HEIGHT as u8) &&
                self.reg_ly >= self.reg_wy &&
                self.reg_ly < self.reg_wy.wrapping_add(FRAME_HEIGHT as u8)
            {
                if !self.pipeline.win_y_triggered {
                    self.pipeline.win_y_triggered = true;
                } else {
                    self.pipeline.win_ly += 1;
                }
            }
        } else if self.hdots >= OAM_LIMIT_PERIOD {
            self.set_mode(LCD_STATUS_MODE_XFER);

            let y = self.reg_ly.wrapping_add(self.reg_scy);
            let addr_y_offset = (y / 8) as u16 * 32;
            let tile_y = y % 8;

            self.pipeline.init_fetcher(addr_y_offset, tile_y);
        }
    }

    /// Mode 3: Drawing pixels
    fn handle_mode_xfer<S: Screen>(&mut self, screen: &mut S, it: &mut InterruptHandler) {
        trace!("xfer");
        if self.pipeline.render_x < FRAME_WIDTH as u8 {
            self.render(screen);
        } else if self.hdots >= XFER_LIMIT_PERIOD {
            self.pipeline.bgw_fifo.clear();
            self.set_mode(LCD_STATUS_MODE_HBLANK);
            if is_set!(self.reg_stat, FLAG_STAT_IT_HBLANK) {
                it.request(InterruptFlag::Lcdc);
            }
        }
    }

    /// Mode 0: Handle HBlank
    fn handle_mode_hblank(&mut self, it: &mut InterruptHandler) {
        trace!("hblank");
        if self.hdots >= HBLANK_LIMIT_PERIOD {
            self.inc_ly(it);
            // When the frame height is reached, switch to vblank mode
            if self.reg_ly >= FRAME_HEIGHT as u8 {
                self.set_mode(LCD_STATUS_MODE_VBLANK);
                it.request(InterruptFlag::Vblank);
                if is_set!(self.reg_stat, FLAG_STAT_IT_VBLANK) {
                    it.request(InterruptFlag::Lcdc);
                }
            } else {
                self.set_mode(LCD_STATUS_MODE_OAM);
                if is_set!(self.reg_stat, FLAG_STAT_IT_OAM) {
                    it.request(InterruptFlag::Lcdc);
                }
            }
            // Reset horizontal dots
            self.hdots = 0;
        }
    }

    /// Mode 1: Handle VBlank
    fn handle_mode_vblank<S: Screen>(&mut self, screen: &mut S, it: &mut InterruptHandler) {
        trace!("vblank");
        if !self.pipeline.disabled && !self.is_lcd_enabled() {
            // disable ppu + next frame is white
            self.disable(screen);
        } else if self.pipeline.disabled && self.is_lcd_enabled() {
            // if ppu is enabled, the drawing starts immediately but the frame remains white
            // instead we can just not re-enable the pipeline in the VBlank mode
            self.pipeline.disabled = false;
        }
        if self.hdots >= HBLANK_LIMIT_PERIOD {
            // End of line is reached
            self.inc_ly(it);
            if (self.reg_ly as u32 * HBLANK_LIMIT_PERIOD) >= VBLANK_LIMIT_PERIOD {
                // reset ly
                self.set_ly(0, it);
                // reset window conditions
                self.pipeline.win_ly = 0;
                self.pipeline.win_y_triggered = false;
                self.set_mode(LCD_STATUS_MODE_OAM);
                if is_set!(self.reg_stat, FLAG_STAT_IT_OAM) {
                    it.request(InterruptFlag::Lcdc);
                }
            }
            self.hdots = 0;
        }
    }

    /// Disable PPU & sets default LCD screen color
    fn disable<S: Screen>(&mut self, screen: &mut S) {
        self.pipeline.disabled = true;
        let px = Pixel { r: 0xFF, g: 0xFF, b: 0xFF, a: 0xFF };
        for y in 0..FRAME_HEIGHT {
            for x in 0..FRAME_WIDTH {
                screen.set_pixel(&px, x as u8, y as u8);
            }
        }
    }

    /// Retrieve background tile index for the current X
    fn select_bg_tiles(&mut self) {
        let x = self.pipeline.fetch_x.wrapping_add(self.reg_scx) as u16 / 8;
        let tile_index = self.read(self.bg_map_area() + self.pipeline.addr_y_offset + x);
        let offset = if is_not_set!(self.reg_lcdc, FLAG_LCDC_BGWIN_TDATA_AREA) {
            128u8
        } else {
            0u8
        };
        self.pipeline.bgw_data[0] = tile_index.wrapping_add(offset);
    }

    /// Retrieve window tile index for the current X
    fn select_win_tiles(&mut self) {
        if self.reg_wx < (FRAME_WIDTH as u8 + 7)
            && self.reg_wy < (FRAME_HEIGHT as u8)
            && self.pipeline.win_y_triggered
            && (self.pipeline.fetch_x + 7) >= self.reg_wx {
                let tile_y = self.pipeline.win_ly as u16 / 8;
                let addr = (self.pipeline.fetch_x as u16 + 7 - self.reg_wx as u16) / 8 + tile_y * 32;
                let tile_index = self.read(self.win_map_area() + addr);
                let offset = if is_not_set!(self.reg_lcdc, FLAG_LCDC_BGWIN_TDATA_AREA) {
                    128u8
                } else {
                    0u8
                };
                self.pipeline.bgw_data[0] = tile_index.wrapping_add(offset);
            }
    }

    /// Retrieve the current background/window tile data
    fn load_bgwin_data(&mut self, offset: u16) {
        let tile_index = self.pipeline.bgw_data[0];
        let addr = self.bgwin_data_area() + tile_index as u16 * 16 + self.pipeline.tile_y as u16 * 2 + offset;
        self.pipeline.bgw_data[1 + offset as usize] = self.read(addr);
    }

    /// Scan for max 10 sprites in the current scan line
    fn scan_sprites(&mut self) {
        let rel_y = self.reg_ly + 16;
        let obj_size = self.obj_size();

        self.pipeline.init_sprites();

        // Check for each sprite matching the current line in the oam (limit to 10)
        for i in (0..OAM_REGION_SIZE).step_by(4) {
            let y = self.oam[i];
            let x = self.oam[i + 1];
            let tile_index = self.oam[i + 2];
            let attrs = self.oam[i + 3];

            if rel_y >= y && rel_y < y + obj_size {
                self.pipeline.push_sprite(Sprite::new(x, y, tile_index, attrs));
                if self.pipeline.obj_count >= 10 {
                    break;
                }
            }
        }
        // Sort sprites by their X coord
        self.pipeline.sort_sprites();
    }

    /// Retrieve sprite tile index(es) for the current X
    fn select_sprites(&mut self) {
        let offset = (self.reg_scx % 8) as i16;
        self.pipeline.obj_fetched_count = 0;

        for i in 0..(self.pipeline.obj_count as usize) {
            let obj = &self.pipeline.obj_list[i];
            let rel_x = (obj.x as i16).wrapping_sub(8).wrapping_add(offset);
            let rel_x1 = rel_x.wrapping_add(8);
            let fetch_x1 = (self.pipeline.fetch_x as i16).wrapping_add(8);

            if (rel_x >= self.pipeline.fetch_x as i16 && rel_x < fetch_x1)
                || (rel_x1 >= self.pipeline.fetch_x as i16 && rel_x1 < fetch_x1) {
                    self.pipeline.obj_fetched_idx[self.pipeline.obj_fetched_count as usize] = i as u8;
                    self.pipeline.obj_fetched_count += 1;
                    // There cannot be more than 3 sprites to appear within 8 pixels
                    // left + middle + right
                    if self.pipeline.obj_fetched_count >= 3 {
                        break;
                    }
                }
        }
    }

    /// Retrieve sprite tile data
    fn load_sprite_data(&mut self, offset: u16) {
        let obj_size = self.obj_size();

        for i in 0..(self.pipeline.obj_fetched_count as usize) {
            let obj = &self.pipeline.obj_list[self.pipeline.obj_fetched_idx[i] as usize];
            let tile_y = if obj.is_y_flipped() {
                ((obj_size * 2) - 2) - ((self.reg_ly + 16) - obj.y) * 2
            } else {
                ((self.reg_ly + 16) - obj.y) * 2
            } as u16;
            let tile_index = if obj_size == 16 {
                obj.tile_index & 0xFE
            } else {
                obj.tile_index
            };
            let addr = TILE_DATA_0_START_ADDR + (tile_index as u16 * 16) + tile_y + offset;
            self.pipeline.obj_data[i * 2 + offset as usize] = self.read(addr);
        }
    }

    /// Push pixel in the bgw_fifo
    fn push_pixels(&mut self) {
        // get color id from low and high bytes at the bit position
        macro_rules! color_id {
            ($low: expr, $high: expr, $bit: expr) => {
                (($low >> $bit) & 0x01) | ((($high >> $bit) & 0x01) << 1)
            }
        }

        let bg_low = self.pipeline.bgw_data[1];
        let bg_high = self.pipeline.bgw_data[2];

        for i in (0..=7u8).rev() {
            let mut bg_color_id = 0;

            // Retrieve bg color id if enabled
            if self.is_bgwin_enabled() {
                bg_color_id = color_id!(bg_low, bg_high, i);
            }

            let mut pixel = Ppu::pixel_from_id(self.reg_bgp, bg_color_id);

            // Check sprites if enabled
            if self.is_obj_enabled() {
                for j in 0..(self.pipeline.obj_fetched_count as usize) {
                    let obj = self.pipeline.obj_list[self.pipeline.obj_fetched_idx[j] as usize];
                    let rel_x = (obj.x as i16).wrapping_sub(8).wrapping_add((self.reg_scx % 8) as i16);

                    // Too far
                    if rel_x.wrapping_add(8) < self.pipeline.fetch_x as i16 {
                        continue;
                    }
                    let offset = self.pipeline.fetch_x as i16 - rel_x as i16;
                    if !(0..=7).contains(&offset) {
                        continue;
                    }
                    let bit = if obj.is_x_flipped() { offset } else { 7 - offset };
                    let obj_low = self.pipeline.obj_data[j * 2];
                    let obj_high = self.pipeline.obj_data[j * 2 + 1];
                    let obj_color_id = color_id!(obj_low, obj_high, bit);

                    if obj_color_id == 0 {
                        continue;
                    }
                    if !obj.is_bgwin_prio() || bg_color_id == 0 {
                        let pal = if obj.palette_number() == 0 { self.reg_obp0 } else { self.reg_obp1 };
                        pixel = Ppu::pixel_from_id(pal, obj_color_id);
                        break;
                    }
                }
            }
            self.pipeline.bgw_fifo.push(pixel);
            self.pipeline.fetch_x += 1;
        }

    }

    /// Handle pixel row and display pixels if any
    fn render<S: Screen>(&mut self, screen: &mut S) {
        if !self.pipeline.disabled {
            self.fetch_pixel_row();

            if self.pipeline.bgw_fifo.size() > 0 {
                let px = self.pipeline.bgw_fifo.pop();
                if self.pipeline.lx >= (self.reg_scx % 8) {
                    screen.set_pixel(&px, self.pipeline.render_x, self.reg_ly);
                    self.pipeline.render_x += 1;
                }
                self.pipeline.lx += 1;
            }
        } else {
            self.pipeline.render_x += 1;
        }
    }

    /// Manage the pixel fetcher state machine
    fn fetch_pixel_row(&mut self) {
        self.pipeline.ticks += 1;

        if self.pipeline.ticks == 2 {
            // Fetch runs at half cpu speed
            self.pipeline.ticks = 0;
            return;
        }

        // Pixel fetcher state machine
        match self.pipeline.state {
            FetchState::Tile => {
                // Retrieve tile index
                if self.is_bgwin_enabled() {
                    self.select_bg_tiles();

                    if self.is_win_enabled() {
                        self.select_win_tiles();
                    }
                }
                if self.is_obj_enabled() {
                    self.select_sprites();
                }
                self.pipeline.state = FetchState::TileDataLow;
            },
            FetchState::TileDataLow => {
                self.load_bgwin_data(0);
                self.load_sprite_data(0);
                self.pipeline.state = FetchState::TileDataHigh;
            },
            FetchState::TileDataHigh => {
                self.load_bgwin_data(1);
                self.load_sprite_data(1);
                self.pipeline.state = FetchState::Sleep;
            },
            FetchState::Sleep => {
                self.pipeline.state = FetchState::Push;
            },
            FetchState::Push => {
                if self.pipeline.bgw_fifo.is_empty() {
                    self.push_pixels();
                    self.pipeline.state = FetchState::Tile;
                }
            },
        }
    }
}

impl MemoryRegion for Ppu {
    fn read(&self, address: u16) -> u8 {
        match address {
            VRAM_REGION_START..=VRAM_REGION_END => {
                self.vram[(address - VRAM_REGION_START) as usize]
            },
            OAM_REGION_START..=OAM_REGION_END => {
                self.oam[(address - OAM_REGION_START) as usize]
            },
            REG_LCDC_ADDR => self.reg_lcdc,
            REG_STAT_ADDR => self.reg_stat,
            REG_SCY_ADDR => self.reg_scy,
            REG_SCX_ADDR => self.reg_scx,
            REG_LY_ADDR => self.reg_ly,
            REG_LYC_ADDR => self.reg_lyc,
            REG_WY_ADDR => self.reg_wy,
            REG_WX_ADDR => self.reg_wx,
            REG_DMA_ADDR => self.reg_dma,
            REG_BGP_ADDR => self.reg_bgp,
            REG_OBP0_ADDR => self.reg_obp0,
            REG_OBP1_ADDR => self.reg_obp1,
            _ => unreachable!(),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            VRAM_REGION_START..=VRAM_REGION_END => {
                self.vram[(address - VRAM_REGION_START) as usize] = value
            },
            OAM_REGION_START..=OAM_REGION_END => {
                self.oam[(address - OAM_REGION_START) as usize] = value;
            },
            REG_LCDC_ADDR => self.reg_lcdc = value,
            // bit 2, 1 and 0 are readonly
            REG_STAT_ADDR => self.reg_stat = (value & 0xF8) | (self.reg_stat & 0x07),
            REG_SCY_ADDR => self.reg_scy = value,
            REG_SCX_ADDR => self.reg_scx = value,
            REG_LYC_ADDR => self.reg_lyc = value,
            REG_WY_ADDR => self.reg_wy = value,
            REG_WX_ADDR => self.reg_wx = value,
            REG_DMA_ADDR => self.dma_start(value),
            REG_BGP_ADDR => self.reg_bgp = value,
            REG_OBP0_ADDR => self.reg_obp0 = value,
            REG_OBP1_ADDR => self.reg_obp1 = value,
            _ => unreachable!(),
        }
    }
}
