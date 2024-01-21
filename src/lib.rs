pub struct AudioBuffer
{
    channels : u32,
    bit_depth : u32,
    buffer_size : u32,
    data : * mut u8,
    ref_count : * mut u32
}
impl AudioBuffer
{
    pub fn init(channels : u32, bit_depth : u32, buffer_size : u32) -> Self
    {
        let size = bit_depth / 8 * buffer_size;
        unsafe
        {
            let ref_count = std::alloc::alloc(std::alloc::Layout::new::<u32>()) as * mut u32;
            *ref_count = 1;

            Self
            {
                channels,
                bit_depth,
                buffer_size,
                data: std::alloc::alloc_zeroed(std::alloc::Layout::array::<u8>(size as usize).expect("Cannot allocate memory.")),
                ref_count 
            }
        }
    }
    pub fn read(&self, index: u32) -> f64
    {
        let bit_depth = self.bit_depth / 8;
        let real_index = index % self.buffer_size * bit_depth;
        let limit = if self.bit_depth == 8 { i8::MAX as i32 }
        else if self.bit_depth == 16 { i16::MAX as i32 }
        else if self.bit_depth == 24 { 2_i32.pow(24) - 1 }
        else if self.bit_depth == 32 { i32::MAX }
        else { 0 };
        let mut data = 0;
        for bit in 0..bit_depth
        {
            data = data << 8 | unsafe { *self.data.offset((real_index + bit_depth - bit) as isize) as u32 }
        }
        if data > limit as u32 { 0.0 - ((data - limit as u32) as i32 / limit) as f64 } else { (data / limit as u32) as f64 }
    }
    pub fn write(&mut self, index: u32, data : f64)
    {
        let bit_depth = self.bit_depth / 8;
        let real_index = index % self.buffer_size * bit_depth;
        let limit = if self.bit_depth == 8 { i8::MAX as i32 }
        else if self.bit_depth == 16 { i16::MAX as i32 }
        else if self.bit_depth == 24 { 2_i32.pow(24) - 1 }
        else if self.bit_depth == 32 { i32::MAX }
        else { 0 };
        let restored = if data < 0.0 { (data * limit as f64) as i32 + limit } else { (data * limit as f64) as i32 + limit };
        for bit in 0..bit_depth
        {
            unsafe { *self.data.offset((real_index + bit) as isize) = (restored >> 8) as u8 }
        }
    }
    pub fn size(&self) -> u32 { self.buffer_size }
    pub fn real_size(&self) -> u32 { self.bit_depth / 8 * self.buffer_size }
    pub fn resize(&mut self, buffer_size : u32)
    {
        unsafe
        {
            let mut size = self.real_size();
            std::alloc::dealloc(self.data, std::alloc::Layout::array::<u8>(size as usize).expect("Cannot allocate memory."));
            self.buffer_size = buffer_size;
            size = self.real_size();
            self.data = std::alloc::alloc_zeroed(std::alloc::Layout::array::<u8>(size as usize).expect("Cannot allocate memory."));
        }
    }
    pub fn get_channel(&self) -> u32 { self.channels }
    pub fn set_channel(&mut self, channels : u32) { self.clear(); self.channels = channels; }
    pub fn get_depth(&self) -> u32 { self.bit_depth }
    pub fn set_depth(&mut self, bit_depth : u32) { self.clear(); self.bit_depth = bit_depth; }
    pub fn clear(&mut self)
    {
        for index in 0..self.real_size()
        {
            unsafe { *self.data.offset(index as isize) = 0; }
        }
    }
}
impl std::clone::Clone for AudioBuffer
{
    fn clone(&self) -> Self
    {
        unsafe { *self.ref_count += 1; }
        AudioBuffer
        {
            channels: self.channels,
            bit_depth: self.bit_depth,
            buffer_size: self.buffer_size,
            data : self.data,
            ref_count : self.ref_count,
        }
    }
}
impl std::default::Default for AudioBuffer
{
    fn default() -> Self
    {
        Self
        {
            channels: Default::default(),
            bit_depth: Default::default(),
            buffer_size: Default::default(),
            data: std::ptr::null_mut(),
            ref_count: std::ptr::null_mut()
        }
    }
}
unsafe impl Sync for AudioBuffer { }
unsafe impl Send for AudioBuffer { }
impl Drop for AudioBuffer
{
    fn drop(&mut self)
    {
        unsafe
        {
            if *self.ref_count == 1
            {
                let size = self.real_size();
                std::alloc::dealloc(self.data, std::alloc::Layout::array::<u8>(size as usize).expect("Cannot allocate memory."));
                std::alloc::dealloc(self.ref_count as * mut u8, std::alloc::Layout::new::<u32>());
            }
            else { *self.ref_count -= 1; }
        }
    }
}

pub struct AudioDevice
{
    name : String,
    sample_rate : u32,
    buffer : AudioBuffer,
    state : bool
}
impl AudioDevice
{
    pub fn name(&self) -> String { self.name.clone() }
    pub fn sample_rate(&self) -> u32 { self.sample_rate }
    pub fn buffer_size(&self) -> u32 { self.buffer.buffer_size }
    pub fn get_channel(&self) -> u32 { self.buffer.channels }
    pub fn get_depth(&self) -> u32 { self.buffer.bit_depth }
    pub fn read(&mut self)
    {
        self.state = true;
        while self.state
        {

        }
    }
    pub fn write(&mut self)
    {
        self.state = true;
        while self.state
        {

        }
    }
    pub fn stop(&mut self) { self.state = false; }
}
impl std::default::Default for AudioDevice
{
    fn default() -> Self 
    {
        Self
        {
            name: Default::default(),
            sample_rate: Default::default(),
            buffer: Default::default(),
            state: Default::default()
        }
    }
}
unsafe impl Sync for AudioDevice { }
unsafe impl Send for AudioDevice { }