/// Audio buffer container to read or write byte data into audio sample.
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
    /// Create an AudioBuffer.
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
    /// Read a single sample from the index.
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
    /// Read a whole buffer into a given slice.
    pub fn read_slice(&self, buffer : &mut [f64])
    {
        if buffer.len() != self.buffer_size as usize { return; }
        for index in 0..self.buffer_size { buffer[index as usize] = self.read(index); }
    }
    /// Write a single sample from the index.
    pub fn write(&mut self, index: u32, data : f64)
    {
        let bit_depth = self.bit_depth / 8;
        let real_index = index % self.buffer_size * bit_depth;
        let limit = if self.bit_depth == 8 { i8::MAX as i32 }
        else if self.bit_depth == 16 { i16::MAX as i32 }
        else if self.bit_depth == 24 { 2_i32.pow(24) - 1 }
        else if self.bit_depth == 32 { i32::MAX }
        else { 0 };
        let restored = if data < 0.0 { (data * limit as f64) as i32 } else { (data * limit as f64) as i32 };
        for bit in 0..bit_depth
        {
            unsafe
            {
                *self.data.offset((real_index + bit) as isize) += ((restored & (0xFF << bit * 8)) >> (bit * 8)) as u8
            }
        }
    }
    /// Write a whole buffer from a given slice.
    pub fn write_slice(&mut self, buffer : &[f64])
    {
        if buffer.len() != self.buffer_size as usize { return; }
        for index in 0..self.buffer_size { self.write(index, buffer[index as usize]); }
    }
    /// Get a buffer size.
    pub fn size(&self) -> u32 { self.buffer_size }
    /// Get a size of a byte container.
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
    /// Clear the buffer.
    pub fn clear(&mut self) { for index in 0..self.real_size() { unsafe { *self.data.offset(index as isize) = 0; } } }
}
impl std::clone::Clone for AudioBuffer
{
    fn clone(&self) -> Self
    {
        unsafe { *self.ref_count += 1; }
        Self
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

/// Audio device for various reader and writer type.
pub struct AudioDevice<R : std::io::Read, W : std::io::Write>
{
    name : String,
    sample_rate : u32,
    in_buffer : AudioBuffer,
    out_buffer : AudioBuffer,
    reader : std::io::BufReader<R>,
    writer : std::io::BufWriter<W>
}
impl<R : std::io::Read, W : std::io::Write> AudioDevice<R, W>
{
    /// Create an AudioDevice.
    pub fn init(name : &str, reader : R, writer : W) -> Self
    {
        Self
        {
            name: String::from(name),
            sample_rate: u32::default(),
            in_buffer: AudioBuffer::default(),
            out_buffer: AudioBuffer::default(),
            reader: std::io::BufReader::new(reader),
            writer: std::io::BufWriter::new(writer),
        }
    }
    /// Get name of the device.
    pub fn name(&self) -> String { self.name.clone() }
    /// Get sample rate of the device.
    pub fn get_sample_rate(&self) -> u32 { self.sample_rate }
    /// Set sample rate of the device.
    pub fn set_sample_rate(&mut self, sample_rate : u32)
    {
        if sample_rate % 22050 == 0
        {
            self.sample_rate = sample_rate;
            let state_data = ((sample_rate / 22050) as u8 - 1) << 2 | 0b01;
            let _ = std::io::Write::write(&mut self.writer, &[state_data]);
            return;
        }
        else if sample_rate % 24000 == 0
        {
            self.sample_rate = sample_rate;
            let state_data = ((sample_rate / 24000) as u8 + 31) << 2 | 0b01;
            let _ = std::io::Write::write(&mut self.writer, &[state_data]);
            return;
        }
        println!("Invalid sample rate!");
    }
    /// Get buffer size of the input.
    pub fn get_in_buffer_size(&self) -> u32 { self.in_buffer.buffer_size }
    /// Get buffer size of the output.
    pub fn get_out_buffer_size(&self) -> u32 { self.out_buffer.buffer_size }
    /// Get channel count of the input.
    pub fn get_in_channels(&self) -> u32 { self.in_buffer.channels }
    /// Get channel count of the output.
    pub fn get_out_channels(&self) -> u32 { self.out_buffer.channels }
    /// Get bit depth of the input.
    pub fn get_in_bit_depth(&self) -> u32 { self.in_buffer.bit_depth }
    /// Get bit depth of the output.
    pub fn get_out_bit_depth(&self) -> u32 { self.out_buffer.bit_depth }
    fn read(&mut self)
    {
        std::io::BufRead::fill_buf(&mut self.reader).expect("Failed to fill the buffer.");
        let buffer = self.reader.buffer();
        let len = buffer.len();
        if len == 0 
        {
            println!("Buffer is empty!");
            return;
        }
        let state_var = buffer[0] & 0b11;
        let state_data = buffer[0] >> 2;
        if state_var == 0b01
        {
            self.sample_rate = if state_data < 32 { (state_data + 1) as u32 * 22050 } else { (state_data - 31) as u32 * 24000  }
        }
        else if state_var == 0b10 { self.in_buffer.buffer_size = (state_data + 1) as u32 * 32 }
        else if state_var == 0b11 { self.in_buffer.channels = (state_data as u32) + 1 }
        if len == 1 { return; }
        if self.in_buffer.bit_depth != 8 * (buffer.len() - 1) as u32 / (self.in_buffer.buffer_size * self.in_buffer.channels)
        {
            println!("Bit depth changed!");
            self.in_buffer.bit_depth = 8 * (buffer.len() - 1) as u32 / (self.in_buffer.buffer_size * self.in_buffer.channels);
        }
        for index in 1..len { unsafe { *self.in_buffer.data.offset(index as isize) = buffer[index]; } }
    }
    fn write(&mut self)
    {
        let mut data = vec![0; self.out_buffer.buffer_size as usize + 1];
        for index in 1..data.len() { data[index] = unsafe { *self.out_buffer.data.offset(index as isize) }; }
        let _ = std::io::Write::write(&mut self.writer, &data);
        self.out_buffer.clear();
    }
    /// Play until state is false.
    pub fn play(&mut self, state : &bool)
    {
        while *state
        {
            let start_time = std::time::Instant::now();

            self.read();
            self.write();

            let elapsed_time = start_time.elapsed();
            let duration = std::time::Duration::from_secs_f64(self.in_buffer.buffer_size as f64 / self.sample_rate as f64);

            if elapsed_time >= duration { continue; }
            else { std::thread::sleep(duration - elapsed_time); }
        }
    }
}
unsafe impl<R : std::io::Read, W : std::io::Write> Sync for AudioDevice<R, W> { }
unsafe impl<R : std::io::Read, W : std::io::Write> Send for AudioDevice<R, W> { }