param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('build', 'run')]
    [string]$Action,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Arguments
)

$ErrorActionPreference = 'Stop'
# ktx2-rw searches MINGW_PREFIX/lib instead of discovering custom GCC installs.
$gcc = (Get-Command gcc -CommandType Application -ErrorAction Stop).Source
$prefix = Split-Path (Split-Path $gcc -Parent) -Parent
if (-not (Test-Path (Join-Path $prefix 'lib'))) {
    throw "GNU toolchain library directory missing under $prefix"
}
$env:MINGW_PREFIX = $prefix
# Bindgen's libclang does not discover this separately installed GNU sysroot.
$clang = Join-Path $env:ProgramFiles 'LLVM/bin/clang.exe'
if (-not (Test-Path $clang)) {
    throw 'Install LLVM (winget install --exact --id LLVM.LLVM) before building.'
}
$resourceDirectory = & $clang -print-resource-dir
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$gnuTarget = & $gcc -dumpmachine
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$sysroot = Join-Path $prefix $gnuTarget
$env:LIBCLANG_PATH = Split-Path $clang -Parent
$env:BINDGEN_EXTRA_CLANG_ARGS = '-resource-dir="' + $resourceDirectory + '" --sysroot="' + $sysroot + '"'
$manifest = Join-Path (Split-Path $PSScriptRoot -Parent) 'Cargo.toml'
# Windows PowerShell 5 treats redirected native stderr as errors, including Cargo progress.
# Keep that output visible and use Cargo's exit code as the failure boundary.
$ErrorActionPreference = 'Continue'
& cargo $Action --manifest-path $manifest --target x86_64-pc-windows-gnu --bin game-engine --no-default-features --features casc,dev @Arguments
exit $LASTEXITCODE
