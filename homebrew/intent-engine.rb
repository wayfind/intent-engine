class IntentEngine < Formula
  desc "AI long-term task memory system - cross-session persistence, hierarchical tasks, decision records"
  homepage "https://github.com/wayfind/intent-engine"
  version "0.10.0"
  license "MIT OR Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/wayfind/intent-engine/releases/download/v#{version}/intent-engine-macos-aarch64.tar.gz"
      sha256 "PLACEHOLDER_ARM64_SHA256"
    else
      url "https://github.com/wayfind/intent-engine/releases/download/v#{version}/intent-engine-macos-x86_64.tar.gz"
      sha256 "PLACEHOLDER_X86_64_SHA256"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/wayfind/intent-engine/releases/download/v#{version}/intent-engine-linux-aarch64.tar.gz"
      sha256 "PLACEHOLDER_LINUX_ARM64_SHA256"
    else
      url "https://github.com/wayfind/intent-engine/releases/download/v#{version}/intent-engine-linux-x86_64.tar.gz"
      sha256 "PLACEHOLDER_LINUX_X86_64_SHA256"
    end
  end

  def install
    bin.install "ie"
  end

  test do
    system "#{bin}/ie", "--version"
    system "#{bin}/ie", "doctor"
  end
end
