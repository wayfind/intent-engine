# Windows 中文编码支持改进

## 概述

本次更新添加了对 Windows 命令行环境（cmd.exe 和 PowerShell）中中文字符的完整支持，解决了中文输入输出乱码的问题。

## 主要更改

### 1. 新增 Windows 控制台支持模块

**文件**: `src/windows_console.rs`

功能：
- `setup_windows_console()`: 自动配置 Windows 控制台为 UTF-8 模式
- `is_console_utf8()`: 检测当前控制台是否使用 UTF-8
- `get_console_code_page()`: 获取当前代码页编号
- `code_page_name()`: 获取代码页的友好名称

实现细节：
- 使用 Windows API (`SetConsoleOutputCP`) 设置控制台输出为 UTF-8 (代码页 65001)
- 启用虚拟终端处理 (`ENABLE_VIRTUAL_TERMINAL_PROCESSING`) 支持 ANSI 转义序列
- 仅在 Windows 平台编译，其他平台为空操作

### 2. 主程序集成

**文件**: `src/main.rs`

在程序启动时自动调用 `setup_windows_console()`，确保：
- 中文字符正确显示
- 如果设置失败，显示友好的警告信息和解决建议

### 3. 添加 Windows 依赖

**文件**: `Cargo.toml`

添加了 `windows` crate (仅在 Windows 平台)：
```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = ["Win32_System_Console"] }
```

### 4. 完整的测试套件

**文件**: `tests/windows_encoding_tests.rs`

包含 9 个测试用例：
- ✅ 中文任务名称
- ✅ 中文任务规格（从 stdin）
- ✅ 中文事件数据
- ✅ 中英文混合
- ✅ 特殊中文标点符号
- ✅ Emoji 支持
- ✅ 中文内容搜索
- ✅ 中文任务报告生成
- ✅ Windows 控制台设置验证

### 5. 详细文档

**文件**: `docs/zh-CN/technical/windows-encoding.md`

包含：
- 问题根源分析
- 三种解决方案对比（用户配置、自动处理、文档引导）
- 详细的配置指南（cmd、PowerShell、Windows Terminal）
- 代码实现示例
- 测试建议
- 常见问题解答

### 6. 示例脚本

**文件**:
- `examples/windows-utf8-example.bat` - cmd 批处理示例
- `examples/windows-utf8-example.ps1` - PowerShell 示例

## 技术细节

### Windows 编码机制

1. **cmd.exe**: 默认使用系统代码页（中文 Windows 通常是 CP936/GBK）
2. **PowerShell 5.x**: 继承系统代码页
3. **PowerShell 7+**: 默认 UTF-8

### Rust 字符串特性

- Rust 字符串内部始终是 UTF-8
- `println!` 输出 UTF-8 字节流
- 当控制台不是 UTF-8 时会出现乱码

### 解决方案

通过 Windows API 在程序启动时自动设置控制台为 UTF-8，确保：
- 输出的中文字符正确显示
- 从 stdin 读取的中文数据正确解析
- JSON 输出包含的中文字段可读

## 用户影响

### 使用前
```bash
# Windows cmd (默认 GBK)
intent-engine task add --name "测试任务"
# 输出: ��� 或其他乱码
```

### 使用后
```bash
# 无需任何配置
intent-engine task add --name "测试任务"
# 输出: 正确显示 "测试任务"
```

## 向后兼容性

✅ **完全兼容** - 所有更改：
- 仅影响 Windows 平台
- 不改变命令行接口
- 不影响 JSON 输出格式
- 在非 Windows 平台上为空操作

## 测试

所有测试通过：
```bash
cargo test --test windows_encoding_tests
cargo check --all-targets  # ✅ 成功
```

## 相关 Issue

Fixes: Windows 命令行中文字符输入和显示问题

## 参考资料

- [Rust 字符串和编码](https://doc.rust-lang.org/book/ch08-02-strings.html)
- [Windows Console Unicode](https://learn.microsoft.com/en-us/windows/console/)
- [chcp 命令参考](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/chcp)

## 后续改进

可选的未来增强（不在本次范围内）：
1. 自动检测并提示用户控制台编码问题
2. 支持从 GBK 输入自动转换
3. 添加 `--force-utf8` 命令行标志
