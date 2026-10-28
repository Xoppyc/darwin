use wasm_bindgen::prelude::*;

pub const NODE_STRIDE: usize = 6;

// Repr(C) ensures deterministic, unpadded field alignment so Float32Array in JS
// maps 1:1 to memory offsets without serialization overhead.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeBufferItem {
    pub id: f32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub flags: f32,
}

#[wasm_bindgen]
pub struct DarwinEngine {
    nodes: Vec<NodeBufferItem>,
    camera_x: f32,
    camera_y: f32,
    zoom: f32,
    dirty: bool,
}

#[wasm_bindgen]
impl DarwinEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(64),
            camera_x: 0.0,
            camera_y: 0.0,
            zoom: 1.0,
            dirty: true,
        }
    }

    pub fn node_stride() -> usize {
        NODE_STRIDE
    }

    pub fn nodes_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn nodes_len(&self) -> usize {
        self.nodes.len() * NODE_STRIDE
    }

    // Expose raw linear memory pointer directly to JS Float32Array
    pub fn nodes_ptr(&self) -> *const f32 {
        self.nodes.as_ptr() as *const f32
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn camera_x(&self) -> f32 {
        self.camera_x
    }

    pub fn camera_y(&self) -> f32 {
        self.camera_y
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    pub fn add_node(&mut self, id: f32, x: f32, y: f32, width: f32, height: f32) {
        self.nodes.push(NodeBufferItem {
            id,
            x,
            y,
            width,
            height,
            flags: 0.0,
        });
        self.dirty = true;
    }

    pub fn move_node(&mut self, id: f32, x: f32, y: f32) -> bool {
        if let Some(node) = self.nodes.iter_mut().find(|n| (n.id - id).abs() < f32::EPSILON) {
            node.x = x;
            node.y = y;
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn set_node_flags(&mut self, id: f32, flags: f32) -> bool {
        if let Some(node) = self.nodes.iter_mut().find(|n| (n.id - id).abs() < f32::EPSILON) {
            node.flags = flags;
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.camera_x += dx;
        self.camera_y += dy;
        self.dirty = true;
    }

    // Zoom anchors to screen pivot point so canvas expands relative to cursor position
    pub fn zoom_at(&mut self, screen_x: f32, screen_y: f32, factor: f32, min_zoom: f32, max_zoom: f32) {
        let old_zoom = self.zoom;
        let new_zoom = (old_zoom * factor).clamp(min_zoom, max_zoom);
        if (new_zoom - old_zoom).abs() < f32::EPSILON {
            return;
        }

        let world_x = (screen_x - self.camera_x) / old_zoom;
        let world_y = (screen_y - self.camera_y) / old_zoom;

        self.zoom = new_zoom;
        self.camera_x = screen_x - world_x * new_zoom;
        self.camera_y = screen_y - world_y * new_zoom;
        self.dirty = true;
    }

    pub fn screen_to_world_x(&self, screen_x: f32) -> f32 {
        (screen_x - self.camera_x) / self.zoom
    }

    pub fn screen_to_world_y(&self, screen_y: f32) -> f32 {
        (screen_y - self.camera_y) / self.zoom
    }

    pub fn world_to_screen_x(&self, world_x: f32) -> f32 {
        world_x * self.zoom + self.camera_x
    }

    pub fn world_to_screen_y(&self, world_y: f32) -> f32 {
        world_y * self.zoom + self.camera_y
    }

    pub fn hit_test(&self, world_x: f32, world_y: f32) -> f32 {
        for node in self.nodes.iter().rev() {
            if world_x >= node.x
                && world_x <= node.x + node.width
                && world_y >= node.y
                && world_y <= node.y + node.height
            {
                return node.id;
            }
        }
        -1.0
    }
}
