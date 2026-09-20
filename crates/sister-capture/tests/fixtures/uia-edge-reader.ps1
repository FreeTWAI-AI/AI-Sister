# Owned Edge profile + local HTML. Only the fixture process tree is controlled.
param([Parameter(Mandatory=$true)][string]$StateDir)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class SisterEdgeWindow {
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd, IntPtr after, int x, int y, int w, int h, uint flags);
}
'@
$browser = $null
try {
    $edge = @("${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe", "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $edge) { throw 'Microsoft Edge is not installed on the native runner' }
    $html = @'
<!doctype html><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'">
<title>Sister Edge loading</title>
<style>body{font:20px sans-serif;margin:24px}main{height:280px;overflow:auto;border:1px solid;padding:8px}p{margin:16px 0}</style>
<main id="reader" role="document" tabindex="0" autofocus>
<p>&#x7db2;&#x9801;&#x96fb;&#x8a71; 0800-333-444</p><p>EDGE-SECOND-PARAGRAPH</p>
<div style="height:4000px">padding</div><p>EDGE-BOTTOM 02-7766-5544</p>
<p hidden>HIDDEN-SENTINEL</p></main>
<p>SIBLING-SENTINEL 0800-999-000</p>
<label>Password <input id="secret" type="password" value="PASSWORD-SENTINEL"></label>
<script>
const reader = document.getElementById('reader');
window.addEventListener('load', () => { reader.focus(); document.title='Sister Edge top'; });
window.addEventListener('keydown', event => {
  if (event.key==='F8') { event.preventDefault(); reader.scrollTop=reader.scrollHeight; document.title='Sister Edge bottom'; }
  if (event.key==='F9') { event.preventDefault(); document.getElementById('secret').focus(); document.title='Sister Edge password'; }
});
</script>
'@
    $htmlPath = Join-Path $StateDir 'reader.html'
    [IO.File]::WriteAllText($htmlPath, $html, [Text.UTF8Encoding]::new($false))
    $uri = ([Uri]$htmlPath).AbsoluteUri
    $profile = Join-Path $StateDir 'edge-profile'
    $browser = Start-Process -FilePath $edge -ArgumentList @("--user-data-dir=`"$profile`"", '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--disable-component-update', '--disable-sync', '--force-renderer-accessibility', '--new-window', "`"$uri`"") -PassThru
    [IO.File]::WriteAllText((Join-Path $StateDir 'provider-pid'), [string]$browser.Id)
    $deadline = [DateTime]::UtcNow.AddSeconds(90)
    $last = ''
    $sent = ''
    while ([DateTime]::UtcNow -lt $deadline) {
        $browser.Refresh()
        if ($browser.HasExited) { throw 'Owned Edge process exited before the test completed' }
        $request = Join-Path $StateDir 'request'
        $mode = if (Test-Path $request) { [IO.File]::ReadAllText($request) } else { '' }
        if ($mode -eq 'stop') { break }
        $hwnd = $browser.MainWindowHandle
        if ($mode -and $mode -ne $last -and $hwnd -ne [IntPtr]::Zero) {
            [SisterEdgeWindow]::SetWindowPos($hwnd, [IntPtr]::new(-1), 40, 40, 760, 620, 0) | Out-Null
            [SisterEdgeWindow]::SetForegroundWindow($hwnd) | Out-Null
            [uint32]$foregroundPid = 0
            $foreground = [SisterEdgeWindow]::GetForegroundWindow()
            [SisterEdgeWindow]::GetWindowThreadProcessId($foreground, [ref]$foregroundPid) | Out-Null
            if ($foreground -ne $hwnd -or $foregroundPid -ne $browser.Id) { Start-Sleep -Milliseconds 40; continue }
            if ($sent -ne $mode) {
                switch ($mode) {
                    'top' { }
                    'bottom' { [System.Windows.Forms.SendKeys]::SendWait('{F8}') }
                    'password' { [System.Windows.Forms.SendKeys]::SendWait('{F9}') }
                    'address' { [System.Windows.Forms.SendKeys]::SendWait('^l') }
                    default { throw "Unknown fixture mode: $mode" }
                }
                $sent = $mode
            }
            $browser.Refresh()
            if ($mode -eq 'address' -or $browser.MainWindowTitle.StartsWith("Sister Edge $mode")) {
                # Only describe metadata under this owned foreground window. This
                # makes a native provider mismatch diagnosable without product logging.
                $node = [System.Windows.Automation.AutomationElement]::FocusedElement
                $metadata = @()
                for ($depth = 0; $depth -lt 8 -and $null -ne $node; $depth++) {
                    $current = $node.Current
                    $metadata += "$depth type=$($current.ControlType.ProgrammaticName) class=$($current.ClassName) rect=$($current.BoundingRectangle) password=$($current.IsPassword) offscreen=$($current.IsOffscreen) focused=$($current.HasKeyboardFocus) text=$($node.GetCurrentPropertyValue([System.Windows.Automation.AutomationElement]::IsTextPatternAvailableProperty))"
                    if ($current.NativeWindowHandle -eq $hwnd.ToInt64()) { break }
                    $node = [System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($node)
                }
                [IO.File]::WriteAllText((Join-Path $StateDir 'metadata'), ($metadata -join "`n"))
                [IO.File]::WriteAllText((Join-Path $StateDir 'ready'), $mode)
                $last = $mode
            }
        }
        Start-Sleep -Milliseconds 40
    }
} catch {
    [IO.File]::WriteAllText((Join-Path $StateDir 'error'), $_.Exception.ToString())
} finally {
    if ($browser) {
        # Unique --user-data-dir means this PID/tree never belongs to a user's Edge session.
        & taskkill.exe /PID $browser.Id /T /F 2>&1 | Out-Null
    }
}
