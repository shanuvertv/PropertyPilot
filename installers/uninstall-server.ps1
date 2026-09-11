<#
.SYNOPSIS
  Removes the PropertyPilotServer Windows Service. Keeps the install directory (.env, logs)
  unless -Purge is given. Never touches the PostgreSQL database.
#>
[CmdletBinding()]
param(
  [string]$InstallDir = "C:\ProgramData\PropertyPilot\server",
  [switch]$Purge
)

$ErrorActionPreference = "Stop"
$ServiceName = "PropertyPilotServer"

$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($svc) {
  if ($svc.Status -ne "Stopped") {
    Stop-Service -Name $ServiceName -Force
    $svc.WaitForStatus("Stopped", (New-TimeSpan -Seconds 30))
  }
  & sc.exe delete $ServiceName | Out-Null
  Write-Host "Removed service $ServiceName"
} else {
  Write-Host "Service $ServiceName is not installed"
}

Get-NetFirewallRule -DisplayName "PropertyPilot Server" -ErrorAction SilentlyContinue | Remove-NetFirewallRule

if ($Purge -and (Test-Path $InstallDir)) {
  Remove-Item -Recurse -Force $InstallDir
  Write-Host "Deleted $InstallDir"
}
