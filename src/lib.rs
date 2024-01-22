use std::io::Write;

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

pub struct AudioDevice<R : std::io::Read, W : std::io::Write>
{
    name : String,
    sample_rate : u32,
    pub buffer : AudioBuffer,
    state : bool,
    reader : std::io::BufReader<R>,
    writer : std::io::BufWriter<W>
}
impl<R : std::io::Read, W : std::io::Write> AudioDevice<R, W>
{
    pub fn init(name : &str, reader : R, writer : W) -> Self
    {
        Self
        {
            name: String::from(name),
            sample_rate: u32::default(),
            buffer: AudioBuffer::default(),
            state: bool::default(),
            reader: std::io::BufReader::new(reader),
            writer: std::io::BufWriter::new(writer),
        }
    }
    pub fn name(&self) -> String { self.name.clone() }
    pub fn get_sample_rate(&self) -> u32 { self.sample_rate }
    pub fn set_sample_rate(&mut self, sample_rate : u32)
    {
        if sample_rate % 22050 == 0
        {
            self.sample_rate = sample_rate;
            let state_data = ((sample_rate / 22050) as u8 - 1) << 2 | 0b01;
            self.inner_write(0, state_data);
            return;
        }
        else if sample_rate % 24000 == 0
        {
            self.sample_rate = sample_rate;
            let state_data = ((sample_rate / 24000) as u8 + 31) << 2 | 0b01;
            self.inner_write(0, state_data);
            return;
        }
        println!("Invalid sample rate!");
    }
    pub fn get_buffer_size(&self) -> u32 { self.buffer.buffer_size }
    pub fn set_buffer_size(&mut self, buffer_size : u32)
    {
        if buffer_size % 32 == 0
        {
            self.buffer.buffer_size = buffer_size;
            let state_data = ((buffer_size / 32) as u8) << 2 | 0b10;
            self.inner_write(0, state_data);
            return;
        }
        println!("Invalid buffer size!");
    }
    pub fn channels(&self) -> u32 { self.buffer.channels }
    pub fn bit_depth(&self) -> u32 { self.buffer.bit_depth }
    pub fn read(&mut self)
    {
        self.state = true;
        while self.state
        {
            let buffer = self.reader.buffer();
            let len = buffer.len();
            if len == 0 
            {
                println!("Buffer is empty!");
                continue;
            }
            let state_var = buffer[0] & 0b11;
            let state_data = buffer[0] >> 2;
            if state_var == 0b01
            {
                self.sample_rate = if state_data < 32 { (state_data + 1) as u32 * 22050 } else { (state_data - 31) as u32 * 24000  }
            }
            else if state_var == 0b10 { self.buffer.buffer_size = (state_data + 1) as u32 * 32 }
            else if state_var == 0b11 { self.buffer.channels = (state_data as u32) + 1 }
            if len == 1 { continue; }
            for index in 1..len { unsafe { *self.buffer.data.offset(index as isize) = buffer[index]; } }
            std::thread::sleep(std::time::Duration::from_secs_f64(1.0 / self.sample_rate as f64));
        }
    }
    pub fn write(&mut self)
    {
        self.state = true;
        let mut count = 0;
        while self.state
        {
            self.inner_write(count, 0);
            std::thread::sleep(std::time::Duration::from_secs_f64(1.0 / self.sample_rate as f64));
            count += 1;
        }
    }
    fn inner_write(&mut self, count : usize, state_var : u8)
    {
        if count == 0 { let _ = self.writer.write(&[state_var]); }
        else
        {
            let mut data = vec![0; self.buffer.buffer_size as usize + 1];
            for index in 1..data.len() { data[index] = unsafe { *self.buffer.data.offset(index as isize) }; }
            let _ = self.writer.write(&data);
        }
    }
    pub fn stop(&mut self) { self.state = false; }
}