#![windows_subsystem = "windows"]

use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use winreg::RegKey;
use winreg::enums::*;
use native_dialog::{FileDialog, MessageDialog};
use winapi::um::winuser::{SetProcessDPIAware, SW_HIDE};

const REGISTRY_SUBKEY: &str = "Software\\QuickLauncher";
const REGISTRY_VALUE: &str = "ShortcutDir";

fn save_shortcut_dir(path: &Path) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(REGISTRY_SUBKEY)?;
    let path_str = path.to_string_lossy().into_owned();
    key.set_value(REGISTRY_VALUE, &path_str)?;
    Ok(())
}

fn get_shortcut_dir() -> io::Result<Option<PathBuf>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    
    match hkcu.open_subkey(REGISTRY_SUBKEY) {
        Ok(subkey) => {
            match subkey.get_value::<String, _>(REGISTRY_VALUE) {
                Ok(path_str) => {
                    let path = PathBuf::from(path_str.trim());
                    if path.is_dir() {
                        Ok(Some(path))
                    } else {
                        Ok(None)
                    }
                },
                Err(_) => Ok(None)
            }
        },
        Err(_) => Ok(None)
    }
}

fn launch_shortcuts(dir: &Path) -> io::Result<()> {
    let entries = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.is_file() && path.extension().map_or(false, |ext| ext == "lnk")
        })
        .collect::<Vec<_>>();

    let mut handles = Vec::new();
    for entry in entries {
        let path = entry.path();
        handles.push(std::thread::spawn(move || {
            unsafe {
                use winapi::um::shellapi::ShellExecuteW;
                use std::ffi::OsStr;
                use std::os::windows::ffi::OsStrExt;

                let path_wide: Vec<u16> = OsStr::new(path.as_os_str()).encode_wide().chain(Some(0)).collect();
                let operation_wide: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();

                let result = ShellExecuteW(
                    std::ptr::null_mut(),
                    operation_wide.as_ptr(),
                    path_wide.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    SW_HIDE
                );

                if (result as usize) <= 32 {
                    let error_msg = match result as usize {
                        0 => "系统内存或资源不足",
                        2 => "文件未找到",
                        3 => "路径未找到",
                        5 => "拒绝访问",
                        8 => "内存不足",
                        11 => "无效格式",
                        26 => "共享冲突",
                        27 => "文件名关联不完整或无效",
                        28 => "DDE事务超时",
                        29 => "DDE事务失败",
                        30 => "DDE正忙",
                        31 => "DDE无响应",
                        32 => "DDE建议超时",
                        _ => "未知错误"
                    };
                    MessageDialog::new()
                        .set_title("错误")
                        .set_text(&format!("启动快捷方式失败: {}\n文件: {:?}\n错误代码: {}", error_msg, path, result as usize))
                        .show_alert()
                        .unwrap_or_else(|_| ());
                }
            }
        }));
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap_or_else(|_| ());
    }

    Ok(())
}

fn prompt_for_directory() -> Option<PathBuf> {
    // 先显示确认对话框
    let confirmed = match MessageDialog::new()
        .set_title("选择快捷方式目录")
        .set_text("请选择包含快捷方式的目录")
        .show_confirm() {
            Ok(c) => c,
            Err(e) => {
                MessageDialog::new()
    .set_title("错误")
    .set_text(&format!("显示对话框失败: {}", e))
    .show_alert()
    .unwrap_or_else(|_| ());
                return None;
            }
        };

    if !confirmed {
        return None;
    }

    // 确认后显示目录选择对话框
    match FileDialog::new()
        .set_title("选择快捷方式目录")
        .show_open_single_dir() {
            Ok(Some(path)) => Some(path),
            Ok(None) => None,
            Err(e) => {
                MessageDialog::new()
    .set_title("错误")
    .set_text(&format!("选择目录失败: {}", e))
    .show_alert()
    .unwrap_or_else(|_| ()); // 静默处理对话框本身的显示错误
                None
            }
        }
}

fn main() {
    unsafe {
        SetProcessDPIAware();
    }
    match get_shortcut_dir() {
        Ok(Some(dir)) => {
            if let Err(e) = launch_shortcuts(&dir) {
                MessageDialog::new()
    .set_title("错误")
    .set_text(&format!("启动快捷方式时出错: {}", e))
    .show_alert()
    .unwrap_or_else(|_| ());
            }
            return;
        },
        Err(e) => {
        MessageDialog::new()
            .set_title("错误")
            .set_text(&format!("读取注册表时出错: {}", e))
            .show_alert()
            .unwrap_or_else(|_| ()); // 静默处理对话框本身的显示错误
    },
        _ => ()
    }

    MessageDialog::new()
    .set_title("提示")
    .set_text("首次运行，请选择快捷方式目录")
    .show_alert()
    .unwrap_or_else(|_| ()); // 静默处理对话框本身的显示错误
    
    if let Some(dir_path) = prompt_for_directory() {
        match save_shortcut_dir(dir_path.as_path()) {
            Ok(_) => {
                if let Err(e) = launch_shortcuts(&dir_path) {
                    MessageDialog::new()
    .set_title("错误")
    .set_text(&format!("启动快捷方式时出错: {}", e))
    .show_alert()
    .unwrap_or_else(|_| ());
                }
            },
            Err(e) => {
                MessageDialog::new()
                    .set_title("错误")
                    .set_text(&format!("保存目录到注册表失败: {}", e))
                    .show_alert()
                    .unwrap_or_else(|_| ()); // 静默处理对话框本身的显示错误
            }
        }
    } else {
        MessageDialog::new()
    .set_title("提示")
    .set_text("未选择目录，程序退出")
    .show_alert()
    .unwrap_or_else(|_| ()); // 静默处理对话框本身的显示错误
    }
}