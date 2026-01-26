module ipc

struct ChannelEndpoint {
mut:
    message_queue MessageQueue
    peer          &ChannelEndpoint = unsafe { nil }
    is_closed     bool
    ref_count     u32
}

pub struct Channel {
mut:
    endpoint0 ChannelEndpoint
    endpoint1 ChannelEndpoint
}

// Simple handle table context passed explicitly
// In production code, this would use OS-level thread-local storage
pub struct ChannelContext {
mut:
    handle_table HandleTable
}

pub fn channel_context_new() !ChannelContext {
    table := handle_table_init(64)!
    return ChannelContext{
        handle_table: table
    }
}

fn channel_endpoint_init() ChannelEndpoint {
    return ChannelEndpoint{
        message_queue: message_queue_init()
        is_closed: false
        ref_count: 1
    }
}

pub fn (mut ctx ChannelContext) channel_create() !(ZxHandle, ZxHandle, ZxStatus) {
    mut channel := &Channel{
        endpoint0: channel_endpoint_init()
        endpoint1: channel_endpoint_init()
    }
    
    channel.endpoint0.peer = &channel.endpoint1
    channel.endpoint1.peer = &channel.endpoint0
    
    handle0, status0 := ctx.handle_table.alloc(&channel.endpoint0, 
        zx_right_read | zx_right_write | zx_right_transfer)!
    if status0 != zx_ok {
        return error('failed to allocate handle0')
    }
    
    handle1, status1 := ctx.handle_table.alloc(&channel.endpoint1,
        zx_right_read | zx_right_write | zx_right_transfer)!
    if status1 != zx_ok {
        unsafe {
            ctx.handle_table.close(handle0) or {}
        }
        return error('failed to allocate handle1')
    }
    
    return handle0, handle1, zx_ok
}

pub fn (mut ctx ChannelContext) channel_write(handle ZxHandle, data []u8, handles []ZxHandle) !ZxStatus {
    if handle == zx_handle_invalid {
        return error('invalid handle')
    }
    
    obj, status := ctx.handle_table.get(handle, zx_right_write)!
    if status != zx_ok {
        return error('failed to get handle')
    }
    
    mut endpoint := unsafe { &ChannelEndpoint(obj) }
    
    if endpoint.is_closed || isnil(endpoint.peer) || endpoint.peer.is_closed {
        return error('channel closed')
    }
    
    mut packet := message_packet_create(data, handles)!
    
    unsafe {
        endpoint.peer.message_queue.enqueue(mut packet)
    }
    
    return zx_ok
}

pub fn (mut ctx ChannelContext) channel_read(handle ZxHandle, mut data []u8, mut handles []ZxHandle) !(u32, u32, ZxStatus) {
    if handle == zx_handle_invalid {
        return error('invalid handle')
    }
    
    obj, status := ctx.handle_table.get(handle, zx_right_read)!
    if status != zx_ok {
        return error('failed to get handle')
    }
    
    mut endpoint := unsafe { &ChannelEndpoint(obj) }
    
    if endpoint.is_closed {
        return error('channel closed')
    }
    
    if endpoint.message_queue.is_empty() {
        return error('no messages available')
    }
    
    mut packet := endpoint.message_queue.dequeue() or {
        return error('failed to dequeue message')
    }
    
    actual_data_size := packet.data_size
    actual_num_handles := packet.num_handles
    
    if data.len >= int(packet.data_size) {
        for i in 0 .. packet.data_size {
            data[i] = packet.data[i]
        }
    }
    
    if handles.len >= int(packet.num_handles) {
        for i in 0 .. packet.num_handles {
            handles[i] = packet.handles[i]
        }
    }
    
    unsafe {
        packet.destroy()
        free(packet)
    }
    
    return actual_data_size, actual_num_handles, zx_ok
}

pub fn (mut ctx ChannelContext) channel_close(handle ZxHandle) !ZxStatus {
    if handle == zx_handle_invalid {
        return error('invalid handle')
    }
    
    obj, status := ctx.handle_table.get(handle, zx_right_none)!
    if status != zx_ok {
        return error('failed to get handle')
    }
    
    mut endpoint := unsafe { &ChannelEndpoint(obj) }
    
    endpoint.is_closed = true
    endpoint.message_queue.destroy()
    
    if !isnil(endpoint.peer) {
        unsafe {
            endpoint.peer.peer = nil
        }
    }
    
    return ctx.handle_table.close(handle)!
}
