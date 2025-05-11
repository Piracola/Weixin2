use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::io::{self, Write};
use winreg::RegKey;
use winreg::enums::*;

fn pause_with_message(message: &str) {
    print!("{}", message);
    io::stdout().flush().unwrap();
    let mut _buffer = String::new();
    io::stdin().read_line(&mut _buffer).unwrap();
}

fn trim_spaces(s: &str) -> String {
    s.trim().to_string()
}

// 函数用于保存用户指定的路径到注册表
fn save_user_specified_path(path: &Path) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let launcher_subkey_path = "Software\\Weixin2Launcher";
    let value_name = "UserSpecifiedPath";

    let (key, _) = hkcu.create_subkey(launcher_subkey_path)?;
    let path_str = path.to_string_lossy().into_owned();
    key.set_value(value_name, &path_str)?;
    Ok(())
}

// 查询注册表项
// _hkey_root_name 参数当前未使用，但保留以表示其来源
fn query_registry_key(_hkey_root_name: &str, hkey: &RegKey, subkey_path: &str, value_name: &str) -> Option<PathBuf> {
    if let Ok(subkey) = hkey.open_subkey(subkey_path) {
        if let Ok(install_path_raw) = subkey.get_value::<String, _>(value_name) {
            let install_path_trimmed = trim_spaces(&install_path_raw);
            if !install_path_trimmed.is_empty() {
                let base_path = PathBuf::from(install_path_trimmed);

                // 情况1: 注册表中的路径直接指向 .exe 文件
                if base_path.is_file() && base_path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("exe")) {
                    return Some(base_path);
                } else {
                    // 情况2: 注册表中的路径是一个目录，尝试附加常见的可执行文件名
                    let exe_names_to_try = ["WeChat.exe", "weixin.exe"];
                    for exe_name in exe_names_to_try.iter() {
                        let potential_path = base_path.join(exe_name);
                        if potential_path.is_file() { // is_file() 会检查存在性和是否为文件
                            return Some(potential_path);
                        }
                    }
                }
            }
        }
    }
    None
}

fn find_wechat_path(tried_paths: &mut Vec<String>) -> Option<PathBuf> {
    // 步骤 0: 检查用户上次手动指定的路径 (从本程序注册表配置)
    let hkcu_launcher = RegKey::predef(HKEY_CURRENT_USER);
    let launcher_subkey_path = "Software\\Weixin2Launcher";
    let launcher_value_name = "UserSpecifiedPath";
    tried_paths.push(format!("  本程序配置: HKEY_CURRENT_USER\\{}\\{}", launcher_subkey_path, launcher_value_name));

    if let Ok(subkey) = hkcu_launcher.open_subkey(launcher_subkey_path) {
        if let Ok(saved_path_str) = subkey.get_value::<String, _>(launcher_value_name) {
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
    if let Some(path) = query_registry_key("HKEY_CURRENT_USER", &hkcu_tencent_weixin, weixin_subkey_path, "InstallPath") {
        return Some(path);
    }
    tried_paths.push(format!("  默认注册表: HKEY_CURRENT_USER\\{}\\Path", weixin_subkey_path)); // 也检查 "Path"
    if let Some(path) = query_registry_key("HKEY_CURRENT_USER", &hkcu_tencent_weixin, weixin_subkey_path, "Path") {
        return Some(path);
    }

    // 步骤 2: 检查常见的安装路径
    let mut common_paths_to_check: Vec<(&str, Box<dyn Fn() -> Option<PathBuf>>)> = Vec::new();

    // 旧版 WeChat 路径
    common_paths_to_check.push(("%ProgramFiles(x86)%\\Tencent\\WeChat\\WeChat.exe", Box::new(|| {
        env::var("ProgramFiles(x86)").ok().map(|p| PathBuf::from(p).join("Tencent\\WeChat\\WeChat.exe"))
    })));
    common_paths_to_check.push(("%ProgramFiles%\\Tencent\\WeChat\\WeChat.exe", Box::new(|| {
        env::var("ProgramFiles").ok().map(|p| PathBuf::from(p).join("Tencent\\WeChat\\WeChat.exe"))
    })));
    common_paths_to_check.push(("%LOCALAPPDATA%\\Programs\\Tencent\\WeChat\\WeChat.exe", Box::new(|| {
        env::var("LOCALAPPDATA").ok().map(|p| PathBuf::from(p).join("Programs\\Tencent\\WeChat\\WeChat.exe"))
    })));
    
    // 新版 Weixin (4.0+) 路径
    common_paths_to_check.push(("%ProgramFiles(x86)%\\Tencent\\Weixin\\weixin.exe", Box::new(|| {
        env::var("ProgramFiles(x86)").ok().map(|p| PathBuf::from(p).join("Tencent\\Weixin\\weixin.exe"))
    })));
    common_paths_to_check.push(("%ProgramFiles%\\Tencent\\Weixin\\weixin.exe", Box::new(|| {
        env::var("ProgramFiles").ok().map(|p| PathBuf::from(p).join("Tencent\\Weixin\\weixin.exe"))
    })));
    common_paths_to_check.push(("%LOCALAPPDATA%\\Programs\\Tencent\\Weixin\\weixin.exe", Box::new(|| {
        env::var("LOCALAPPDATA").ok().map(|p| PathBuf::from(p).join("Programs\\Tencent\\Weixin\\weixin.exe"))
    })));
    
    // 示例自定义路径 (保留用户原有的)
    common_paths_to_check.push(("D:\\Program Files\\Tencent\\WeChat\\WeChat.exe", Box::new(|| {
        Some(PathBuf::from("D:\\Program Files\\Tencent\\WeChat\\WeChat.exe"))
    })));
    common_paths_to_check.push(("D:\\Program Files\\Tencent\\Weixin\\weixin.exe", Box::new(|| {
        Some(PathBuf::from("D:\\Program Files\\Tencent\\Weixin\\weixin.exe"))
    })));

    for (description, path_fn) in common_paths_to_check {
        tried_paths.push(format!("  位置: {}", description));
        if let Some(path) = path_fn() {
            if path.is_file() {
                return Some(path);
            }
        }
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
        if let Some(path) = query_registry_key("HKEY_LOCAL_MACHINE", &hklm, subkey, value_name) {
            return Some(path);
        }
    }
    
    None
}

fn launch_wechat(wechat_exe_path: &Path) {
    match Command::new(wechat_exe_path).spawn() {
        Ok(_) => {} // 启动命令已发出，静默处理成功情况
        Err(e) => println!("    启动微信失败: {:?}", e),
    }
}

fn main() {
    let mut tried_paths: Vec<String> = Vec::new();
    let mut found_wechat_path = find_wechat_path(&mut tried_paths);

    if found_wechat_path.is_none() {
        println!("\n错误：自动查找未能找到微信安装路径。");
        println!("已尝试以下位置和注册表项:");
        for tried_path in &tried_paths {
            println!("{}", tried_path);
        }
        
        print!("\n是否要手动指定 WeChat.exe 或 weixin.exe 的路径? (y/n): ");
        io::stdout().flush().unwrap();
        let mut user_choice = String::new();
        io::stdin().read_line(&mut user_choice).unwrap();

        if user_choice.trim().eq_ignore_ascii_case("y") {
            print!("请输入 WeChat.exe 或 weixin.exe 的完整路径: ");
            io::stdout().flush().unwrap();
            let mut manual_path_str = String::new();
            io::stdin().read_line(&mut manual_path_str).unwrap();
            let manual_path_trimmed = manual_path_str.trim();

            if !manual_path_trimmed.is_empty() {
                let manual_path_buf = PathBuf::from(manual_path_trimmed);
                if manual_path_buf.is_file() {
                    if let Err(e) = save_user_specified_path(&manual_path_buf) {
                        println!("    警告: 保存用户指定路径到注册表失败: {:?}", e);
                    }
                    found_wechat_path = Some(manual_path_buf);
                } else {
                    println!("  错误: 手动输入的路径 \"{}\" 无效或文件不存在。", manual_path_buf.display());
                    pause_with_message("按任意键退出...");
                    exit(3); 
                }
            } else {
                println!("  错误: 未输入路径。");
                pause_with_message("按任意键退出...");
                exit(4); 
            }
        } else {
            println!("  用户选择不手动指定路径。");
            pause_with_message("按任意键退出...");
            exit(1);
        }
    }

    match found_wechat_path {
        Some(path) => {
            if !path.is_file(){ // 再次确认路径有效性
                println!("    警告: 找到的路径 \"{}\" 最终检测为无效! 这可能是一个错误。", path.display());
                pause_with_message("按任意键退出...");
                exit(2);
            }
            
            let launch_count = if let Ok(current_exe_path) = env::current_exe() {
                if let Some(file_stem_osstr) = current_exe_path.file_stem() {
                    if let Some(file_stem_str) = file_stem_osstr.to_str() {
                        if let Ok(num) = file_stem_str.parse::<u32>() {
                            if num >= 1 && num <= 10 {
                                num // 使用文件名数字 (1-10)
                            } else {
                                2 // 文件名数字无效 (0 或 >10)，使用默认值
                            }
                        } else {
                            2 // 文件名不是纯数字，使用默认值
                        }
                    } else {
                        2 // 无法转换文件名，使用默认值
                    }
                } else {
                    2 // 无法获取文件名，使用默认值
                }
            } else {
                2 // 无法获取当前执行路径，使用默认值
            };
            
            println!("准备启动微信 {} 次...", launch_count); // 添加一些反馈
            for i in 0..launch_count {
                println!("  正在启动微信 (第 {} 次)...", i + 1);
                launch_wechat(&path);
            }
            println!("所有微信启动命令已发出。程序将自动退出。"); // 修改提示信息
        }
        None => { // 此处理论上不会到达，因为前面已处理 None 的情况并退出
            println!("  错误：最终未能找到微信安装路径。");
            pause_with_message("按任意键退出...");
            exit(5);
        }
    }
    
    // pause_with_message("操作完成。按任意键退出脚本."); // 将此行注释掉或删除
}