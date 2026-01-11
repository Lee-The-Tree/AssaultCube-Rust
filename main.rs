mod model;
mod offset;
mod util;

use anyhow::Result;
use eframe::egui;
use std::{
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{
            Diagnostics::ToolHelp::*,
            Threading::*,
            LibraryLoader::GetModuleHandleW,
        },
        UI::{WindowsAndMessaging::*, Input::KeyboardAndMouse::*},
        Graphics::Gdi::*,
    },
};

const FRAME_RATE: u64 = 60;
const TICK_RATE: Duration = Duration::from_millis(1000 / FRAME_RATE);

struct EspBox {
    rect: RECT,
    name: String,
    feet_pos: POINT,
}

struct AppState {
    esp_enabled: bool,
    show_names: bool,
    show_snaplines: bool,
    aimbot_enabled: bool,
    aimbot_fov: f32,
    aimbot_smoothness: f32,
    aimbot_max_dist: f32,
    aimbot_vis_check: bool,
    show_fov_circle: bool,
    inf_ammo: bool,
    inf_hp: bool,
    no_recoil: bool,
    attached: bool,
    process_id: Option<u32>,
    player_count: u32,
    box_count: usize,
    module_base: usize,
    entity_list_ptr: usize,
    box_color: [f32; 3],
    current_frame: u32,
    debug_last_vis: u32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            esp_enabled: false,
            show_names: true,
            show_snaplines: false,
            aimbot_enabled: false,
            aimbot_fov: 30.0,
            aimbot_smoothness: 5.0,
            aimbot_max_dist: 100.0,
            aimbot_vis_check: true,
            show_fov_circle: true,
            inf_ammo: false,
            inf_hp: false,
            no_recoil: false,
            attached: false,
            process_id: None,
            player_count: 0,
            box_count: 0,
            module_base: 0,
            entity_list_ptr: 0,
            box_color: [1.0, 0.0, 0.0], // Default Red
            current_frame: 0,
            debug_last_vis: 0,
        }
    }
}

struct GuiApp {
    state: Arc<RwLock<AppState>>,
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(mut state) = self.state.write() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("AssaultCube Multi-Hack");
                ui.add_space(10.0);

                if state.attached {
                    ui.label(egui::RichText::new("Status: Attached").color(egui::Color32::GREEN));
                    ui.label(format!("Players in Memory: {}", state.player_count));
                } else {
                    ui.label(egui::RichText::new("Status: Waiting for game...").color(egui::Color32::YELLOW));
                }

                ui.separator();
                ui.heading("ESP Settings");
                ui.checkbox(&mut state.esp_enabled, "Enable ESP");
                ui.checkbox(&mut state.show_names, "Show Player Names");
                ui.checkbox(&mut state.show_snaplines, "Show Snaplines");

                ui.separator();
                ui.heading("Aimbot Settings");
                ui.checkbox(&mut state.aimbot_enabled, "Enable Aimbot (Hold Right Click)");
                ui.add(egui::Slider::new(&mut state.aimbot_fov, 1.0..=180.0).text("FOV"));
                ui.add(egui::Slider::new(&mut state.aimbot_smoothness, 1.0..=50.0).text("Smoothness"));
                ui.add(egui::Slider::new(&mut state.aimbot_max_dist, 10.0..=500.0).text("Max Distance"));
                ui.checkbox(&mut state.aimbot_vis_check, "Visibility Check");
                ui.checkbox(&mut state.show_fov_circle, "Show FOV Circle");

                ui.separator();
                ui.heading("Misc Settings");
                ui.checkbox(&mut state.inf_ammo, "Infinite Ammo (420)");
                ui.checkbox(&mut state.inf_hp, "Infinite HP (67)");
                ui.checkbox(&mut state.no_recoil, "No Recoil");

                ui.separator();
                ui.add_space(5.0);
                ui.label("Theme Color:");
                ui.color_edit_button_rgb(&mut state.box_color);
            });
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn main() -> Result<()> {
    let app_state = Arc::new(RwLock::new(AppState::default()));
    let app_state_clone = app_state.clone();

    thread::spawn(move || {
        loop {
            if let Err(e) = run_esp(app_state_clone.clone()) {
                eprintln!("Hack Error: {:?}", e);
                if let Ok(mut state) = app_state_clone.write() {
                    state.attached = false;
                    state.process_id = None;
                    state.player_count = 0;
                    state.box_count = 0;
                }
            }
            thread::sleep(Duration::from_secs(2));
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([350.0, 850.0]),
        ..Default::default()
    };

    eframe::run_native(
        "AC Multi-Hack",
        options,
        Box::new(|_cc| Ok(Box::new(GuiApp { state: app_state }))),
    ).map_err(|e| anyhow::anyhow!("Eframe error: {}", e))?;

    Ok(())
}

fn run_esp(app_state: Arc<RwLock<AppState>>) -> Result<()> {
    let draw_rect_list = Arc::new(RwLock::new(Vec::<EspBox>::with_capacity(32)));

    let game_window = loop {
        if let Ok(hwnd) = unsafe { FindWindowW(None, w!("AssaultCube")) } {
            if !hwnd.is_invalid() {
                break hwnd;
            }
        }
        thread::sleep(Duration::from_secs(1));
    };

    let mut process_id: u32 = 0;
    unsafe { GetWindowThreadProcessId(game_window, Some(&mut process_id)) };

    let process_handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, process_id) }?;
    let module_base_addr = get_module_base_address(process_id, "ac_client.exe")?;

    if let Ok(mut state) = app_state.write() {
        state.attached = true;
        state.process_id = Some(process_id);
        state.module_base = module_base_addr;
    }

    let draw_rect_list_clone = Arc::clone(&draw_rect_list);
    let app_state_overlay = Arc::clone(&app_state);
    let game_window_ptr = game_window.0 as isize;

    thread::spawn(move || {
        let hwnd_game = HWND(game_window_ptr as *mut _);
        let h_module = unsafe { GetModuleHandleW(None).unwrap() };
        let h_instance = HINSTANCE(h_module.0);
        let window_class = w!("OverlayClass");

        let wc = WNDCLASSW {
            hInstance: h_instance,
            lpszClassName: window_class,
            lpfnWndProc: Some(wnd_proc),
            ..Default::default()
        };

        unsafe { RegisterClassW(&wc); }

        let hwnd_overlay = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_LAYERED,
                window_class,
                w!("Overlay"),
                WS_POPUP,
                0, 0, 1920, 1080,
                None, None, Some(h_instance), None
            ).unwrap()
        };

        unsafe {
            let _ = SetLayeredWindowAttributes(hwnd_overlay, COLORREF(0), 255, LWA_COLORKEY);
            let _ = ShowWindow(hwnd_overlay, SW_SHOW);
        }

        loop {
            let mut msg = MSG::default();
            while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            let mut window_info = WINDOWINFO::default();
            if unsafe { GetWindowInfo(hwnd_game, &mut window_info) }.is_err() { break; }

            let width = window_info.rcClient.right - window_info.rcClient.left;
            let height = window_info.rcClient.bottom - window_info.rcClient.top;

            let mut point = POINT { x: 0, y: 0 };
            unsafe { let _ = ClientToScreen(hwnd_game, &mut point); }

            unsafe {
                let _ = SetWindowPos(hwnd_overlay, Some(HWND_TOPMOST), point.x, point.y, width, height, SWP_SHOWWINDOW | SWP_NOACTIVATE);
            }

            let hdc = unsafe { GetDC(Some(hwnd_overlay)) };
            if !hdc.is_invalid() {
                let rect = RECT { left: 0, top: 0, right: width, bottom: height };
                unsafe {
                    let black_brush = GetStockObject(BLACK_BRUSH);
                    FillRect(hdc, &rect, HBRUSH(black_brush.0));
                }

                let (color, show_names, show_snaplines, show_fov, fov_val) = if let Ok(state) = app_state_overlay.read() {
                    let c = state.box_color;
                    let r = (c[0] * 255.0) as u32;
                    let g = (c[1] * 255.0) as u32;
                    let b = (c[2] * 255.0) as u32;
                    (COLORREF(b << 16 | g << 8 | r), state.show_names, state.show_snaplines, state.show_fov_circle, state.aimbot_fov)
                } else {
                    (COLORREF(0x0000FF), true, false, true, 30.0)
                };

                if show_fov {
                    unsafe {
                        let pen = CreatePen(PS_SOLID, 1, color);
                        let old_pen = SelectObject(hdc, HGDIOBJ(pen.0));
                        let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
                        let radius = (fov_val * (width as f32 / 90.0)) as i32;
                        let _ = Ellipse(hdc, width/2 - radius, height/2 - radius, width/2 + radius, height/2 + radius);
                        let _ = SelectObject(hdc, old_pen);
                        let _ = SelectObject(hdc, old_brush);
                        let _ = DeleteObject(HGDIOBJ(pen.0));
                    }
                }

                if let Ok(boxes) = draw_rect_list_clone.read() {
                    for esp_box in boxes.iter() {
                        unsafe {
                            let brush = CreateSolidBrush(color);
                            FrameRect(hdc, &esp_box.rect, brush);

                            if show_names {
                                let _ = SetTextColor(hdc, COLORREF(0xFFFFFF));
                                let _ = SetBkMode(hdc, TRANSPARENT);
                                let name_wide: Vec<u16> = esp_box.name.encode_utf16().collect();
                                let _ = TextOutW(hdc, esp_box.rect.left, esp_box.rect.top - 15, &name_wide);
                            }

                            if show_snaplines {
                                let pen = CreatePen(PS_SOLID, 1, color);
                                let old_pen = SelectObject(hdc, HGDIOBJ(pen.0));
                                let _ = MoveToEx(hdc, width / 2, height, None);
                                let _ = LineTo(hdc, esp_box.feet_pos.x, esp_box.feet_pos.y);
                                let _ = SelectObject(hdc, old_pen);
                                let _ = DeleteObject(HGDIOBJ(pen.0));
                            }

                            let _ = DeleteObject(HGDIOBJ(brush.0));
                        }
                    }
                }
                unsafe { ReleaseDC(Some(hwnd_overlay), hdc); }
            }
            thread::sleep(TICK_RATE);
        }
    });

    read_game_data_loop(
        process_handle.0 as isize,
        module_base_addr,
        game_window.0 as isize,
        draw_rect_list,
        app_state,
    );

    Ok(())
}

fn get_module_base_address(process_id: u32, module_name: &str) -> Result<usize> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, process_id) }?;

    let mut entry = MODULEENTRY32::default();
    entry.dwSize = std::mem::size_of::<MODULEENTRY32>() as u32;

    if unsafe { Module32First(snapshot, &mut entry) }.is_ok() {
        loop {
            let current_name = unsafe {
                std::ffi::CStr::from_ptr(entry.szModule.as_ptr() as *const i8)
                    .to_string_lossy()
            };

            if current_name == module_name {
                unsafe { CloseHandle(snapshot) }?;
                return Ok(entry.modBaseAddr as usize);
            }

            if unsafe { Module32Next(snapshot, &mut entry) }.is_err() {
                break;
            }
        }
    }

    unsafe { CloseHandle(snapshot) }?;
    Err(anyhow::anyhow!("Module not found"))
}

fn read_game_data_loop(
    process_handle: isize,
    module_base_addr: usize,
    game_window: isize,
    draw_rect_list: Arc<RwLock<Vec<EspBox>>>,
    app_state: Arc<RwLock<AppState>>,
) {
    let mut last_tick = Instant::now();
    loop {
        let (is_enabled, aimbot_enabled, aimbot_fov, aimbot_smoothness, aimbot_max_dist, aimbot_vis_check, inf_ammo, inf_hp, no_recoil) = if let Ok(state) = app_state.read() {
            (state.esp_enabled, state.aimbot_enabled, state.aimbot_fov, state.aimbot_smoothness, state.aimbot_max_dist, state.aimbot_vis_check, state.inf_ammo, state.inf_hp, state.no_recoil)
        } else {
            (false, false, 30.0, 5.0, 100.0, true, false, false, false)
        };

        let mut window_info = WINDOWINFO::default();
        if unsafe { GetWindowInfo(HWND(game_window as *mut _), &mut window_info) }.is_err() {
             break;
        }

        let window_width = window_info.rcClient.right - window_info.rcClient.left;
        let window_height = window_info.rcClient.bottom - window_info.rcClient.top;

        let entity_list_base_addr = util::read_memory::<u32>(process_handle, module_base_addr + offset::ENTITY_LIST as usize) as usize;
        let player_count = util::read_memory::<u32>(process_handle, module_base_addr + offset::PLAYER_COUNT as usize);
        let local_player_ptr = util::read_memory::<u32>(process_handle, module_base_addr + offset::LOCAL_PLAYER as usize) as usize;

        let current_frame = util::read_memory::<u32>(process_handle, module_base_addr + offset::CURRENT_FRAME as usize);

        if let Ok(mut state) = app_state.write() {
            state.player_count = player_count;
            state.entity_list_ptr = entity_list_base_addr;
            state.current_frame = current_frame;
        }

        // Apply Misc Hacks
        if inf_ammo {
            util::write_memory::<i32>(process_handle, local_player_ptr + offset::ENTITY_AMMO as usize, 420);
        }
        if inf_hp {
            util::write_memory::<i32>(process_handle, local_player_ptr + offset::ENTITY_HEALTH as usize, 67);
        }
        if no_recoil {
            util::write_memory::<i32>(process_handle, local_player_ptr + offset::LOCAL_RECOIL as usize, 0);
        }

        let local_player = model::Entity { process_handle, base_addr: local_player_ptr };
        let local_team = local_player.team();
        let local_pos = local_player.head_position();

        let current_yaw = util::read_memory::<f32>(process_handle, local_player_ptr + offset::LOCAL_YAW as usize);
        let current_pitch = util::read_memory::<f32>(process_handle, local_player_ptr + offset::LOCAL_PITCH as usize);

        let mut best_target: Option<(f32, f32)> = None;
        let mut closest_fov = aimbot_fov;
        let mut new_draw_rect_list = Vec::new();
        let mut debug_last_vis = 0;

        if entity_list_base_addr != 0 {
            let view_matrix = util::read_memory::<[f32; 16]>(process_handle, module_base_addr + offset::VIEW_MATRIX as usize);

            for i in 1..player_count {
                let entity_ptr = util::read_memory::<u32>(process_handle, entity_list_base_addr + (i as usize * 4));
                if entity_ptr == 0 { continue; }

                let entity = model::Entity {
                    process_handle,
                    base_addr: entity_ptr as usize,
                };

                let health = entity.health();
                if health <= 0 || health > 1000 { continue; }

                let is_enemy = entity.team() != local_team;
                let head_pos = entity.head_position();
                let dist = local_pos.distance(&head_pos);

                let last_vis_frame = util::read_memory::<u32>(process_handle, entity_ptr as usize + offset::ENTITY_LAST_VIS_FRAME as usize);
                if i == 1 { debug_last_vis = last_vis_frame; }

                // Aimbot Logic
                if aimbot_enabled && is_enemy && dist <= aimbot_max_dist {
                    let mut can_target = true;
                    if aimbot_vis_check {
                        if last_vis_frame < current_frame { can_target = false; }
                    }

                    if can_target {
                        let delta_x = head_pos.x - local_pos.x;
                        let delta_y = head_pos.y - local_pos.y;
                        let delta_z = head_pos.z - local_pos.z;
                        let hyp = (delta_x * delta_x + delta_y * delta_y).sqrt();

                        let target_yaw = delta_y.atan2(delta_x).to_degrees() + 90.0;
                        let target_pitch = delta_z.atan2(hyp).to_degrees();

                        let mut yaw_diff = target_yaw - current_yaw;
                        if yaw_diff > 180.0 { yaw_diff -= 360.0; }
                        if yaw_diff < -180.0 { yaw_diff += 360.0; }

                        let pitch_diff = target_pitch - current_pitch;
                        let fov_dist = (yaw_diff * yaw_diff + pitch_diff * pitch_diff).sqrt();

                        if fov_dist < closest_fov {
                            closest_fov = fov_dist;
                            let smooth_yaw = current_yaw + (yaw_diff / aimbot_smoothness);
                            let smooth_pitch = current_pitch + (pitch_diff / aimbot_smoothness);
                            best_target = Some((smooth_yaw, smooth_pitch));
                        }
                    }
                }

                // ESP Logic
                if is_enabled {
                    if let Some(head_screen_position) = util::world_to_screen(
                        head_pos,
                        view_matrix,
                        window_width,
                        window_height,
                    ) {
                        if let Some(feet_screen_position) = util::world_to_screen(
                            entity.feet_position(),
                            view_matrix,
                            window_width,
                            window_height,
                        ) {
                            let height = (feet_screen_position.y - head_screen_position.y) as i32;
                            let width = height / 2;
                            let left = head_screen_position.x as i32 - width / 2;
                            let top = head_screen_position.y as i32;

                            new_draw_rect_list.push(EspBox {
                                rect: RECT {
                                    left,
                                    right: left + width,
                                    top,
                                    bottom: top + height,
                                },
                                name: entity.name(),
                                feet_pos: POINT { x: feet_screen_position.x as i32, y: feet_screen_position.y as i32 },
                            });
                        }
                    }
                }
            }
        }

        // Apply Aimbot
        if aimbot_enabled {
            let right_click = unsafe { GetAsyncKeyState(VK_RBUTTON.0 as i32) } as u16 & 0x8000 != 0;
            if right_click {
                if let Some((yaw, pitch)) = best_target {
                    util::write_memory::<f32>(process_handle, local_player_ptr + offset::LOCAL_YAW as usize, yaw);
                    util::write_memory::<f32>(process_handle, local_player_ptr + offset::LOCAL_PITCH as usize, pitch);
                }
            }
        }

        if let Ok(mut state) = app_state.write() {
            state.box_count = new_draw_rect_list.len();
            state.debug_last_vis = debug_last_vis;
        }

        if let Ok(mut draw_rect_list) = draw_rect_list.write() {
            draw_rect_list.clear();
            draw_rect_list.extend(new_draw_rect_list);
        }

        let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
        thread::sleep(timeout);
        last_tick = Instant::now();
    }
}
