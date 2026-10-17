<#
.SYNOPSIS
  Synapse Standby - park Razer Synapse so it stops running, while hardware keeps the last applied settings.

.USAGE (elevated PowerShell, or just run it and it self-elevates)
  .\synapse-standby.ps1 status          Show what Synapse stuff is running / enabled
  .\synapse-standby.ps1 standby         Kill processes, stop + disable services, disable tasks, remove autostarts
  .\synapse-standby.ps1 standby -KeepLighting   Park Synapse but keep Chroma lighting animating
  .\synapse-standby.ps1 guard-on -KeepLighting  Same, re-applied at every boot/logon
  .\synapse-standby.ps1 resume          Restore everything standby changed (startup types, tasks, autostarts)
  .\synapse-standby.ps1 guard-on        Re-run standby at every boot/logon (survives Synapse updates re-enabling itself)
  .\synapse-standby.ps1 guard-off       Remove that boot/logon guard

.WORKFLOW
  1. resume (or just launch Synapse), set up everything you want, let it apply.
  2. standby.
  3. Optional: guard-on so a Synapse update can't quietly re-arm itself.

.NOTES
  - Kernel drivers (rz*.sys) are NOT touched, so keyboard/mouse input keeps working.
  - Only settings stored in device firmware/onboard memory persist. Software-driven things
    (Chroma effects, macros, per-game profiles, DPI stages on some devices, and possibly the
    Blade undervolt offset) need the service. Verify per-feature; see README notes in chat.
  - Anything in $Keep is never touched (razer-ctl, R-Helper, razer-tray).
#>
param(
  [Parameter(Position = 0)]
  [ValidateSet('status', 'standby', 'resume', 'guard-on', 'guard-off')]
  [string]$Action = 'status',
  [switch]$Quiet,
  [switch]$KeepLighting,  # standby only: keep Chroma lighting alive, park Synapse itself
  [switch]$DropGuard      # resume only: also remove the boot/logon guard
)

$ErrorActionPreference = 'Continue'

# ---- config ---------------------------------------------------------------
$Keep      = @('razer-ctl', 'razer-tray', 'r-helper', 'rhelper', 'ghelper')   # never touched (lowercase)
$Pattern   = '^(razer|rz)'                                                    # name prefix for procs/services/tasks/autostarts
$StateDir  = Join-Path $env:ProgramData 'SynapseStandby'
$StateFile = Join-Path $StateDir 'state.json'
$GuardName = 'SynapseStandby-Guard'

# -KeepLighting: only run the Chroma half of the app engine, park these services
$EngineExe    = 'C:\Program Files\Razer\RazerAppEngine\RazerAppEngine.exe'
$ChromaArgs   = '--url-params=apps=chroma-app --launch-force-hidden=chroma-app --autoStart=1'
$LightingPark = @('RazerExperienceService', 'Razer Game Manager Service 3', 'RGS', 'Razer Chroma Stream Server')
$RunKeys   = @(
  'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run',
  'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run',
  'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run'
)

# The boot/logon guard runs as SYSTEM, whose HKCU is not the user's. When running as SYSTEM, also
# process the Run key of every loaded real-user hive (present at logon) so the autostart gets fixed.
if ($env:USERNAME -like '*$') {
  Get-ChildItem 'Registry::HKEY_USERS' -ErrorAction SilentlyContinue |
    Where-Object { $_.PSChildName -match '^S-1-5-21-\d+-\d+-\d+-\d+$' } |
    ForEach-Object { $RunKeys += "Registry::HKEY_USERS\$($_.PSChildName)\SOFTWARE\Microsoft\Windows\CurrentVersion\Run" }
}

# ---- helpers --------------------------------------------------------------
function Say($m) { if (-not $Quiet) { Write-Host $m } }

function Test-Admin {
  ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Test-Kept($name) { $Keep -contains ($name.ToLower() -replace '\.exe$', '') }

function Get-SynSvc {
  Get-CimInstance Win32_Service | Where-Object {
    ($_.Name -match $Pattern -or $_.DisplayName -match '^Razer') -and
    -not (Test-Kept $_.Name) -and
    $_.PathName -notmatch '\.sys'
  }
}

function Get-SynProc {
  Get-Process | Where-Object { $_.Name -match $Pattern -and -not (Test-Kept $_.Name) }
}

function Get-SynTask {
  Get-ScheduledTask -ErrorAction SilentlyContinue | Where-Object {
    ($_.TaskName -match 'razer' -or $_.TaskPath -match 'razer') -and $_.TaskName -ne $GuardName
  }
}

function Load-State {
  if (Test-Path $StateFile) { Get-Content $StateFile -Raw | ConvertFrom-Json }
  else { [pscustomobject]@{ services = @(); tasks = @(); runs = @() } }
}

function Save-State($s) {
  New-Item -ItemType Directory -Force -Path $StateDir | Out-Null
  $s | ConvertTo-Json -Depth 5 | Set-Content $StateFile -Encoding UTF8
}

# ---- elevation ------------------------------------------------------------
if (-not (Test-Admin)) {
  Say 'Elevating...'
  $argList = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"", $Action)
  if ($Quiet) { $argList += '-Quiet' }
  if ($KeepLighting) { $argList += '-KeepLighting' }
  if ($DropGuard) { $argList += '-DropGuard' }
  Start-Process powershell -Verb RunAs -ArgumentList $argList
  exit
}

# ---- actions --------------------------------------------------------------
switch ($Action) {

  'status' {
    Say '== Services =='
    Get-SynSvc | Sort-Object Name | Format-Table Name, State, StartMode, DisplayName -AutoSize
    Say '== Processes =='
    Get-SynProc | Sort-Object Name | Format-Table Name, Id, @{n = 'MB'; e = { [int]($_.WorkingSet64 / 1MB) } } -AutoSize
    Say '== Scheduled tasks =='
    Get-SynTask | Format-Table TaskName, State, TaskPath -AutoSize
    Say '== Autostart entries =='
    foreach ($k in $RunKeys) {
      if (Test-Path $k) {
        $p = Get-ItemProperty $k
        $p.PSObject.Properties | Where-Object { $_.Name -notmatch '^PS' -and ($_.Name -match 'razer' -or "$($_.Value)" -match 'razer') } |
          ForEach-Object { '{0}  {1} = {2}' -f $k, $_.Name, $_.Value }
      }
    }
    Say "== Guard task: $(if (Get-ScheduledTask -TaskName $GuardName -ErrorAction SilentlyContinue) {'ON'} else {'off'})"
  }

  'standby' {
    $state = Load-State
    $svcSaved = @($state.services)
    $taskSaved = @($state.tasks)
    $runSaved = @($state.runs)

    # 1) autostarts: remember + remove
    foreach ($k in $RunKeys) {
      if (-not (Test-Path $k)) { continue }
      $p = Get-ItemProperty $k
      foreach ($prop in $p.PSObject.Properties) {
        if ($prop.Name -match '^PS') { continue }
        if (($prop.Name -match 'razer' -or "$($prop.Value)" -match 'razer') -and -not (Test-Kept $prop.Name) -and
            -not ("$($prop.Value)".ToLower() -match 'razer-ctl|razer-tray|r-helper|ghelper')) {
          if (-not ($runSaved | Where-Object { $_.key -eq $k -and $_.name -eq $prop.Name })) {
            $runSaved += [pscustomobject]@{ key = $k; name = $prop.Name; value = "$($prop.Value)" }
          }
          if ($KeepLighting -and "$($prop.Value)" -match 'RazerAppEngine\.exe') {
            # keep the engine autostart, but Chroma-only (no Synapse UI)
            Set-ItemProperty -Path $k -Name $prop.Name -Value "`"$EngineExe`" $ChromaArgs"
            Say "autostart changed to Chroma-only: $($prop.Name)"
          } else {
            Remove-ItemProperty -Path $k -Name $prop.Name -ErrorAction SilentlyContinue
            Say "autostart removed: $($prop.Name)"
          }
        }
      }
    }

    # 2) scheduled tasks: remember + disable
    foreach ($t in Get-SynTask) {
      if ($t.State -ne 'Disabled') {
        if (-not ($taskSaved | Where-Object { $_.path -eq $t.TaskPath -and $_.name -eq $t.TaskName })) {
          $taskSaved += [pscustomobject]@{ path = $t.TaskPath; name = $t.TaskName }
        }
        Disable-ScheduledTask -TaskPath $t.TaskPath -TaskName $t.TaskName -ErrorAction SilentlyContinue | Out-Null
        Say "task disabled: $($t.TaskName)"
      }
    }

    # 3) services: remember startup type, disable, stop
    $svcs = Get-SynSvc
    if ($KeepLighting) { $svcs = $svcs | Where-Object { $LightingPark -contains $_.Name } }
    foreach ($s in $svcs) {
      if ($s.StartMode -ne 'Disabled' -and -not ($svcSaved | Where-Object { $_.name -eq $s.Name })) {
        $svcSaved += [pscustomobject]@{ name = $s.Name; start = $s.StartMode }
      }
      Set-Service -Name $s.Name -StartupType Disabled -ErrorAction SilentlyContinue
    }
    Save-State ([pscustomobject]@{ services = $svcSaved; tasks = $taskSaved; runs = $runSaved })

    foreach ($s in $svcs) {
      Stop-Service -Name $s.Name -Force -ErrorAction SilentlyContinue
      Say "service parked: $($s.Name)"
    }

    # 4) processes
    if ($KeepLighting) {
      # Park the Synapse-launched engine; leave a Chroma-only engine and all Rz* device/SDK processes alone.
      $engines = @(Get-CimInstance Win32_Process -Filter "Name='RazerAppEngine.exe'")
      $synEngine = $engines | Where-Object { $_.CommandLine -match 'apps=[^ ]*synapse' }
      $chromaOnly = $engines | Where-Object { $_.CommandLine -match 'apps=chroma-app(\s|$)' }
      if ($synEngine) {
        Stop-Process -Name RazerAppEngine -Force -ErrorAction SilentlyContinue
        Say 'Synapse engine killed.'
        Start-Sleep -Seconds 1
        $isSystem = ($env:USERNAME -like '*$')
        if (-not $isSystem) {
          # relaunch Chroma-only as the normal (non-elevated) user so it doesn't run as admin
          $me   = "$env:USERDOMAIN\$env:USERNAME"
          $act  = New-ScheduledTaskAction -Execute $EngineExe -Argument $ChromaArgs
          $prin = New-ScheduledTaskPrincipal -UserId $me -LogonType Interactive -RunLevel Limited
          Register-ScheduledTask -TaskName 'SynapseStandby-Launch' -Action $act -Principal $prin -Force | Out-Null
          Start-ScheduledTask -TaskName 'SynapseStandby-Launch'
          Start-Sleep -Seconds 3
          Unregister-ScheduledTask -TaskName 'SynapseStandby-Launch' -Confirm:$false -ErrorAction SilentlyContinue
          Say 'Chroma-only engine launched.'
        } else {
          Say 'Running as SYSTEM: not relaunching; the autostart entry will start Chroma-only at next logon.'
        }
      } elseif ($chromaOnly) {
        Say 'Chroma-only engine already running; leaving it.'
      } else {
        Say 'No engine running; Chroma-only will start at next logon (or run the Chroma-only command by hand).'
      }
      Say 'Lighting mode: Synapse parked, Chroma kept alive.'
    } else {
      # full standby: two passes, since some respawn each other
      1..2 | ForEach-Object {
        Get-SynProc | ForEach-Object {
          Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
          Say "killed: $($_.Name)"
        }
        Start-Sleep -Milliseconds 600
      }
      $left = @(Get-SynProc)
      if ($left.Count) { Say "WARNING: still running: $($left.Name -join ', ')" } else { Say 'Synapse is in standby.' }
    }
  }

  'resume' {
    $state = Load-State
    foreach ($s in $state.services) {
      $mode = switch ($s.start) { 'Auto' { 'Automatic' } 'Manual' { 'Manual' } default { 'Manual' } }
      Set-Service -Name $s.name -StartupType $mode -ErrorAction SilentlyContinue
      if ($s.start -eq 'Auto') { Start-Service -Name $s.name -ErrorAction SilentlyContinue }
      Say "service restored: $($s.name) -> $mode"
    }
    foreach ($t in $state.tasks) {
      Enable-ScheduledTask -TaskPath $t.path -TaskName $t.name -ErrorAction SilentlyContinue | Out-Null
      Say "task enabled: $($t.name)"
    }
    foreach ($r in $state.runs) {
      if (-not (Test-Path $r.key)) { New-Item -Path $r.key -Force | Out-Null }
      New-ItemProperty -Path $r.key -Name $r.name -Value $r.value -PropertyType String -Force | Out-Null
      Say "autostart restored: $($r.name)"
    }
    Remove-Item $StateFile -ErrorAction SilentlyContinue
    if ($DropGuard) { Unregister-ScheduledTask -TaskName $GuardName -Confirm:$false -ErrorAction SilentlyContinue }
    Say 'Synapse resumed. Launch it from the Start menu.'
  }

  'guard-on' {
    # The guard runs as SYSTEM, so it must not point at a user-writable script: keep an
    # admin-owned copy under ProgramData and run that one.
    New-Item -ItemType Directory -Force -Path $StateDir | Out-Null
    $GuardScript = Join-Path $StateDir 'synapse-standby.ps1'
    if ($PSCommandPath -ne $GuardScript) { Copy-Item -LiteralPath $PSCommandPath -Destination $GuardScript -Force }
    $act = New-ScheduledTaskAction -Execute 'powershell.exe' `
      -Argument "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$GuardScript`" standby -Quiet$(if ($KeepLighting) { ' -KeepLighting' })"
    $trg = @((New-ScheduledTaskTrigger -AtStartup), (New-ScheduledTaskTrigger -AtLogOn))
    $pri = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -RunLevel Highest
    Register-ScheduledTask -TaskName $GuardName -Action $act -Trigger $trg -Principal $pri -Force | Out-Null
    Say 'Guard on: standby re-applies at every boot and logon.'
  }

  'guard-off' {
    Unregister-ScheduledTask -TaskName $GuardName -Confirm:$false -ErrorAction SilentlyContinue
    Say 'Guard off.'
  }
}

if (-not $Quiet -and $Host.Name -eq 'ConsoleHost') { Start-Sleep -Seconds 2 }
