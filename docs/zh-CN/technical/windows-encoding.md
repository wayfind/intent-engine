# Windows 命令行中文编码问题分析与解决方案

## ✅ 重要更新：已自动修复

**自 v0.1.13 起，intent-engine 已自动处理 Windows 编码问题！**

程序启动时会自动：
- ✅ 设置控制台输入编码为 UTF-8 (`SetConsoleCP(65001)`)
- ✅ 设置控制台输出编码为 UTF-8 (`SetConsoleOutputCP(65001)`)
- ✅ 启用虚拟终端处理（支持 ANSI 颜色）

**这意味着**：
- 🎯 **无需手动配置**：直接使用即可
- 🎯 **管道传输正常**：`echo "中文" | intent-engine ...` 正常工作
- 🎯 **输出正确显示**：JSON 中的中文正确显示

**如果仍然遇到乱码**，请参考下面的疑难解答部分。

## 问题背景

在 Windows 的 cmd 和 PowerShell 中使用 intent-engine 时，可能会遇到中文字符输入和显示的问题。本文档详细分析了问题根源和多种解决方案。

## 问题根源

### 1. Windows 控制台编码机制

**cmd.exe**:
- 默认使用系统活动代码页（Active Code Page）
- 中文 Windows 通常是 **CP936 (GBK)**
- 可通过 `chcp` 命令查看和修改：`chcp` 显示当前代码页

**PowerShell 5.x**:
- 默认继承系统代码页（通常是 GBK）
- 输入输出编码可能不一致

**PowerShell 7+**:
- 默认使用 **UTF-8** 编码
- 对现代应用更友好

### 2. Rust 程序的编码特性

- **内部字符串**: Rust 的 `String` 和 `str` 始终是 UTF-8 编码
- **标准输出**: `println!` 宏输出 UTF-8 字节流到 stdout
- **标准输入**: `std::io::stdin()` 读取字节流，需要正确解码为 UTF-8

### 3. 编码不匹配导致的问题

#### 场景 1: 输出乱码
```bash
# Windows cmd (CP936) 下运行
intent-engine task add --name "测试任务"
# 输出的 JSON 中中文显示为 ��� 或 ???
```

**原因**: Rust 输出 UTF-8，cmd 按 GBK 解析，导致乱码。

#### 场景 2: 输入乱码
```bash
# 从 stdin 读取包含中文的数据
echo "这是中文规格说明" | intent-engine task add --name "任务" --spec-stdin
# 数据库中存储的是乱码
```

**原因**: cmd 通过管道传递 GBK 编码，Rust 按 UTF-8 解析失败。

#### 场景 3: JSON 解析失败
```bash
# 输入包含无效 UTF-8 序列
echo 无效字节 | intent-engine event add --type decision --data-stdin
# 报错: InvalidInput: Invalid UTF-8 in input
```

## 解决方案

### 方案 A: 用户侧配置（最简单）

#### A1. 临时设置（推荐用于测试）

**cmd.exe**:
```cmd
REM 切换到 UTF-8 (代码页 65001)
chcp 65001

REM 使用 intent-engine
intent-engine task add --name "测试任务"
```

**PowerShell 5.x**:
```powershell
# 设置输出编码为 UTF-8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding = [System.Text.Encoding]::UTF8

# 使用 intent-engine
intent-engine task add --name "测试任务"
```

**PowerShell 7+**:
```powershell
# 默认已是 UTF-8，无需配置
intent-engine task add --name "测试任务"
```

#### A2. 永久配置（推荐日常使用）

**PowerShell Profile**:
```powershell
# 编辑 Profile 文件
notepad $PROFILE

# 添加以下内容:
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding = [System.Text.Encoding]::UTF8
```

**cmd 批处理脚本**:
```cmd
@echo off
chcp 65001 > nul
intent-engine %*
```

保存为 `intent-engine-utf8.bat`，使用时调用这个脚本。

#### A3. Windows Terminal 配置

Windows Terminal 默认使用 UTF-8，体验最佳：

```json
// settings.json
{
  "profiles": {
    "defaults": {
      "fontFace": "Consolas",
      "fontSize": 10
    },
    "list": [
      {
        "name": "PowerShell",
        "commandline": "pwsh.exe -NoExit -Command \"[Console]::OutputEncoding=[System.Text.Encoding]::UTF8\""
      }
    ]
  }
}
```

### 方案 B: 应用层自动处理（最佳用户体验）

在 Rust 代码中自动检测和处理 Windows 控制台编码。

#### B1. 依赖库选择

推荐使用 `windows` crate:

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = ["Win32_System_Console"] }
```

#### B2. 实现控制台 UTF-8 初始化

创建 `src/windows_console.rs`:

```rust
#[cfg(windows)]
pub fn setup_windows_console() -> Result<(), Box<dyn std::error::Error>> {
    use windows::Win32::System::Console::{
        GetConsoleMode, SetConsoleMode, SetConsoleOutputCP,
        GetStdHandle, STD_OUTPUT_HANDLE,
        ENABLE_VIRTUAL_TERMINAL_PROCESSING,
    };

    unsafe {
        // 设置输出代码页为 UTF-8 (65001)
        SetConsoleOutputCP(65001);

        // 获取标准输出句柄
        let handle = GetStdHandle(STD_OUTPUT_HANDLE)?;

        // 启用虚拟终端处理（支持 ANSI 转义序列）
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode).is_ok() {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING.0)?;
        }
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn setup_windows_console() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
```

#### B3. 在 main 函数中调用

修改 `src/main.rs`:

```rust
#[tokio::main]
async fn main() {
    // Windows 控制台 UTF-8 设置
    #[cfg(windows)]
    if let Err(e) = windows_console::setup_windows_console() {
        eprintln!("Warning: Failed to setup Windows console UTF-8: {}", e);
    }

    if let Err(e) = run().await {
        let error_response = e.to_error_response();
        eprintln!("{}", serde_json::to_string_pretty(&error_response).unwrap());
        std::process::exit(1);
    }
}
```

#### B4. 健壮的 stdin 读取

改进 `read_stdin()` 函数以处理编码错误:

```rust
fn read_stdin() -> Result<String> {
    use std::io::Read;

    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer)?;

    // 尝试 UTF-8 解码
    match String::from_utf8(buffer.clone()) {
        Ok(s) => Ok(s.trim().to_string()),
        Err(_) => {
            // UTF-8 解码失败，尝试 GBK (仅 Windows)
            #[cfg(windows)]
            {
                use encoding_rs::GBK;
                let (decoded, _, had_errors) = GBK.decode(&buffer);
                if had_errors {
                    return Err(IntentError::InvalidInput(
                        "Input contains invalid characters. Please ensure your terminal is set to UTF-8 (run 'chcp 65001' in cmd).".to_string()
                    ));
                }
                Ok(decoded.trim().to_string())
            }

            #[cfg(not(windows))]
            {
                Err(IntentError::InvalidInput(
                    "Invalid UTF-8 in input".to_string()
                ))
            }
        }
    }
}
```

需要添加依赖:
```toml
[dependencies]
encoding_rs = "0.8"  # GBK 解码支持
```

### 方案 C: 文档引导（补充方案）

在安装文档中添加 Windows 用户专属说明。

#### 更新 `docs/zh-CN/guide/installation.md`

添加以下章节:

```markdown
## Windows 用户必读：中文编码配置

### 问题症状

如果你在使用 intent-engine 时看到：
- 输出的中文显示为乱码（如 `���` 或 `???`）
- 错误信息提示 "Invalid UTF-8 in input"

这是因为 Windows 默认使用 GBK 编码，而 intent-engine 使用 UTF-8。

### 快速解决方案

#### 方案 1: 使用 Windows Terminal（推荐）

Windows Terminal 默认 UTF-8，无需配置。

下载: https://aka.ms/terminal

#### 方案 2: 在 cmd 中使用

每次使用前运行:
```cmd
chcp 65001
```

#### 方案 3: 在 PowerShell 中使用

在 PowerShell Profile 中添加:
```powershell
notepad $PROFILE

# 添加以下行:
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
```

### 验证设置

运行以下命令测试:
```bash
intent-engine task add --name "测试中文" --spec-stdin
```

输入:
```
这是一个中文测试
```

如果输出的 JSON 中能正确显示中文，说明配置成功。
```

## 推荐方案对比

| 方案 | 优点 | 缺点 | 适用场景 |
|------|------|------|----------|
| **A. 用户配置** | 简单，无需修改代码 | 需要用户手动设置，易忘记 | 快速测试，临时使用 |
| **B. 自动处理** | 零配置，用户体验好 | 需要额外依赖，增加复杂度 | 生产环境，长期维护 |
| **C. 文档引导** | 教育用户，治本 | 仍需用户操作 | 配合 A/B 方案使用 |

## 实施建议

### 短期（v0.1.x）
1. ✅ **添加文档说明**（方案 C）
2. ✅ **在错误信息中提示**用户检查编码设置

### 中期（v0.2.x）
1. 🔄 **实现自动检测和提示**
   - 检测到 Windows + 非 UTF-8 时，输出警告
   - 提供 `--force-utf8` 标志手动启用

### 长期（v0.3.x+）
1. 🚀 **完全自动化**（方案 B）
   - 自动设置控制台 UTF-8
   - 自动检测输入编码并转换

## 测试建议

### 测试用例

```rust
#[cfg(target_os = "windows")]
#[test]
fn test_chinese_input_output() {
    // 测试中文任务名
    let output = Command::new(env!("CARGO_BIN_EXE_intent-engine"))
        .args(["task", "add", "--name", "测试任务"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("测试任务"));
}

#[cfg(target_os = "windows")]
#[test]
fn test_chinese_stdin() {
    // 测试从 stdin 读取中文
    let mut child = Command::new(env!("CARGO_BIN_EXE_intent-engine"))
        .args(["task", "add", "--name", "任务", "--spec-stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all("这是中文规格说明".as_bytes()).unwrap();
    drop(stdin);

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("这是中文规格说明"));
}
```

### 手动测试清单

- [ ] cmd.exe (默认 GBK)
- [ ] cmd.exe (chcp 65001 后)
- [ ] PowerShell 5.x (默认)
- [ ] PowerShell 7+
- [ ] Windows Terminal
- [ ] Git Bash for Windows

## 疑难解答

### 问题：PowerShell 管道传输中文仍然乱码

**症状**：
```powershell
PS> echo "实现 JWT 认证，支持刷新 Token，有效期 7 天" | intent-engine task add --name "测试" --spec-stdin
# spec 显示为: "?? JWT ??????? Token???? 7 ?"
```

**根本原因**：
- PowerShell 5.x 的 `echo` 默认使用系统代码页（通常是 GBK）
- 即使程序设置了 UTF-8，管道数据已经是 GBK 编码

**解决方法 1：使用 PowerShell 7+**
```powershell
# PowerShell 7 默认 UTF-8，无需配置
pwsh  # 启动 PowerShell 7
echo "实现 JWT 认证" | intent-engine task add --name "测试" --spec-stdin
```

**解决方法 2：在 PowerShell 5.x 中设置编码**
```powershell
# 在使用管道前设置
[Console]::InputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

echo "实现 JWT 认证" | intent-engine task add --name "测试" --spec-stdin
```

**解决方法 3：使用 Out-File + Get-Content**
```powershell
# 写入临时文件
"实现 JWT 认证" | Out-File -Encoding utf8 temp.txt
Get-Content temp.txt | intent-engine task add --name "测试" --spec-stdin
Remove-Item temp.txt
```

**解决方法 4：使用 Here-String**
```powershell
@"
实现 JWT 认证，支持刷新 Token，有效期 7 天
"@ | intent-engine task add --name "测试" --spec-stdin
```

**最佳实践**：
- 在 PowerShell Profile (`$PROFILE`) 中永久设置编码
- 或者使用 Windows Terminal + PowerShell 7

### 问题：升级后仍然看到乱码

**检查步骤**：

1. 确认版本是否为 v0.1.13+：
```bash
intent-engine --version
```

2. 检查控制台编码是否已设置：
```powershell
# PowerShell 中检查
[Console]::InputEncoding.CodePage   # 应该是 65001
[Console]::OutputEncoding.CodePage  # 应该是 65001
```

```cmd
REM cmd 中检查
chcp  # 应该显示 "活动代码页: 65001"
```

3. 测试简单命令（不使用管道）：
```bash
intent-engine task add --name "测试中文"
```

如果不使用管道能正常显示，说明是管道编码问题，请参考上面的 PowerShell 管道解决方法。

## 常见问题

### Q1: 为什么不直接输出 GBK？

**A**: GBK 是中文专用编码，不支持全球化。UTF-8 是现代标准，支持所有语言。

### Q2: 为什么 JSON 中中文显示为 \uXXXX？

**A**: 这是 JSON 的 Unicode 转义，完全正常。`serde_json` 默认转义非 ASCII 字符以确保兼容性。

如果希望可读，使用:
```rust
serde_json::to_string_pretty(&task)?
```

### Q3: 为什么 PowerShell 7 没问题，PowerShell 5 有问题？

**A**: PowerShell 7 是跨平台重写版本，默认 UTF-8。PowerShell 5.x 是 Windows 专有版本，继承了旧的编码系统。

### Q4: 为什么程序设置了 UTF-8，管道传输还是乱码？

**A**: 程序只能设置自己的控制台编码，无法改变 PowerShell 管道传输的编码。PowerShell 5.x 管道默认使用系统代码页。解决方法是在 PowerShell 中设置编码，或使用 PowerShell 7。

## 相关资源

- [Rust 字符串和编码](https://doc.rust-lang.org/book/ch08-02-strings.html)
- [Windows Console Unicode and UTF-8](https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences)
- [chcp 命令参考](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/chcp)
- [PowerShell 编码指南](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_character_encoding)

## 总结

Windows 控制台的中文编码问题根源在于历史遗留的代码页系统。最佳实践是：

1. **用户侧**: 使用 Windows Terminal 或配置 UTF-8
2. **开发侧**: 在代码中自动处理编码转换
3. **文档侧**: 清晰说明问题和解决方案

通过组合使用上述方案，可以为 Windows 用户提供良好的中文支持体验。
