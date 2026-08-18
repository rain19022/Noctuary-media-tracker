$gnu = Join-Path $env:USERPROFILE ".rustup\toolchains\stable-x86_64-pc-windows-gnu"
$gcc = Join-Path $gnu "lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained"
$env:PATH = "$gcc;$gnu\bin"
$env:RUSTC = Join-Path $gnu "bin\rustc.exe"
$env:RUSTFLAGS = "-C link-self-contained=yes"

# Some Windows Application Control policies block executables launched from
# user-profile cache folders. Use a short build directory on D: instead.
$env:CARGO_TARGET_DIR = "D:\shelf-target"

if ($args.Length -gt 0 -and $args[0] -eq "run") {
    $env:CARGO_TARGET_DIR = "D:\shelf-target-run"
    & (Join-Path $gnu "bin\cargo.exe") build
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }

    $builtExe = Join-Path $env:CARGO_TARGET_DIR "x86_64-pc-windows-gnu\debug\personal_media_tracker.exe"
    $runDir = "D:\shelf-run"
    $runExe = Join-Path $runDir "personal_media_tracker.exe"
    New-Item -ItemType Directory -Force -Path $runDir | Out-Null
    Copy-Item $builtExe $runExe -Force

    Get-Process -Name personal_media_tracker -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 300

    & $runExe
    exit $LASTEXITCODE
}

& (Join-Path $gnu "bin\cargo.exe") @args
