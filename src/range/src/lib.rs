#![no_std]
#![feature(abi_ptx)]
#![feature(stdsimd)]
#![feature(core_intrinsics)]

mod device;
use core::arch::nvptx::*;

#[panic_handler]
fn my_panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub unsafe extern "ptx-kernel" fn kernel(output: *mut i32, size: i32)
{
    let thread_id: i32 = _thread_idx_x();
    let block_id: i32 = _block_idx_x();
    
    let block_dim: i32 = _block_dim_x();
    let grid_dim: i32 = _grid_dim_x();
    
    let n_threads = block_dim * grid_dim;
    let thread_index =  thread_id + block_id * block_dim;

    let tasks = size / n_threads;
    let start = thread_index * tasks;
    let end = start + tasks;

    let mut i = start;
    while i < end && i < size {
        let value = device::device(i);
        *output.offset(i as isize) = value;
        i += 1;
    }
}
