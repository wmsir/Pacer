@echo off
chcp 65001 >nul
title 稳骥 PacerSQL (wmDB) 服务停止工具
echo ============================================================
echo          稳骥 PacerSQL (wmDB) 服务一键停止脚本
echo ============================================================
echo.
echo 正在停止后端 Spring Boot 服务 (端口 8088 / 8080)...
for %%p in (8088 8080) do (
    for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":%%p " ^| findstr "LISTENING"') do (
        taskkill /f /pid %%a >nul 2>nul
        echo   已终止端口 %%p 进程 PID: %%a
    )
)
echo 正在停止前端 Vite 开发服务 (端口 5173)...
for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":5173 " ^| findstr "LISTENING"') do (
    taskkill /f /pid %%a >nul 2>nul
    echo   已终止端口 5173 进程 PID: %%a
)
echo.
echo ============================================================
echo 稳骥 PacerSQL 前后端开发服务已全部停止！
echo ============================================================
pause
