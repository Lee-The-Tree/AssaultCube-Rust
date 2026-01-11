use crate::model;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use std::ffi::c_void;

pub fn world_to_screen(
    world_position: model::Vec3,
    view_matrix: [f32; 16],
    window_width: i32,
    window_height: i32,
) -> Option<model::Vec2> {
    let w = world_position.x * view_matrix[3]
        + world_position.y * view_matrix[7]
        + world_position.z * view_matrix[11]
        + view_matrix[15];

    if w < 0.001 {
        return None;
    }

    let x = world_position.x * view_matrix[0]
        + world_position.y * view_matrix[4]
        + world_position.z * view_matrix[8]
        + view_matrix[12];
    let y = world_position.x * view_matrix[1]
        + world_position.y * view_matrix[5]
        + world_position.z * view_matrix[9]
        + view_matrix[13];

    let nx = x / w;
    let ny = y / w;

    let window_center_x = (window_width / 2) as f32;
    let window_center_y = (window_height / 2) as f32;

    let screen_position = model::Vec2 {
        x: window_center_x + (window_center_x * nx),
        y: window_center_y - (window_center_y * ny),
    };

    Some(screen_position)
}

pub fn read_memory<T>(process_handle: isize, address: usize) -> T
where
    T: Copy + Default,
{
    let mut buffer = T::default();
    let mut bytes_read: usize = 0;
    unsafe {
        let _ = ReadProcessMemory(
            HANDLE(process_handle as *mut c_void),
            address as *const c_void,
            &mut buffer as *mut T as *mut c_void,
            std::mem::size_of::<T>(),
            Some(&mut bytes_read),
        );
    }
    buffer
}

pub fn write_memory<T>(process_handle: isize, address: usize, value: T)
where
    T: Copy,
{
    let mut bytes_written: usize = 0;
    unsafe {
        let _ = WriteProcessMemory(
            HANDLE(process_handle as *mut c_void),
            address as *const c_void,
            &value as *const T as *const c_void,
            std::mem::size_of::<T>(),
            Some(&mut bytes_written),
        );
    }
}

pub fn read_string(process_handle: isize, address: usize, max_len: usize) -> String {
    let mut buffer = vec![0u8; max_len];
    let mut bytes_read: usize = 0;
    unsafe {
        let _ = ReadProcessMemory(
            HANDLE(process_handle as *mut c_void),
            address as *const c_void,
            buffer.as_mut_ptr() as *mut c_void,
            max_len,
            Some(&mut bytes_read),
        );
    }

    if let Some(pos) = buffer.iter().position(|&x| x == 0) {
        buffer.truncate(pos);
    }

    String::from_utf8_lossy(&buffer).to_string()
}
