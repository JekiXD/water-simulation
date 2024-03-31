use wgpu::QUERY_SET_MAX_QUERIES;

pub struct FrameTime {
    last_printed_instant: web_time::Instant,
    elapsed_secs: f32,
}

impl FrameTime {
    fn new() -> Self {
        Self {
            last_printed_instant: web_time::Instant::now(),
            elapsed_secs:  1.0 / 120.0,
        }
    }

    fn update(&mut self) {
        let new_instant = web_time::Instant::now();
       // self.elapsed_secs = (new_instant - self.last_printed_instant).as_secs_f32();
        self.last_printed_instant = new_instant;
    }
}

pub struct FrameTimeState {
    pub time: FrameTime,
    pub buffer: wgpu::Buffer
}

impl FrameTimeState {
    pub fn new(device: &wgpu::Device) -> Self {
        let time = FrameTime::new();

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Frame time buffer"),
            size: std::mem::size_of::<f32>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false
        });

        FrameTimeState {
            time,
            buffer
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.time.update();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.time.elapsed_secs]));
    }   
}