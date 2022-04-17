use crate::collections::Queue;
use super::{Pixel, Sprite};

/// 5 steps of the fetching
pub enum FetchState {
    Tile,
    TileDataLow,
    TileDataHigh,
    Sleep,
    Push,
}

pub struct Pipeline {
    /// Whether the ppu processing is disabled
    pub disabled: bool,
    /// To process 1 / 2 times
    pub ticks: u8,
    /// BG/Win Pixel fifo
    pub bgw_fifo: Queue<Pixel, 16>,
    /// Objects list
    pub obj_list: [Sprite; 10],
    pub obj_count: u8,
    pub obj_fetched_idx: [u8; 3],
    pub obj_fetched_count: u8,
    /// Tile map y offset
    pub addr_y_offset: u16,
    /// Current fetched X value in the tile map
    pub fetch_x: u8,
    /// Current Y inside the tile
    pub tile_y: u8,
    /// Current X rendered
    pub render_x: u8,
    /// Current X to render within scx
    pub lx: u8,
    /// Fetch data (tile index, tile data low, tile data high)
    pub bgw_data: [u8; 3],
    /// Sprite data (tile data low, tile data high)
    pub obj_data: [u8; 6],
    /// State of the processing
    pub state: FetchState,
    /// At some point in this frame the value of WY was equal to LY
    pub win_y_triggered: bool,
    /// Save the window line y coords
    pub win_ly: u8,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            disabled: false,
            ticks: 0,
            bgw_fifo: Queue::new([Pixel::default(); 16]),
            obj_list: [Sprite::default(); 10],
            obj_count: 0,
            obj_fetched_idx: [0u8; 3],
            obj_fetched_count: 0,
            addr_y_offset: 0,
            fetch_x: 0,
            tile_y: 0,
            bgw_data: [0u8; 3],
            obj_data: [0u8; 6],
            state: FetchState::Tile,
            render_x: 0,
            lx: 0,
            win_y_triggered: false,
            win_ly: 0,
        }
    }

    /// Init the pipeline fetcher to handle the pipeline during mode 3 (transfer)
    pub fn init_fetcher(&mut self, addr_y_offset: u16, tile_y: u8) {
        self.addr_y_offset = addr_y_offset;
        self.tile_y = tile_y;
        self.state = FetchState::Tile;
        self.bgw_fifo.clear();
        self.render_x = 0;
        self.fetch_x = 0;
        self.lx = 0;
    }

    /// Init sprites storage
    pub fn init_sprites(&mut self) {
        self.obj_count = 0;
        self.obj_fetched_count = 0;
    }

    /// Add sprites in the 10 potentials
    pub fn push_sprite(&mut self, obj: Sprite) {
        self.obj_list[self.obj_count as usize] = obj;
        self.obj_count += 1;
    }

    /// Sort sprites by X
    pub fn sort_sprites(&mut self) {
        self.obj_list[..self.obj_count as usize].sort_unstable();
    }
}
