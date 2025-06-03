// u32 vec with basic getters and setters
pub struct FrameBuffer {
    buf: Vec<u32>,
}

impl FrameBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            buf: vec![0; width * height],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    // read single value at index
    pub fn read(&self, index: usize) -> u32 {
        self.buf[index]
    }

    // write single value at index
    pub fn write(&mut self, index: usize, value: u32) {
        self.buf[index] = value;
    }

    // get ref to u32 vec
    pub fn raw(&self) -> &Vec<u32> {
        &self.buf
    }

    // returns rgb u8 vec
    pub fn rgb(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.buf.len() * 3);
        for &pixel in &self.buf {
            let r = ((pixel & 0xFF0000) >> 16) as u8;
            let g = ((pixel & 0x00FF00) >> 8) as u8;
            let b = (pixel & 0x0000FF) as u8;

            // Push the RGB components to the result vector
            result.push(r);
            result.push(g);
            result.push(b);
        }
        result
    }
}
