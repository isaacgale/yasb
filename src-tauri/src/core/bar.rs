use anyhow::{Result, Context};
use tauri::{PhysicalSize, PhysicalPosition, Manager, AppHandle};
use windows::Win32::UI::WindowsAndMessaging::{GWL_STYLE, WS_POPUP, SetWindowLongW};
use super::configuration::{BarConfig, BarEdge, YasbConfig, validate_bar_label};
use super::constants::{DEFAULT_BAR_EDGE, DEFAULT_BAR_THICKNESS, FRONTEND_INDEX, FRONTEND_SETUP};
use crate::win32::app_bar;

pub fn create_bars_from_config(app_handle: &AppHandle, config: YasbConfig) -> () {
  // Close any existing windows
  for (_, window) in app_handle.windows() {
    let _ = window.close();
  }

  // Create bar windows from config
  for (label, config) in config.clone().bars {
    validate_bar_label(&label.as_str());

    if let Err(e) = create_bars(app_handle, &label, &config) {
      log::error!("Failed to create bar(s) for bar config '{}': {:#?}", label, e);
      app_handle.exit(1);
    }
  }
}

fn create_window(app_handle: &AppHandle, label: String, url: &str) -> Result<tauri::Window> {
  let window_builder = tauri::WindowBuilder::new(
    app_handle,
    label.clone(),
    tauri::WindowUrl::App(url.into())
  ).min_inner_size(10.0, 10.0).visible(false).transparent(true);

  window_builder.build().context(format!("Failed to build window for bar '{}'", label))
}

fn create_bar(app_handle: &AppHandle, bar_index: usize, monitor: &tauri::Monitor, bar_label: &String, bar_config: &BarConfig) -> Result<tauri::Window> {
  let label = format!("{}_{}", bar_label, bar_index+1);
  let window = create_window(app_handle, label.clone(), FRONTEND_INDEX)?;
  let bar_thickness = bar_config.thickness.unwrap_or(DEFAULT_BAR_THICKNESS);
  let bar_edge = bar_config.edge.clone().unwrap_or(DEFAULT_BAR_EDGE);

  // Default bar size and position is for top edge
  let mut bar_position = PhysicalPosition::new(monitor.position().x, monitor.position().y);
  let mut bar_size = PhysicalSize::new(monitor.size().width, bar_thickness);

  // Change bar size and position based on edge provided in bar_config
  match bar_edge {
    BarEdge::Bottom => {
      bar_position.y = monitor.position().y + monitor.size().height as i32 - bar_thickness as i32;
    },
    BarEdge::Left => {
      bar_size.width = bar_thickness;
      bar_size.height = monitor.size().height;
    },
    BarEdge::Right => {
      bar_position.x = monitor.position().x + monitor.size().width as i32 - bar_thickness as i32;
      bar_size.width = bar_thickness;
      bar_size.height = monitor.size().height;
    },
    _ => {}
  }
  
  let monitor_name = monitor.name().context(format!("Monitor for bar '{}' has NO NAME.", label));
  
  window.hide()?;
  window.set_decorations(false)?;
  window.set_size(bar_size)?;
  window.set_skip_taskbar(true)?;
  window.set_resizable(false)?;

  // Minimum window height fix
  let hwnd = window.hwnd().unwrap().clone();
  unsafe {
    SetWindowLongW(hwnd, 
      GWL_STYLE, 
      WS_POPUP.0 as i32
    );
  }

  window.set_position(bar_position)?;

  log::info!(
    "Created {} on {} at {},{}",
    label,
    monitor_name?,
    bar_position.x,
    bar_position.y
  );

  Ok(window)
}

fn create_bars(app_handle: &AppHandle, bar_label: &String, bar_config: &BarConfig) -> Result<Vec<tauri::Window>> {
  let mut bars: Vec<tauri::Window> = Vec::new();
  let setup_window = create_window(app_handle, "setup_window".to_string(), FRONTEND_SETUP).unwrap();

  // Create bars for screens defined in bar_config.screens
  // If no screens are provided in config, create bar on all available screens
  for (idx, monitor) in setup_window.available_monitors()?.iter().enumerate() {
    if let Some(ref screen_names) = bar_config.screens.clone() {
      if screen_names.is_empty() {
        bars.push(create_bar(app_handle, idx, &monitor, &bar_label, &bar_config)?);
      } else {
        for screen_name in screen_names {
          if screen_name == monitor.name().unwrap_or(&"".to_string()) {
            bars.push(create_bar(app_handle, idx, &monitor, &bar_label, &bar_config)?);
          }
        }
      }
    } else {
      bars.push(create_bar(app_handle, idx, &monitor, &bar_label, &bar_config)?);
    }
  }

  setup_window.close()?;

  Ok(bars)
}

pub fn register_win32_app_bar(bar_window: tauri::Window, bar_label: &str, bar_config: &BarConfig) -> () {
  app_bar::ab_register_and_position(&bar_window, bar_config.edge.clone()).unwrap();

  // Unregister Win32 App Bar if KeyboardInterrupt is detected
  ctrlc::set_handler(move || {
    app_bar::ab_remove(&bar_window).unwrap();
    let _ = bar_window.close().unwrap();
  }).expect(format!("Failed to set ctrlc handler for bar '{}'", bar_label).as_str());
}