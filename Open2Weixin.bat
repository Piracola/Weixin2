@echo off
:: 设置控制台代码页为简体中文GBK
chcp 936 > nul
setlocal enabledelayedexpansion
echo 正在查找微信安装路径...
:: 定义可能的微信安装路径
set "FOUND_WECHAT="
set "SCRIPT_FILE=%~f0"
set "TEMP_FILE=%TEMP%\temp_wechat_script.bat"
:: 检查是否使用自更新参数
if "%~1"=="/update" (
echo 正在更新脚本...
call :UPDATE_SCRIPT
goto :END
)
:: 检查常见安装位置
if exist "%ProgramFiles(x86)%\Tencent\WeChat\WeChat.exe" (
set "FOUND_WECHAT=%ProgramFiles(x86)%\Tencent\WeChat\WeChat.exe"
) else if exist "%ProgramFiles%\Tencent\WeChat\WeChat.exe" (
set "FOUND_WECHAT=%ProgramFiles%\Tencent\WeChat\WeChat.exe"
) else if exist "%LOCALAPPDATA%\Programs\Tencent\WeChat\WeChat.exe" (
set "FOUND_WECHAT=%LOCALAPPDATA%\Programs\Tencent\WeChat\WeChat.exe"
) else if exist "D:\Tools\Weixin\Weixin.exe" (
set "FOUND_WECHAT=D:\Tools\Weixin\Weixin.exe"
)
:: 如果未找到，尝试从注册表查询
if "D:\Tools\Weixin\Weixin.exe"=="" (
echo 尝试从注册表查询微信路径...
for /f "tokens=2*" %%a in ('reg query "HKLM\SOFTWARE\Tencent\WeChat" /v InstallPath 2^>nul') do (
set "REG_PATH=%%b\WeChat.exe"
if exist "" (
set "FOUND_WECHAT="
)
)
)
:: 检查是否找到微信
if "D:\Tools\Weixin\Weixin.exe"=="" (
echo 错误：未找到微信安装路径
goto :END
) else (
echo 找到微信: D:\Tools\Weixin\Weixin.exe
)
:: 启动微信实例
echo 正在启动微信...
start "" "D:\Tools\Weixin\Weixin.exe"
start "" "D:\Tools\Weixin\Weixin.exe"
:: 询问是否更新脚本
echo.
echo 是否要更新脚本中的微信路径? (Y/N)
set /p UPDATE_CHOICE=
if /i "y"=="Y" (
call :UPDATE_SCRIPT
)
goto :END
:UPDATE_SCRIPT
(
for /f "tokens=*" %%a in ('type "I:\JBCode\BAT\Open2Weixin.bat"') do (
set "line=%%a"
if "set "line=%%a"" neq "set "line=%%a"" (
SET WECHAT_PATH="D:\Tools\Weixin\Weixin.exe"
) else (
echo ) else (
)
)
) > "C:\Users\17719\AppData\Local\Temp\temp_wechat_script.bat"
move /y "C:\Users\17719\AppData\Local\Temp\temp_wechat_script.bat" "I:\JBCode\BAT\Open2Weixin.bat" > nul
echo 脚本已更新
goto :eof
:END
echo 脚本执行完毕
endlocal
