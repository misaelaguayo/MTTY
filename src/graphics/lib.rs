// Vertex structure representing a vertex with position and color attributes
// repr C ensures the struct has a predictable memory layout
// bytemuck traits allow for safe casting to/from byte slices
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            // The size of one vertex in bytes
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            // Indicates that each vertex is read per vertex (not per instance)
            step_mode: wgpu::VertexStepMode::Vertex,
            // Describes the attributes of the vertex (position and color)
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
