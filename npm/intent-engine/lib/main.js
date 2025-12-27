"use strict";

const os = require("os");
const path = require("path");

// Platform to package name mapping
const PLATFORMS = {
  "darwin-arm64": "@m3task/intent-engine-darwin-arm64",
  "darwin-x64": "@m3task/intent-engine-darwin-x64",
  "linux-arm64": "@m3task/intent-engine-linux-arm64",
  "linux-x64": "@m3task/intent-engine-linux-x64",
  "win32-x64": "@m3task/intent-engine-win32-x64",
};

function getPlatformPackage() {
  const platform = os.platform();
  const arch = os.arch();
  const key = `${platform}-${arch}`;

  const packageName = PLATFORMS[key];
  if (!packageName) {
    throw new Error(
      `Unsupported platform: ${platform}-${arch}. ` +
        `Supported platforms: ${Object.keys(PLATFORMS).join(", ")}`
    );
  }

  return packageName;
}

function getBinaryPath() {
  const packageName = getPlatformPackage();
  const binaryName = os.platform() === "win32" ? "ie.exe" : "ie";

  try {
    // Try to find the platform package
    const packagePath = require.resolve(`${packageName}/package.json`);
    const packageDir = path.dirname(packagePath);
    return path.join(packageDir, "bin", binaryName);
  } catch (e) {
    throw new Error(
      `Could not find platform package '${packageName}'. ` +
        `Please reinstall @m3task/intent-engine: npm install -g @m3task/intent-engine`
    );
  }
}

module.exports = {
  getPlatformPackage,
  getBinaryPath,
};
