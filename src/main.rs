#![windows_subsystem = "windows"]

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use winreg::RegKey;
use winreg::enums::*;
use native_dialog::{MessageDialog, FileDialog};

fn show_alert(title: &str, text: &str) {
    MessageDialog::new()
        .set_title(title)
        .set_text(text)
        .show_alert()
        .unwrap_or_else(|_| ());
}

fn trim_spaces(s: &str) -> String {
    s.trim().to_string()
}

// 函数用于保存用户指定的路径到注册表
const LAUNCHER_SUBKEY: &str = "Software\\QuickLauncher\\Weixin";
const USER_SPECIFIED_PATH_VALUE: &str = "UserSpecifiedPath";

fn save_user_specified_path(path: &Path) -> Result<(), std::io::Error> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let (key, _) = hkcu.create_subkey(LAUNCHER_SUBKEY)?;
    let path_str = path.to_string_lossy().into_owned();
    key.set_value(USER_SPECIFIED_PATH_VALUE, &path_str)?;
    Ok(())
}

// 查询注册表项
// _hkey_root_name 参数当前未使用，但保留以表示其来源
fn query_registry_key(hkey: &RegKey, subkey_path: &str, value_name: &str) -> Option<PathBuf> {
    let subkey = hkey.open_subkey(subkey_path).ok()?;
    let install_path_raw = subkey.get_value::<String, _>(value_name).ok()?;
    let install_path_trimmed = trim_spaces(&install_path_raw);
    if install_path_trimmed.is_empty() {
        return None;
    }

    let base_path = PathBuf::from(install_path_trimmed);
    
    // 情况1: 注册表中的路径直接指向 .exe 文件
    if base_path.is_file() && base_path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("exe")) {
        return Some(base_path);
    }
    
    // 情况2: 注册表中的路径是一个目录，尝试附加常见的可执行文件名
    let exe_names_to_try = ["WeChat.exe", "weixin.exe"];
    exe_names_to_try.iter()
        .map(|exe_name| base_path.join(exe_name))
        .find(|path| path.is_file())
}

fn find_wechat_path(tried_paths: &mut Vec<String>) -> Option<PathBuf> {
    // 步骤 0: 检查用户上次手动指定的路径 (从本程序注册表配置)
    let hkcu_launcher = RegKey::predef(HKEY_CURRENT_USER);
    tried_paths.push(format!("  本程序配置: HKEY_CURRENT_USER\\{}\\{}", LAUNCHER_SUBKEY, USER_SPECIFIED_PATH_VALUE));

    if let Ok(subkey) = hkcu_launcher.open_subkey(LAUNCHER_SUBKEY) {
        if let Ok(saved_path_str) = subkey.get_value::<String, _>(USER_SPECIFIED_PATH_VALUE) {
            let saved_path_trimmed = trim_spaces(&saved_path_str);
            if !saved_path_trimmed.is_empty() {
                let wechat_path = PathBuf::from(saved_path_trimmed);
                if wechat_path.is_file() {
                    return Some(wechat_path);
                }
            }
        }
    }

    // 步骤 1: 检查微信官方的当前用户注册表位置
    let hkcu_tencent_weixin = RegKey::predef(HKEY_CURRENT_USER);
    let weixin_subkey_path = "Software\\Tencent\\Weixin";
    
    tried_paths.push(format!("  默认注册表: HKEY_CURRENT_USER\\{}\\InstallPath", weixin_subkey_path));
    if let Some(path) = query_registry_key(&hkcu_tencent_weixin, weixin_subkey_path, "InstallPath") {
        return Some(path);
    }
    tried_paths.push(format!("  默认注册表: HKEY_CURRENT_USER\\{}\\Path", weixin_subkey_path)); // 也检查 "Path"
    if let Some(path) = query_registry_key(&hkcu_tencent_weixin, weixin_subkey_path, "Path") {
        return Some(path);
    }

    // 步骤 3: 尝试从其他微信官方的本地计算机注册表查询微信路径
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let hklm_paths_to_check = [
        ("SOFTWARE\\Tencent\\WeChat", "InstallPath"),
        ("SOFTWARE\\WOW6432Node\\Tencent\\WeChat", "InstallPath"), // 针对64位系统上的32位微信
        ("SOFTWARE\\Tencent\\Weixin", "InstallPath"),
        ("SOFTWARE\\WOW6432Node\\Tencent\\Weixin", "InstallPath"),
    ];

    for (subkey, value_name) in hklm_paths_to_check.iter() {
        tried_paths.push(format!("  注册表: HKEY_LOCAL_MACHINE\\{}\\{}", subkey, value_name));
        if let Some(path) = query_registry_key(&hklm, subkey, value_name) {
            return Some(path);
        }
    }
    
    None
}

use std::os::windows::process::CommandExt;
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn launch_wechat(wechat_exe_path: &Path) {
    match Command::new(wechat_exe_path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn() {
        Ok(_) => {}
        Err(e) => {
            show_alert("错误", &format!("启动微信失败: {:?}", e));
        }
    }
}

fn main() {
    let mut tried_paths: Vec<String> = Vec::new();
    let mut found_wechat_path = find_wechat_path(&mut tried_paths);

    if found_wechat_path.is_none() {
        let mut error_message = "错误：自动查找未能找到微信安装路径。\n已尝试以下位置和注册表项:\n".to_string();
        for tried_path in &tried_paths {
            error_message.push_str(&format!("{}\n", tried_path));
        }
        
        let choice = MessageDialog::new()
            .set_title("未找到微信路径")
            .set_text(&error_message)
            .show_confirm()
            .unwrap_or(false);
            
        if choice {
            if let Some(path) = FileDialog::new()
                .set_title("选择微信可执行文件")
                .add_filter("可执行文件", &["exe"])
                .show_open_single_file()
                .unwrap_or(None) 
            {
                if path.is_file() {
                    if let Err(e) = save_user_specified_path(&path) {
                        show_alert("警告", &format!("保存用户指定路径到注册表失败: {:?}", e));
                    }
                    found_wechat_path = Some(path);
                } else {
                    show_alert("错误", &format!("路径 \"{}\" 无效或文件不存在", path.display()));
                    exit(3);
                }
            } else {
                show_alert("错误", "未选择路径");
                exit(4);
            }
        } else {
            exit(1);
        }
    }

    match found_wechat_path {
        Some(path) => {
            if !path.is_file(){ // 再次确认路径有效性
            show_alert("警告", &format!("找到的路径 \"{}\" 最终检测为无效! 这可能是一个错误。", path.display()));
                exit(2);
            }
            
            let launch_count = env::current_exe()
                .ok()
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
                .and_then(|s| s.parse::<u32>().ok())
                .filter(|&n| n >= 1 && n <= 10)
                .unwrap_or(2);
            
            for _i in 0..launch_count {
                launch_wechat(&path);
            }
        },
        None => exit(5)
    }
}