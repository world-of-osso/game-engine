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
$manifest = Join-Path (Split-Path $PSScriptRoot -Parent) 'Cargo.toml'
& cargo $Action --manifest-path $manifest --target x86_64-pc-windows-gnu --bin game-engine --no-default-features --features casc,dev @Arguments
exit $LASTEXITCODE
