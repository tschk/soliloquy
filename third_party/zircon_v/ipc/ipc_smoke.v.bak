module main

import ipc

fn main() {
    println('Running IPC smoke test...')
    
    // Create channel context
    mut ctx := ipc.channel_context_new() or {
        eprintln('Failed to create channel context: ${err}')
        exit(1)
    }
    
    // Test channel creation
    handle0, handle1, status := ctx.channel_create() or {
        eprintln('Failed to create channel: ${err}')
        exit(1)
    }
    
    if status != ipc.zx_ok {
        eprintln('Channel create returned bad status: ${status}')
        exit(1)
    }
    
    println('✓ Channel created: handle0=${handle0}, handle1=${handle1}')
    
    // Test channel write
    test_data := 'Hello from V IPC!'.bytes()
    write_status := ctx.channel_write(handle0, test_data, []) or {
        eprintln('Failed to write to channel: ${err}')
        exit(1)
    }
    
    if write_status != ipc.zx_ok {
        eprintln('Channel write returned bad status: ${write_status}')
        exit(1)
    }
    
    println('✓ Message written to channel')
    
    // Test channel read
    mut read_buffer := []u8{len: 64}
    mut handles_buffer := []ipc.ZxHandle{len: 8}
    
    data_size, num_handles, read_status := ctx.channel_read(handle1, mut read_buffer, mut handles_buffer) or {
        eprintln('Failed to read from channel: ${err}')
        exit(1)
    }
    
    if read_status != ipc.zx_ok {
        eprintln('Channel read returned bad status: ${read_status}')
        exit(1)
    }
    
    received_msg := read_buffer[..data_size].bytestr()
    expected_msg := 'Hello from V IPC!'
    
    if received_msg != expected_msg {
        eprintln('Message mismatch: got "${received_msg}", expected "${expected_msg}"')
        exit(1)
    }
    
    println('✓ Message received: "${received_msg}"')
    
    // Test channel close
    close_status0 := ctx.channel_close(handle0) or {
        eprintln('Failed to close handle0: ${err}')
        exit(1)
    }
    
    if close_status0 != ipc.zx_ok {
        eprintln('Channel close returned bad status: ${close_status0}')
        exit(1)
    }
    
    close_status1 := ctx.channel_close(handle1) or {
        eprintln('Failed to close handle1: ${err}')
        exit(1)
    }
    
    if close_status1 != ipc.zx_ok {
        eprintln('Channel close returned bad status: ${close_status1}')
        exit(1)
    }
    
    println('✓ Channels closed successfully')
    println('')
    println('✅ All IPC smoke tests passed!')
}
