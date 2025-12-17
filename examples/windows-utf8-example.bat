@echo off
REM Windows CMD UTF-8 Setup Example
REM This script demonstrates proper UTF-8 configuration for intent-engine

echo ========================================
echo Intent-Engine Windows UTF-8 Example
echo ========================================
echo.

REM Set console to UTF-8
echo Setting console to UTF-8 (Code Page 65001)...
chcp 65001 > nul
echo Done.
echo.

REM Display current code page
echo Current Code Page:
chcp
echo.

REM Example 1: Add a task with Chinese name
echo Example 1: Adding a task with Chinese name...
intent-engine task add --name "实现用户认证系统"
echo.

REM Example 2: Add a task with Chinese spec
echo Example 2: Adding a task with Chinese spec from stdin...
echo 使用 JWT 实现认证，支持 7 天有效期和刷新令牌 | intent-engine task add --name "JWT 认证" --spec-stdin
echo.

REM Example 3: Start the task
echo Example 3: Starting the task...
intent-engine task start 1
echo.

REM Example 4: Add an event with Chinese description
echo Example 4: Adding a decision event...
echo 选择 HS256 算法，因为我们暂时不需要非对称加密 | intent-engine event add --type decision --data-stdin
echo.

REM Example 5: Search for tasks
echo Example 5: Searching for tasks containing "认证"...
intent-engine task search "认证"
echo.

REM Example 6: Generate report
echo Example 6: Generating summary report...
intent-engine report --summary-only
echo.

echo ========================================
echo All examples completed successfully!
echo ========================================
pause
