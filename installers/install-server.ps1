<#
.SYNOPSIS
  Installs renewal-server (PropertyPilot API + scheduler) as a Windows Service.

.DESCRIPTION
  Copies renewal-server.exe into the install directory, writes a .env there (kept if one
  already exists), registers the "PropertyPilotServer" service with automatic start and
  restart-on-failure, opens the firewall port, and starts it.

  Run from an elevated PowerShell:
    .\install-server.ps1 -DatabaseUrl "postgres://renewal:SECRET@localhost:5432/renewal" -BindAddr "0.0.0.0:8787"

  Re-running upgrades the binary in place (stops the service, copies, starts it again).

.PARAMETER Exe
  Path to the built renewal-server.exe (default: ..\target\release\renewal-server.exe).
.PARAMETER InstallDir
  Where the service lives (default: C:\ProgramData\PropertyPilot\server).
.PARAMETER DatabaseUrl
  PostgreSQL connection string. Required on first install; ignored if .env already exists.
.PARAMETER BindAddr
  Listen address. Use 0.0.0.0:8787 so desktop and Android clients on the LAN can reach it.
.PARAMETER Timezone
  Organisation timezone used for "today" (default Asia/Dubai).
.PARAMETER TlsCert / TlsKey
  Optional PEM certificate and key; when both are given the server serves HTTPS.
#>
[CmdletBinding()]
param(
  [string]$Exe = (Join-Path $PSScriptRoot "..\target\release\renewal-server.exe"),
  [string]$InstallDir = "C:\ProgramData\PropertyPilot\server",
  [string]$DatabaseUrl,
  [string]$BindAddr = "0.0.0.0:8787",
  [string]$Timezone = "Asia/Dubai",
  [string]$TlsCert,
  [string]$TlsKey
)

$ErrorActionPreference = "Stop"
$ServiceName = "PropertyPilotServer"
$DisplayName = "PropertyPilot Server"

$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw "Run this script from an elevated (Administrator) PowerShell."
}
if (-not (Test-Path $Exe)) {
  throw "renewal-server.exe not found at $Exe. Build it first: cargo build --release -p renewal-server"
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $InstallDir "logs") | Out-Null

$envPath = Join-Path $InstallDir ".env"
if (-not (Test-Path $envPath)) {
  if (-not $DatabaseUrl) { throw "First install: pass -DatabaseUrl (no .env exists yet in $InstallDir)." }
  $lines = @(
    "# PropertyPilot server configuration (read at service start)",
    "DATABASE_URL=$DatabaseUrl",
    "BIND_ADDR=$BindAddr",
    "ORG_TIMEZONE=$Timezone",
    "SCHEDULER=on",
    "RUST_LOG=info,renewal_server=info,sqlx=warn",
    "LOG_DIR=$(Join-Path $InstallDir 'logs')",
    "",
    "# Email: log | smtp | graph  (see .env.example in the repository for the SMTP/Graph keys)",
    "MAIL_PROVIDER=log",
    "MAIL_FROM_NAME=Leasing Department",
    "MAIL_FROM_ADDRESS=leasing@example.com"
  )
  if ($TlsCert -and $TlsKey) {
    $lines += ""
    $lines += "TLS_CERT=$TlsCert"
    $lines += "TLS_KEY=$TlsKey"
  }
  Set-Content -Path $envPath -Value ($lines -join "`r`n") -Encoding utf8
  Write-Host "Wrote $envPath — edit it to configure email before going live."
} else {
  Write-Host "Keeping existing $envPath"
}

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing -and $existing.Status -ne "Stopped") {
  Write-Host "Stopping $ServiceName..."
  Stop-Service -Name $ServiceName -Force
  $existing.WaitForStatus("Stopped", (New-TimeSpan -Seconds 30))
}

$target = Join-Path $InstallDir "renewal-server.exe"
Copy-Item -Path $Exe -Destination $target -Force
Write-Host "Installed binary to $target"

if (-not $existing) {
  $binPath = "`"$target`" --service"
  & sc.exe create $ServiceName binPath= $binPath start= auto DisplayName= "$DisplayName" | Out-Null
  & sc.exe description $ServiceName "PropertyPilot rental contract renewal API and scheduler" | Out-Null
  Write-Host "Registered service $ServiceName"
}
# Restart automatically after crashes (5s, 30s, then every minute), reset the counter daily.
& sc.exe failure $ServiceName reset= 86400 actions= restart/5000/restart/30000/restart/60000 | Out-Null

$port = ($BindAddr -split ":")[-1]
if (-not (Get-NetFirewallRule -DisplayName "PropertyPilot Server" -ErrorAction SilentlyContinue)) {
  New-NetFirewallRule -DisplayName "PropertyPilot Server" -Direction Inbound -Protocol TCP -LocalPort $port -Action Allow -Profile Domain,Private | Out-Null
  Write-Host "Opened inbound TCP $port on Domain/Private profiles"
}

Start-Service -Name $ServiceName
(Get-Service -Name $ServiceName).WaitForStatus("Running", (New-TimeSpan -Seconds 30))
Write-Host "$ServiceName is running. Logs: $(Join-Path $InstallDir 'logs')"
