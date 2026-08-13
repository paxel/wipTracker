<#
.SYNOPSIS
    Puts WipTracker in the Windows Start menu, and optionally starts it at login.

.DESCRIPTION
    The zip release is just an executable in whatever folder it was unpacked into, so
    nothing offers it in the Start menu. This creates the shortcut, next to the
    wiptracker.exe that sits beside this script. Scoop users already get the Start menu
    entry from the manifest and do not need this.

    Everything is per-user: no administrator rights, nothing written outside the profile.

.PARAMETER Startup
    Also start WipTracker when you log in.

.PARAMETER Remove
    Delete the shortcuts instead of creating them.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File install-shortcut.ps1 -Startup
#>
[CmdletBinding()]
param(
    [switch] $Startup,
    [switch] $Remove
)

$ErrorActionPreference = 'Stop'

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $here 'wiptracker.exe'
if (-not $Remove -and -not (Test-Path $exe)) {
    throw "wiptracker.exe not found next to this script ($here)."
}

$startMenu = Join-Path ([Environment]::GetFolderPath('Programs')) 'WipTracker.lnk'
$startupDir = Join-Path ([Environment]::GetFolderPath('Startup')) 'WipTracker.lnk'

function Set-Shortcut([string] $Path) {
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($Path)
    $shortcut.TargetPath = $exe
    $shortcut.WorkingDirectory = $here
    $shortcut.Description = 'The task you are focused on right now, in a one-line bar'
    $shortcut.Save()
    Write-Host "  $Path"
}

if ($Remove) {
    foreach ($path in @($startMenu, $startupDir)) {
        if (Test-Path $path) {
            Remove-Item $path
            Write-Host "  removed $path"
        }
    }
    return
}

Write-Host 'Created:'
Set-Shortcut $startMenu
if ($Startup) {
    Set-Shortcut $startupDir
} else {
    Write-Host 'Pass -Startup to also start WipTracker when you log in.'
}
