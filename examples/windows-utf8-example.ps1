# PowerShell UTF-8 Setup Example
# This script demonstrates proper UTF-8 configuration for intent-engine

Write-Host "========================================"
Write-Host "Intent-Engine Windows UTF-8 Example"
Write-Host "========================================"
Write-Host ""

# Set console encoding to UTF-8
Write-Host "Setting console encoding to UTF-8..."
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding = [System.Text.Encoding]::UTF8
Write-Host "Done."
Write-Host ""

# Display current encoding
Write-Host "Current Output Encoding: $([Console]::OutputEncoding.EncodingName)"
Write-Host "Current Input Encoding: $([Console]::InputEncoding.EncodingName)"
Write-Host ""

# Example 1: Add a task with Chinese name
Write-Host "Example 1: Adding a task with Chinese name..."
intent-engine task add --name "实现用户认证系统"
Write-Host ""

# Example 2: Add a task with Chinese spec
Write-Host "Example 2: Adding a task with Chinese spec from stdin..."
"使用 JWT 实现认证，支持 7 天有效期和刷新令牌" | intent-engine task add --name "JWT 认证" --spec-stdin
Write-Host ""

# Example 3: Start the task
Write-Host "Example 3: Starting the task..."
intent-engine task start 1
Write-Host ""

# Example 4: Add an event with Chinese description
Write-Host "Example 4: Adding a decision event..."
"选择 HS256 算法，因为我们暂时不需要非对称加密" | intent-engine event add --type decision --data-stdin
Write-Host ""

# Example 5: Search for tasks
Write-Host "Example 5: Searching for tasks containing '认证'..."
intent-engine task search "认证"
Write-Host ""

# Example 6: Generate report
Write-Host "Example 6: Generating summary report..."
intent-engine report --summary-only
Write-Host ""

# Example 7: Mixed languages
Write-Host "Example 7: Mixed Chinese and English..."
intent-engine task add --name "Implement 用户权限管理 with RBAC"
Write-Host ""

# Example 8: Special Chinese punctuation
Write-Host "Example 8: Special Chinese punctuation..."
intent-engine task add --name "测试：特殊字符「引号」【括号】"
Write-Host ""

Write-Host "========================================"
Write-Host "All examples completed successfully!"
Write-Host "========================================"
Write-Host ""
Write-Host "Tip: To make UTF-8 permanent, add the encoding settings to your PowerShell profile:"
Write-Host "  notepad `$PROFILE"
Write-Host "  Add these lines:"
Write-Host "    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8"
Write-Host "    [Console]::InputEncoding = [System.Text.Encoding]::UTF8"
Write-Host ""
