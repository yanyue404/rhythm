param(
    [string]$Version = "",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$ProjectDir = Split-Path -Parent $PSScriptRoot
$RepoRoot = Split-Path -Parent $ProjectDir
$CargoToml = Join-Path $ProjectDir "Cargo.toml"

if (!(Test-Path $ProjectDir)) {
    throw "windows 项目目录不存在: $ProjectDir"
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    $toml = Get-Content $CargoToml -Raw
    $match = [regex]::Match($toml, 'version\s*=\s*"([^"]+)"')
    if (!$match.Success) {
        throw "无法从 Cargo.toml 解析版本号"
    }
    $Version = $match.Groups[1].Value
}

$DistRoot = Join-Path $RepoRoot "dist"
$OutDir = Join-Path $DistRoot ("RhythmWin-v{0}-win64" -f $Version)
$ZipPath = Join-Path $DistRoot ("RhythmWin-v{0}-win64.zip" -f $Version)
$ExePath = Join-Path $ProjectDir "target\release\rhythm-win.exe"

New-Item -ItemType Directory -Path $DistRoot -Force | Out-Null

if (-not $SkipBuild) {
    Push-Location $ProjectDir
    try {
        $cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
        if ($null -eq $cargoCmd) {
            throw "未找到 cargo 命令，请先安装 Rust 并确保 cargo 在 PATH 中"
        }
        & $cargoCmd.Source build --release
    }
    finally {
        Pop-Location
    }
}

if (!(Test-Path $ExePath)) {
    throw "未找到 release 可执行文件: $ExePath"
}

if (Test-Path $OutDir) {
    Remove-Item -Path $OutDir -Recurse -Force
}
New-Item -ItemType Directory -Path $OutDir | Out-Null

Copy-Item $ExePath (Join-Path $OutDir "rhythm-win.exe")
Copy-Item (Join-Path $RepoRoot "LICENSE") (Join-Path $OutDir "LICENSE")
Copy-Item (Join-Path $ProjectDir "README.md") (Join-Path $OutDir "README.md")

if (Test-Path $ZipPath) {
    Remove-Item $ZipPath -Force
}
Compress-Archive -Path (Join-Path $OutDir "*") -DestinationPath $ZipPath -CompressionLevel Optimal

Write-Host "打包完成:"
Write-Host "目录: $OutDir"
Write-Host "压缩包: $ZipPath"
