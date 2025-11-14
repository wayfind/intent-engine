# 本地安装 ie 到 Cargo 标准目录

## 快速安装

### 方法 1：从项目源码安装（推荐）

```bash
# 1. 确保在项目根目录
cd /mnt/d/prj/intent-engine

# 2. 安装到 ~/.cargo/bin
cargo install --path .

# 或者使用 --force 覆盖已有版本
cargo install --path . --force
```

### 方法 2：通过 Makefile

```bash
# 使用项目提供的 Makefile
make install
```

---

## 详细说明

### 安装位置

Cargo 会将 `ie` binary 安装到：

- **Linux/macOS/WSL**: `~/.cargo/bin/ie`
- **Windows**: `%USERPROFILE%\.cargo\bin\ie.exe`

### 安装内容

```
~/.cargo/bin/
└── ie           # 单个可执行文件（约 7MB）
```

**注意**：
- ✅ 只会安装 `ie` 这一个可执行文件
- ✅ 不会安装配置文件或文档
- ✅ 不会修改系统级目录
- ✅ 如果之前有 `intent-engine` binary，会自动移除

---

## 完整安装步骤

### 步骤 1：检查当前状态

```bash
# 查看是否已安装旧版本
which ie
which intent-engine

# 查看当前安装的版本（如果有）
ie --version 2>/dev/null || echo "未安装"
intent-engine --version 2>/dev/null || echo "未安装"
```

### 步骤 2：执行安装

```bash
# 在项目根目录执行
cargo install --path . --force
```

**预期输出**：
```
   Compiling intent-engine v0.3.3 (/mnt/d/prj/intent-engine)
    Finished release [optimized] target(s) in 40.63s
   Replacing /home/user/.cargo/bin/ie
    Removing executable `/home/user/.cargo/bin/intent-engine` from previous version intent-engine v0.3.3
    Replaced package `intent-engine v0.3.3` with `intent-engine v0.3.3 (/mnt/d/prj/intent-engine)` (executable `ie`)
```

### 步骤 3：验证安装

```bash
# 1. 检查 binary 位置
which ie
# 应该输出: /home/user/.cargo/bin/ie

# 2. 检查旧 binary 是否已移除
which intent-engine
# 应该无输出或提示未找到

# 3. 测试版本
ie --version
# 应该输出: intent-engine 0.3.3

# 4. 运行健康检查
ie doctor
```

### 步骤 4：确保 PATH 配置

如果 `which ie` 找不到命令，需要确保 `~/.cargo/bin` 在 PATH 中：

```bash
# 检查 PATH
echo $PATH | grep -q ".cargo/bin" && echo "✅ PATH 已配置" || echo "❌ 需要配置 PATH"

# 如果需要配置，添加到 shell 配置文件：
# Bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Zsh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# Fish
fish_add_path $HOME/.cargo/bin
```

---

## 安装选项说明

### `--path .`
- 从当前目录的源码构建并安装
- 使用项目根目录的 `Cargo.toml`

### `--force`
- 强制覆盖已安装的版本
- **推荐使用**，避免版本冲突提示

### 其他有用选项

```bash
# 指定安装目录（如果不想用默认的 ~/.cargo/bin）
cargo install --path . --root /custom/path

# 仅构建不安装（用于测试）
cargo build --release
# binary 会在 target/release/ie

# 查看安装的详细信息
cargo install --list | grep intent-engine
```

---

## 卸载

如果需要卸载：

```bash
# 卸载 ie
cargo uninstall intent-engine

# 验证已卸载
which ie
# 应该无输出
```

**注意**：
- 卸载命令使用的是 **包名** `intent-engine`，而不是 binary 名 `ie`
- 这是因为 `Cargo.toml` 中的 `name = "intent-engine"`

---

## 从 crates.io 安装（未来）

当项目发布到 crates.io 后，可以直接安装：

```bash
# 从 crates.io 安装
cargo install intent-engine

# 或指定版本
cargo install intent-engine --version 0.3.3
```

---

## 常见问题

### Q: 为什么包名是 intent-engine，但 binary 是 ie？

A:
- **包名** (`intent-engine`) 用于 Cargo 识别和管理
- **Binary 名** (`ie`) 是实际的可执行文件名
- 这样设计既保持了包名的完整性，又提供了简短的命令名

### Q: 安装后旧的 intent-engine binary 会怎样？

A: Cargo 会自动移除旧的 `intent-engine` binary，你会在安装日志中看到：
```
Removing executable `/home/user/.cargo/bin/intent-engine` from previous version
```

### Q: 我可以同时保留 ie 和 intent-engine 两个命令吗？

A: 不推荐。新版本只提供 `ie` 命令。如果需要，可以创建符号链接：
```bash
# 不推荐，但如果需要向后兼容
ln -s ~/.cargo/bin/ie ~/.cargo/bin/intent-engine
```

### Q: 安装失败怎么办？

A: 常见原因和解决方法：

1. **编译错误**
   ```bash
   # 清理后重试
   cargo clean
   cargo install --path . --force
   ```

2. **权限问题**
   ```bash
   # 确保 ~/.cargo/bin 有写权限
   chmod +w ~/.cargo/bin
   ```

3. **Rust 版本过旧**
   ```bash
   # 更新 Rust
   rustup update
   ```

### Q: 如何验证安装的文件？

A:
```bash
# 查看 binary 信息
ls -lh ~/.cargo/bin/ie

# 查看依赖
ldd ~/.cargo/bin/ie  # Linux
otool -L ~/.cargo/bin/ie  # macOS

# 查看大小
du -h ~/.cargo/bin/ie
```

---

## 一键安装脚本

创建一个便捷脚本：

```bash
#!/bin/bash
# install-ie.sh

set -e

echo "🚀 安装 ie 到 ~/.cargo/bin"
echo ""

# 检查是否在项目目录
if [ ! -f "Cargo.toml" ] || ! grep -q "name = \"intent-engine\"" Cargo.toml; then
    echo "❌ 错误：请在 intent-engine 项目根目录运行此脚本"
    exit 1
fi

# 显示当前状态
echo "📍 当前状态："
which ie 2>/dev/null && echo "  ie: $(which ie)" || echo "  ie: 未安装"
which intent-engine 2>/dev/null && echo "  intent-engine: $(which intent-engine)" || echo "  intent-engine: 未安装"
echo ""

# 安装
echo "📦 开始安装..."
cargo install --path . --force

echo ""
echo "✅ 安装完成！"
echo ""

# 验证
echo "🔍 验证安装："
echo "  位置: $(which ie)"
echo "  版本: $(ie --version)"
echo ""

# 运行健康检查
echo "🏥 运行健康检查..."
ie doctor | head -10
echo ""

echo "✨ ie 已成功安装到 $(which ie)"
echo ""
echo "下一步："
echo "  - 运行: ie --help"
echo "  - 测试: ie task add --name 'Test task'"
echo "  - 文档: cat README.md"
```

使用方法：
```bash
chmod +x install-ie.sh
./install-ie.sh
```

---

## 总结

**推荐安装命令**：
```bash
cargo install --path . --force
```

**安装位置**：
```
~/.cargo/bin/ie
```

**验证命令**：
```bash
which ie && ie --version && ie doctor
```

就这么简单！🎉
