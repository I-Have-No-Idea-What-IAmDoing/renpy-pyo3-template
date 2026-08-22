# ==============================================================================
# Ren'Py Rust Build Wrapper (PowerShell)
# Delegates to 'cargo xtask dist'
# ==============================================================================
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$PassThruArgs
)

Push-Location -Path $PSScriptRoot
cargo xtask dist @PassThruArgs
Pop-Location