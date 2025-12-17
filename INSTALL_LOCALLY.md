# 本地安装 ie 到 Cargo 标准目录

## 快速安装

### 方法 1：从项目源码安装（推荐）

```bash
# 在项目根目录执行
cargo install --path . --force
```

### 方法 2：通过 Makefile

```bash
make install
```

---

## 安装位置

Cargo 会将 `ie` binary 安装到：

- **Linux/macOS/WSL**: `~/.cargo/bin/ie`
- **Windows**: `%USERPROFILE%\.cargo\bin\ie.exe`

**注意**：
- ✅ 只安装单个可执行文件 `ie`（约 7MB）
- ✅ 不安装配置文件或文档
- ✅ 如果之前有 `intent-engine` binary，会自动移除

---

## 完整安装步骤

### 1. 检查当前状态

```bash
# 查看是否已安装
which ie
ie --version 2>/dev/null || echo "未安装"
```

### 2. 执行安装

```bash
cargo install --path . --force
```

### 3. 验证安装

```bash
# 检查 binary 位置
which ie

# 测试版本
ie --version

# 运行健康检查
ie doctor
```

### 4. 配置 PATH（如需要）

如果 `which ie` 找不到命令：

```bash
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

## 安装选项

### `--path .`
从当前目录的源码构建并安装

### `--force`
强制覆盖已安装的版本（推荐使用）

### 其他选项

```bash
# 指定自定义安装目录
cargo install --path . --root /custom/path

# 仅构建不安装
cargo build --release
# binary 在 target/release/ie

# 查看已安装的包
cargo install --list | grep intent-engine
```

---

## 卸载

```bash
# 卸载
cargo uninstall intent-engine

# 验证
which ie  # 应该无输出
```

**注意**：使用包名 `intent-engine` 卸载，而不是 binary 名 `ie`

---

## 从 crates.io 安装（未来）

当项目发布后：

```bash
cargo install intent-engine
# 或指定版本
cargo install intent-engine --version 0.4.0
```

---

## 常见问题

### Q: 为什么包名是 intent-engine，但命令是 ie？

A: **包名**用于 Cargo 管理，**binary 名**是实际可执行文件。这样既保持包名完整性，又提供简短命令。

### Q: 安装失败怎么办？

```bash
# 清理后重试
cargo clean
cargo install --path . --force

# 更新 Rust
rustup update
```

### Q: 如何验证安装的文件？

```bash
# 查看 binary 信息
ls -lh ~/.cargo/bin/ie

# 查看大小
du -h ~/.cargo/bin/ie
```

---

## 总结

**推荐命令**：
```bash
cargo install --path . --force
```

**验证**：
```bash
which ie && ie --version && ie doctor
```

就这么简单！🎉
