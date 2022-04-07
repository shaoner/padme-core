use crate::collections::Queue;
use super::{Pixel, Sprite};

pub enum FetchState {
    Tile,
    TileDataLow,
    TileDataHigh,
    Sleep,
    Push,
}

pub struct Pipeline {
    /// To process 1 / 2 times
    pub ticks: u8,
    /// BG Pixel fifo
    pub fifo: Queue<Pixel, 16>,
    /// Objects list
    pub obj_list: [Sprite; 10],
    pub obj_count: u8,
    pub obj_fetched_idx: [u8; 3],
    pub obj_fetched_count: u8,
    /// Address of the tile map + y
    pub map_addr: u16,
    /// Current fetched X value in the tile map
    pub fetch_x: u8,
    /// Current Y inside the tile
    pub tile_y: u8,
    /// Current X to render
    pub render_x: u8,
    /// Fetch data (tile index, tile data low, tile data high)
    pub data: [u8; 3],
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
            ticks: 0,
            fifo: Queue::new([Pixel::default(); 16]),
            obj_list: [Sprite::default(); 10],
            obj_count: 0,
            obj_fetched_idx: [0u8; 3],
            obj_fetched_count: 0,
            map_addr: 0,
            fetch_x: 0,
            tile_y: 0,
            data: [0u8; 3],
            obj_data: [0u8; 6],
            state: FetchState::Tile,
            render_x: 0,
            win_y_triggered: false,
            win_ly: 0,
        }
    }

    pub fn init(&mut self, addr: u16, tile_y: u8) {
        self.map_addr = addr;
        self.tile_y = tile_y;
        self.state = FetchState::Tile;
        self.fifo.clear();
        self.render_x = 0;
        self.fetch_x = 0;
    }

    pub fn init_sprites(&mut self) {
        self.obj_count = 0;
        self.obj_fetched_count = 0;
    }

    pub fn push_sprite(&mut self, obj: Sprite) {
        self.obj_list[self.obj_count as usize] = obj;
        self.obj_count += 1;
    }

    pub fn sort_sprites(&mut self) {
        self.obj_list[..self.obj_count as usize].sort_unstable();
    }
}
