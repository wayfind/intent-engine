class IntentEngine < Formula
  desc "Command-line database service for tracking strategic intent, tasks, and events"
  homepage "https://github.com/wayfind/intent-engine"
  version "0.1.3"
  license "MIT OR Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/wayfind/intent-engine/releases/download/v#{version}/intent-engine-macos-aarch64.tar.gz"
      # SHA256 will be updated automatically during release
      sha256 "PLACEHOLDER_ARM64_SHA256"
    else
      url "https://github.com/wayfind/intent-engine/releases/download/v#{version}/intent-engine-macos-x86_64.tar.gz"
      # SHA256 will be updated automatically during release
      sha256 "PLACEHOLDER_X86_64_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/wayfind/intent-engine/releases/download/v#{version}/intent-engine-linux-aarch64.tar.gz"
      # SHA256 will be updated automatically during release
      sha256 "PLACEHOLDER_LINUX_ARM64_SHA256"
    else
      url "https://github.com/wayfind/intent-engine/releases/download/v#{version}/intent-engine-linux-x86_64.tar.gz"
      # SHA256 will be updated automatically during release
      sha256 "PLACEHOLDER_LINUX_X86_64_SHA256"
    end
  end

  def install
    bin.install "intent-engine"
  end

  test do
    system "#{bin}/intent-engine", "--version"
    system "#{bin}/intent-engine", "doctor"
  end
end
