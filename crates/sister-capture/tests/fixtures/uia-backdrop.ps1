# An owned blank window isolates full-monitor OCR from runner console text.
# Never hide, move or close another process's windows.
function New-SisterUiaBackdrop {
    Add-Type -AssemblyName System.Windows.Forms
    $backdrop = New-Object System.Windows.Forms.Form
    $backdrop.FormBorderStyle = 'None'
    $backdrop.StartPosition = 'Manual'
    $backdrop.Bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $backdrop.BackColor = [System.Drawing.Color]::White
    $backdrop.ShowInTaskbar = $false
    $backdrop.TopMost = $true
    $backdrop.Show()
    $backdrop.Refresh()
    [System.Windows.Forms.Application]::DoEvents()
    return $backdrop
}
