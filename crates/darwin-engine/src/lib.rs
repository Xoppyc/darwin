use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

pub const NODE_STRIDE: usize = 6;
pub const EDGE_STRIDE: usize = 18;
pub const HEADER_HEIGHT: f32 = 36.0;
pub const ROW_HEIGHT: f32 = 24.0;
pub const CARD_BOTTOM_PAD: f32 = 6.0;

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColumnDef {
    pub id: f32,
    pub name: String,
    pub data_type: String,
    pub is_pk: bool,
    pub is_fk: bool,
    pub is_nullable: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableNode {
    pub id: f32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub flags: f32,
    pub columns: Vec<ColumnDef>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeDef {
    pub id: f32,
    pub from_node: f32,
    pub from_col: f32,
    pub to_node: f32,
    pub to_col: f32,
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
    tables: Vec<TableNode>,
    edges: Vec<EdgeDef>,
    node_buffer: Vec<NodeBufferItem>,
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
            tables: Vec::with_capacity(64),
            edges: Vec::with_capacity(128),
            node_buffer: Vec::with_capacity(64),
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
        self.node_buffer.len()
    }

    pub fn nodes_len(&self) -> usize {
        self.node_buffer.len() * NODE_STRIDE
    }

    pub fn nodes_ptr(&self) -> *const f32 {
        self.node_buffer.as_ptr() as *const f32
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

    pub fn add_table(&mut self, id: f32, name: &str, x: f32, y: f32, width: f32) {
        let height = HEADER_HEIGHT + CARD_BOTTOM_PAD;
        self.tables.push(TableNode {
            id,
            name: name.to_string(),
            x,
            y,
            width,
            height,
            flags: 0.0,
            columns: Vec::with_capacity(16),
        });
        self.recompute_node_buffer();
        self.recompute_edges();
        self.dirty = true;
    }

    pub fn add_column(
        &mut self,
        table_id: f32,
        col_id: f32,
        name: &str,
        data_type: &str,
        is_pk: bool,
        is_fk: bool,
        is_nullable: bool,
    ) -> bool {
        if let Some(table) = self.tables.iter_mut().find(|t| (t.id - table_id).abs() < f32::EPSILON) {
            table.columns.push(ColumnDef {
                id: col_id,
                name: name.to_string(),
                data_type: data_type.to_string(),
                is_pk,
                is_fk,
                is_nullable,
            });
            table.height = HEADER_HEIGHT + (table.columns.len() as f32 * ROW_HEIGHT) + CARD_BOTTOM_PAD;
            self.recompute_node_buffer();
            self.recompute_edges();
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn move_node(&mut self, id: f32, x: f32, y: f32) -> bool {
        if let Some(table) = self.tables.iter_mut().find(|t| (t.id - id).abs() < f32::EPSILON) {
            table.x = x;
            table.y = y;
            self.recompute_node_buffer();
            self.recompute_edges();
            self.dirty = true;
            return true;
        }
        false
    }

    pub fn add_edge(
        &mut self,
        id: f32,
        from_node: f32,
        from_col: f32,
        to_node: f32,
        to_col: f32,
        card_from: f32,
        card_to: f32,
    ) {
        self.edges.push(EdgeDef {
            id,
            from_node,
            from_col,
            to_node,
            to_col,
            cardinality_from: card_from,
            cardinality_to: card_to,
        });
        self.recompute_edges();
        self.dirty = true;
    }

    // Static JSON export for initial UI mounting (never called inside render loop)
    pub fn get_schema_json(&self) -> String {
        serde_json::to_string(&self.tables).unwrap_or_else(|_| "[]".to_string())
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
        for table in self.tables.iter().rev() {
            if world_x >= table.x
                && world_x <= table.x + table.width
                && world_y >= table.y
                && world_y <= table.y + table.height
            {
                return table.id;
            }
        }
        -1.0
    }

    fn recompute_node_buffer(&mut self) {
        self.node_buffer.clear();
        for t in &self.tables {
            self.node_buffer.push(NodeBufferItem {
                id: t.id,
                x: t.x,
                y: t.y,
                width: t.width,
                height: t.height,
                flags: t.flags,
            });
        }
    }

    // Recompute orthogonal waypoints using clean, predictable Manhattan routing
    fn recompute_edges(&mut self) {
        self.edge_buffer.clear();

        for edge in &self.edges {
            let from_opt = self.tables.iter().find(|t| (t.id - edge.from_node).abs() < f32::EPSILON);
            let to_opt = self.tables.iter().find(|t| (t.id - edge.to_node).abs() < f32::EPSILON);

            if let (Some(from), Some(to)) = (from_opt, to_opt) {
                let from_left = from.x;
                let from_right = from.x + from.width;
                let from_center_x = from.x + from.width * 0.5;

                // Anchor port to column row center if specific column is linked
                let from_anchor_y = if edge.from_col >= 0.0 {
                    let col_idx = from
                        .columns
                        .iter()
                        .position(|c| (c.id - edge.from_col).abs() < f32::EPSILON)
                        .unwrap_or(0);
                    from.y + HEADER_HEIGHT + (col_idx as f32 * ROW_HEIGHT) + (ROW_HEIGHT * 0.5)
                } else {
                    from.y + from.height * 0.5
                };

                let to_left = to.x;
                let to_right = to.x + to.width;
                let to_center_x = to.x + to.width * 0.5;

                let to_anchor_y = if edge.to_col >= 0.0 {
                    let col_idx = to
                        .columns
                        .iter()
                        .position(|c| (c.id - edge.to_col).abs() < f32::EPSILON)
                        .unwrap_or(0);
                    to.y + HEADER_HEIGHT + (col_idx as f32 * ROW_HEIGHT) + (ROW_HEIGHT * 0.5)
                } else {
                    to.y + to.height * 0.5
                };

                let (p0_x, p0_y, src_normal, pend_x, pend_y, tgt_normal, bus_x) =
                    if to_left >= from_right {
                        // Case A: Target is cleanly to the right
                        let mid = (from_right + to_left) * 0.5;
                        (from_right, from_anchor_y, 1.0_f32, to_left, to_anchor_y, -1.0_f32, mid)
                    } else if to_right <= from_left {
                        // Case B: Target is cleanly to the left
                        let mid = (from_left + to_right) * 0.5;
                        (from_left, from_anchor_y, -1.0_f32, to_right, to_anchor_y, 1.0_f32, mid)
                    } else if to_center_x >= from_center_x {
                        // Case C1: Horizontally overlapping/stacked, route along right side
                        let bus = from_right.max(to_right) + 24.0;
                        (from_right, from_anchor_y, 1.0_f32, to_right, to_anchor_y, 1.0_f32, bus)
                    } else {
                        // Case C2: Horizontally overlapping/stacked, route along left side
                        let bus = from_left.min(to_left) - 24.0;
                        (from_left, from_anchor_y, -1.0_f32, to_left, to_anchor_y, -1.0_f32, bus)
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
