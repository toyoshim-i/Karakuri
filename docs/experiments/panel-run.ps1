# Drives the three runs in frame-cost-off-this-machine.md, on Windows.
#
# **This is a convenience and not the protocol.** Doing the runs by hand is
# equally good. It exists because the reading is invalid if anything touches the
# window while it is being taken, and a pointer that drifts over the window — or
# a hand reaching for `esc` a little early — is exactly that.
#
#   .\panel-run.ps1 -All -Out .\readouts
#
# writes readouts\run1-untouched.txt, run2-folded.txt and run3-loaded.txt, each
# carrying the window geometry and the graphics modules the process loaded
# alongside the program's own output. Or one at a time:
#
#   .\panel-run.ps1 -Out run1.txt
#   .\panel-run.ps1 -Out run2.txt -Fold
#   .\panel-run.ps1 -Out run3.txt -Load
#
# It expects the example to be built already:
#
#   cargo build -p karakuri-console --example panel
#
# and launches target\debug\examples\panel.exe rather than going through
# `cargo run`, so that the process this script holds is the process that owns
# the window. Same profile and same binary either way.
#
# ## Two things this got wrong before it got them right
#
# **The reading is one-shot.** It fires three seconds after the window was last
# touched by anything and `Costs::said` makes sure it is never taken twice. Every
# event resets that countdown, so the fold has all the time it needs *provided
# the first touch lands inside the first three seconds* — after that the reading
# has already been printed and folding the picture changes nothing but the
# picture. That is why the pointer is put on the window the moment it exists.
#
# **`MainWindowHandle` answers before the window is real.** Caught at creation it
# is a 16x16 box at the origin, and which top-level window it names is
# timing-dependent, so the handle is re-read every pass and only a plausible
# rectangle is accepted.
#
# It also aims the fold at the rectangle the panel reports for `program-view`
# rather than at a fraction of the window, because guessing put the pointer in a
# gap and the panel answered `fold: nothing under the pointer`.

param(
  # Where to write the readout. With -All, a directory instead.
  [Parameter(Mandatory = $true)][string]$Out,
  # Run 2: fold the picture away before the reading is taken.
  [switch]$Fold,
  # Run 3: spin every core but two for the length of the run.
  [switch]$Load,
  # All three runs in sequence, into $Out as a directory.
  [switch]$All,
  # How long to leave the window alone. The protocol asks for at least thirty.
  [int]$Seconds = 35
)

$ErrorActionPreference = 'Stop'

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class PanelWin {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref POINT p);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
}
"@

# No P/Invoke callbacks anywhere in here on purpose. An earlier version passed a
# delegate to EnumWindows to find the window; .NET collected it while the OS
# still held the pointer, and a fatal-error dialog is not a result.
function Send-Key([byte]$vk) {
  [PanelWin]::keybd_event($vk, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [PanelWin]::keybd_event($vk, 0, 2, [UIntPtr]::Zero)
}

function Get-RepoRoot {
  # docs\experiments\panel-run.ps1 -> the workspace root.
  return (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
}

function Invoke-PanelRun {
  param([string]$OutFile, [switch]$DoFold, [switch]$DoLoad, [int]$ForSeconds)

  $repo = Get-RepoRoot
  $exe  = Join-Path $repo 'target\debug\examples\panel.exe'
  if (-not (Test-Path $exe)) {
    throw "$exe is missing. Build it first: cargo build -p karakuri-console --example panel"
  }

  $so    = "$OutFile.stdout"
  $se    = "$OutFile.stderr"
  $notes = @()
  $load  = @()

  if ($DoLoad) {
    # Every core but two, so the panel's own thread is not the one starved. The
    # point is the machine's power state, not a stress test.
    $n = [Math]::Max(1, [Environment]::ProcessorCount - 2)
    $load = 1..$n | ForEach-Object {
      Start-Job -ScriptBlock { $x = 0.0; while ($true) { $x = [Math]::Sqrt($x + 1.0) * 1.0000001 } }
    }
    Start-Sleep -Seconds 3
    $notes += "cpu load: $n of $([Environment]::ProcessorCount) logical processors spinning"
  }

  try {
    # Park the pointer clear of wherever the window will open.
    [void][PanelWin]::SetCursorPos(1, 1)

    $p = Start-Process -FilePath $exe -WorkingDirectory $repo -PassThru `
                       -RedirectStandardOutput $so -RedirectStandardError $se

    # Wait on the *client* rectangle, and wait for it to stop changing. The
    # window is still being sized while its frame already measures over a
    # thousand pixels, so a geometry noted on the first plausible reading
    # misdescribes the run. Settling rather than a threshold, because a
    # threshold in pixels would be a threshold on the DPI scale.
    $hwnd = [IntPtr]::Zero
    $wr   = New-Object PanelWin+RECT
    $cr   = New-Object PanelWin+RECT
    $last = ''
    for ($i = 0; $i -lt 400; $i++) {
      Start-Sleep -Milliseconds 50
      $p.Refresh()
      if ($p.HasExited) { break }
      $h = $p.MainWindowHandle
      if ($h -eq [IntPtr]::Zero) { continue }
      [void][PanelWin]::GetClientRect($h, [ref]$cr)
      if ($cr.Right -lt 640 -or $cr.Bottom -lt 400) { $last = ''; continue }
      $now = "$($cr.Right)x$($cr.Bottom)"
      if ($now -ne $last) { $last = $now; continue }
      [void][PanelWin]::GetWindowRect($h, [ref]$wr)
      $hwnd = $h
      break
    }

    if ($hwnd -eq [IntPtr]::Zero) {
      $notes += "NO WINDOW (process exited: $($p.HasExited))"
    } else {
      $origin = New-Object PanelWin+POINT
      [void][PanelWin]::ClientToScreen($hwnd, [ref]$origin)
      $notes += "window rect: $($wr.Left),$($wr.Top) - $($wr.Right),$($wr.Bottom)"
      $notes += "client: $($cr.Right) x $($cr.Bottom) at screen $($origin.X),$($origin.Y)"

      if ($DoFold) {
        # Touch it at once — this is what slides the three-second countdown so
        # that everything below has room.
        $safeX = $origin.X + [int]($cr.Right / 2)
        $safeY = $origin.Y + [int]($cr.Bottom / 2)
        [void][PanelWin]::SetCursorPos($safeX, $safeY)
        [void][PanelWin]::SetForegroundWindow($hwnd)
        Start-Sleep -Milliseconds 300

        # Ask the panel where the picture is rather than guessing at it.
        Send-Key 0x50   # p
        Start-Sleep -Milliseconds 600
        $sofar = Get-Content $so -Raw -ErrorAction SilentlyContinue
        $m = [regex]::Match($sofar, 'program-view\s+([\d.]+)\s*,\s*([\d.]+)\s+([\d.]+)\s*x\s*([\d.]+)')
        if ($m.Success) {
          $lx = [double]$m.Groups[1].Value; $ly = [double]$m.Groups[2].Value
          $lw = [double]$m.Groups[3].Value; $lh = [double]$m.Groups[4].Value
          $notes += "program-view logical rect: $lx,$ly  $lw x $lh"
          # The client area is the logical viewport times the DPI scale. At 100%
          # they are the same; anywhere else this needs the scale factor, and so
          # does every figure in the readout.
          $tx = $origin.X + [int]($lx + $lw / 2)
          $ty = $origin.Y + [int]($ly + $lh / 2)
        } else {
          $notes += "could not read program-view's rectangle from p; aiming at the client centre"
          $tx = $safeX; $ty = $safeY
        }

        $folded = $false
        foreach ($attempt in 1..6) {
          [void][PanelWin]::SetForegroundWindow($hwnd)
          Start-Sleep -Milliseconds 150
          # Two positions, so a pointer that is already there still moves.
          [void][PanelWin]::SetCursorPos($tx - 1, $ty)
          Start-Sleep -Milliseconds 50
          [void][PanelWin]::SetCursorPos($tx, $ty)
          Start-Sleep -Milliseconds 150
          Send-Key 0x46   # f
          Start-Sleep -Milliseconds 400
          $sofar = Get-Content $so -Raw -ErrorAction SilentlyContinue
          if ($sofar -match 'fold: program-view is now folded') { $folded = $true; break }
          $notes += "fold attempt ${attempt} did not land at ($tx, $ty)"
        }
        $notes += "fold landed: $folded (aimed at $tx, $ty)"
        [void][PanelWin]::SetCursorPos(1, 1)
      }

      # Which backend wgpu chose is not printed anywhere, so ask the OS what the
      # process loaded. It is a hint and not an answer: both backends' runtimes
      # get loaded during enumeration.
      try {
        $mods = (Get-Process -Id $p.Id).Modules | ForEach-Object { $_.ModuleName }
        $notes += 'graphics modules: ' + (($mods |
          Where-Object { $_ -match 'vulkan|amdvlk|nvoglv|d3d12|D3D12Core|dxgi|opengl' } |
          Sort-Object -Unique) -join ', ')
      } catch { $notes += 'modules: unavailable' }

      Start-Sleep -Seconds $ForSeconds

      [void][PanelWin]::SetForegroundWindow($hwnd)
      Start-Sleep -Milliseconds 300
      Send-Key 0x1B     # esc
      if (-not $p.WaitForExit(15000)) { $notes += 'esc did not quit it; killed'; $p.Kill() }
    }
  } finally {
    if ($load) { $load | Stop-Job -PassThru | Remove-Job -Force | Out-Null }
  }

  Start-Sleep -Milliseconds 500
  $text = ($notes -join "`n") +
          "`n`n=== STDOUT ===`n" + (Get-Content $so -Raw -ErrorAction SilentlyContinue) +
          "`n=== STDERR ===`n" + (Get-Content $se -Raw -ErrorAction SilentlyContinue)
  # UTF-8 with no BOM: the readout has em-dashes in it and the file is meant to
  # be pasted into a report.
  [IO.File]::WriteAllText($OutFile, $text, (New-Object Text.UTF8Encoding($false)))
  Write-Host "wrote $OutFile"
}

if ($All) {
  New-Item -ItemType Directory -Force -Path $Out | Out-Null
  Invoke-PanelRun -OutFile (Join-Path $Out 'run1-untouched.txt') -ForSeconds $Seconds
  Invoke-PanelRun -OutFile (Join-Path $Out 'run2-folded.txt')    -DoFold -ForSeconds $Seconds
  Invoke-PanelRun -OutFile (Join-Path $Out 'run3-loaded.txt')    -DoLoad -ForSeconds $Seconds
  Write-Host ''
  Write-Host "Three readouts in $Out. Paste them whole, and say your refresh rate and DPI scale."
} else {
  Invoke-PanelRun -OutFile $Out -DoFold:$Fold -DoLoad:$Load -ForSeconds $Seconds
}
