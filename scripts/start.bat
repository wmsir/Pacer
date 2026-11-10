@echo off
chcp 65001 >nul
title 稳骥 PacerSQL (wmDB) 本地一键启动控制台
echo ============================================================
echo        稳骥 PacerSQL (wmDB) 本地开发服务一键启动脚本
echo ============================================================
echo.
set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..") do set "ROOT_DIR=%%~fI"

:: 1. 检测与配置 JAVA_HOME
if not defined JAVA_HOME if exist "C:\Program Files\Java\jdk-17" (
    set "JAVA_HOME=C:\Program Files\Java\jdk-17"
    set "PATH=%JAVA_HOME%\bin;%PATH%"
)

:: 2. 检测 Maven 环境
set MVN_CMD=
where mvn >nul 2>nul
if %ERRORLEVEL% EQU 0 set MVN_CMD=mvn
if not defined MVN_CMD if defined MAVEN_HOME if exist "%MAVEN_HOME%\bin\mvn.cmd" set MVN_CMD="%MAVEN_HOME%\bin\mvn.cmd"
if not defined MVN_CMD (
    echo [错误] 未检测到 Maven 环境！请配置 MAVEN_HOME 或将 mvn 加入 PATH 环境变量。
    pause
    exit /b 1
)

:: 3. 检测 Node.js / NPM 环境
set NPM_CMD=
where npm >nul 2>nul
if %ERRORLEVEL% EQU 0 set NPM_CMD=npm
if not defined NPM_CMD if exist "C:\Program Files\nodejs\npm.cmd" set NPM_CMD="C:\Program Files\nodejs\npm.cmd"
if not defined NPM_CMD (
    echo [错误] 未检测到 Node.js/NPM 环境！
    pause
    exit /b 1
)

echo [环境就绪] 项目根目录: %ROOT_DIR%
echo [环境就绪] Maven:      %MVN_CMD%
echo [环境就绪] NPM:        %NPM_CMD%
echo.

:: 4. 启动后端服务
echo [1/2] 正在启动后端服务 (Spring Boot 端口 8088)...
start "PacerSQL-Backend-8088" cmd /k "chcp 65001 >nul && cd /d "%ROOT_DIR%\backend" && echo 后端服务启动与编译中... && %MVN_CMD% spring-boot:run"

echo 正在侦听等待后端服务启动就绪 (检测 8088 端口)...
powershell -NoProfile -Command ^
  "$maxRetries = 60; $count = 0;" ^
  "Write-Host -NoNewline '后端服务启动中 ';" ^
  "while ($count -lt $maxRetries) {" ^
  "  try {" ^
  "    $tcp = New-Object System.Net.Sockets.TcpClient;" ^
  "    $tcp.Connect('127.0.0.1', 8088);" ^
  "    if ($tcp.Connected) {" ^
  "      $tcp.Close();" ^
  "      Write-Host '';" ^
  "      Write-Host '[后端启动成功] Spring Boot 8088 端口已就绪！' -ForegroundColor Green;" ^
  "      exit 0;" ^
  "    }" ^
  "  } catch {}" ^
  "  Write-Host -NoNewline '.';" ^
  "  Start-Sleep -Seconds 2;" ^
  "  $count++;" ^
  "};" ^
  "Write-Host '';" ^
  "Write-Host '[提示] 等待超时，可能正在首次编译或下载依赖，将继续启动前端。' -ForegroundColor Yellow;" ^
  "exit 1;"

echo.
:: 5. 启动前端开发服务
echo [2/2] 正在启动前端开发服务 (Vue 3 端口 5173)...
start "PacerSQL-Frontend-5173" cmd /k "chcp 65001 >nul && cd /d "%ROOT_DIR%\frontend" && echo 前端服务启动中... && %NPM_CMD% run dev"

echo.
echo ============================================================
echo [启动完成] 稳骥 PacerSQL 前后端服务已在独立窗口中启动！
echo.
echo   * Web 控制台入口:   http://localhost:5173
echo   * Swagger 接口文档: http://localhost:8088/swagger-ui/index.html
echo.
echo   * 默认测试账号:     testadmin1 / 123456
echo   * 手机快捷登录:     13800000001 / 123456
echo ============================================================
echo.
timeout /t 3 >nul
start http://localhost:5173
