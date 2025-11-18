use crate::math::Vector;

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct GlyphQuadVertex {
    pub(crate) position: [f32; 2],
    pub(crate) uv: [f32; 2],
    pub(crate) color: [f32; 4],
}
impl GlyphQuadVertex {
    pub fn new(position: Vector, uv: Vector, color: Vector) -> GlyphQuadVertex {
        GlyphQuadVertex {
            position: position.to_array2(),
            uv: uv.to_array2(),
            color: color.to_array4()
        }
    }
}

#[derive(Debug)]
pub struct Glyph {
    pub uv_min: Vector,
    pub uv_max: Vector,
    pub plane_min: Vector,
    pub plane_max: Vector,
    pub advance: f32,
}
impl Glyph {
    pub fn get_quad(&self, position: Vector, scale_factor: &Vector, color: &Vector) -> [GlyphQuadVertex; 4] {
        let position_extent = (self.plane_max - self.plane_min) * scale_factor;
        let uv_extent = self.uv_max - self.uv_min;

        let p = position * scale_factor;
        let bl = GlyphQuadVertex::new( // min
                                       p + (self.plane_min * scale_factor),
                                       self.uv_min,
                                       color.clone()
        );
        let tl = GlyphQuadVertex::new(
            p + (self.plane_min * scale_factor) + Vector::new_vec2(0.0, position_extent.y),
            self.uv_min + Vector::new_vec2(0.0, uv_extent.y),
            color.clone()
        );
        let tr = GlyphQuadVertex::new( // max
                                       p + (self.plane_max * scale_factor),
                                       self.uv_max,
                                       color.clone()
        );
        let br = GlyphQuadVertex::new(
            p + (self.plane_min * scale_factor) + Vector::new_vec2(position_extent.x, 0.0),
            self.uv_min + Vector::new_vec2(uv_extent.x, 0.0),
            color.clone()
        );
        [bl, tl, tr, br]
    }
    pub fn push_to_buffers(&self, vertex_buffer: &mut Vec<GlyphQuadVertex>, index_buffer: &mut Vec<u32>, position: Vector, scale_factor: &Vector, color: &Vector) {
        let v = vertex_buffer.len() as u32;
        let [bl, tl, tr, br] = self.get_quad(position, &scale_factor, &color);
        vertex_buffer.push(bl); vertex_buffer.push(tl); vertex_buffer.push(tr); vertex_buffer.push(br);
        index_buffer.push(v); index_buffer.push(v + 1); index_buffer.push(v + 2);
        index_buffer.push(v); index_buffer.push(v + 2); index_buffer.push(v + 3);
    }
}