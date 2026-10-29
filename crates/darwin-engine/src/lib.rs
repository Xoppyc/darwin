use wasm_bindgen::prelude::*;

pub const NODE_STRIDE: usize = 6;
pub const EDGE_STRIDE: usize = 18;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeDef {
    pub id: f32,
    pub from_node: f32,
    pub to_node: f32,
    pub cardinality_from: f32,
    pub cardinality_to: f32,
}

// Fixed-stride edge command buffer:
// [0] id
// [1] point_count (always 4.0 for clean Manhattan routing)
// [2] from_node
// [3] to_node
// [4] source_normal_x (-1.0 = Left, +1.0 = Right)
// [5] target_normal_x (-1.0 = Left, +1.0 = Right)
// [6..17] waypoint pairs: (x0, y0, x1, y1, x2, y2, x3, y3, ...)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeBufferItem {
    pub id: f32,
    pub point_count: f32,
    pub from_node: f32,
    pub to_node: f32,
    pub source_normal_x: f32,
    pub target_normal_x: f32,
    pub points: [f32; 12],
}

#[wasm_bindgen]
pub struct DarwinEngine {
    nodes: Vec<NodeBufferItem>,
    edges: Vec<EdgeDef>,
    edge_buffer: Vec<EdgeBufferItem>,
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
            edges: Vec::with_capacity(128),
            edge_buffer: Vec::with_capacity(128),
            camera_x: 0.0,
            camera_y: 0.0,
            zoom: 1.0,
            dirty: true,
        }
    }

    pub fn node_stride() -> usize {
        NODE_STRIDE
    }

    pub fn edge_stride() -> usize {
        EDGE_STRIDE
    }

    pub fn nodes_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn nodes_len(&self) -> usize {
        self.nodes.len() * NODE_STRIDE
    }

    pub fn nodes_ptr(&self) -> *const f32 {
        self.nodes.as_ptr() as *const f32
    }

    pub fn edges_count(&self) -> usize {
        self.edge_buffer.len()
    }

    pub fn edges_len(&self) -> usize {
        self.edge_buffer.len() * EDGE_STRIDE
    }

    pub fn edges_ptr(&self) -> *const f32 {
        self.edge_buffer.as_ptr() as *const f32
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
        self.recompute_edges();
        self.dirty = true;
    }

    pub fn move_node(&mut self, id: f32, x: f32, y: f32) -> bool {
        if let Some(node) = self.nodes.iter_mut().find(|n| (n.id - id).abs() < f32::EPSILON) {
            node.x = x;
            node.y = y;
            self.recompute_edges();
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn add_edge(&mut self, id: f32, from_node: f32, to_node: f32, card_from: f32, card_to: f32) {
        self.edges.push(EdgeDef {
            id,
            from_node,
            to_node,
            cardinality_from: card_from,
            cardinality_to: card_to,
        });
        self.recompute_edges();
        self.dirty = true;
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

    // Recompute orthogonal waypoints using clean, predictable Manhattan routing
    fn recompute_edges(&mut self) {
        self.edge_buffer.clear();

        for edge in &self.edges {
            let from_opt = self.nodes.iter().find(|n| (n.id - edge.from_node).abs() < f32::EPSILON);
            let to_opt = self.nodes.iter().find(|n| (n.id - edge.to_node).abs() < f32::EPSILON);

            if let (Some(from), Some(to)) = (from_opt, to_opt) {
                let from_left = from.x;
                let from_right = from.x + from.width;
                let from_center_x = from.x + from.width * 0.5;
                let from_center_y = from.y + from.height * 0.5;

                let to_left = to.x;
                let to_right = to.x + to.width;
                let to_center_x = to.x + to.width * 0.5;
                let to_center_y = to.y + to.height * 0.5;

                let (p0_x, p0_y, src_normal, pend_x, pend_y, tgt_normal, bus_x) =
                    if to_left >= from_right {
                        // Case A: Target is cleanly to the right
                        let mid = (from_right + to_left) * 0.5;
                        (from_right, from_center_y, 1.0_f32, to_left, to_center_y, -1.0_f32, mid)
                    } else if to_right <= from_left {
                        // Case B: Target is cleanly to the left
                        let mid = (from_left + to_right) * 0.5;
                        (from_left, from_center_y, -1.0_f32, to_right, to_center_y, 1.0_f32, mid)
                    } else if to_center_x >= from_center_x {
                        // Case C1: Horizontally overlapping/stacked, route along right side
                        let bus = from_right.max(to_right) + 24.0;
                        (from_right, from_center_y, 1.0_f32, to_right, to_center_y, 1.0_f32, bus)
                    } else {
                        // Case C2: Horizontally overlapping/stacked, route along left side
                        let bus = from_left.min(to_left) - 24.0;
                        (from_left, from_center_y, -1.0_f32, to_left, to_center_y, -1.0_f32, bus)
                    };

                // Pure 4-point orthogonal path: P0 -> (bus, P0.y) -> (bus, Pend.y) -> Pend
                let mut pts = [0.0_f32; 12];
                pts[0] = p0_x;
                pts[1] = p0_y;
                pts[2] = bus_x;
                pts[3] = p0_y;
                pts[4] = bus_x;
                pts[5] = pend_y;
                pts[6] = pend_x;
                pts[7] = pend_y;

                self.edge_buffer.push(EdgeBufferItem {
                    id: edge.id,
                    point_count: 4.0,
                    from_node: edge.from_node,
                    to_node: edge.to_node,
                    source_normal_x: src_normal,
                    target_normal_x: tgt_normal,
                    points: pts,
                });
            }
        }
    }
}
