param([Parameter(Mandatory=$true)][string]$SourceExe)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()
$form = New-Object Windows.Forms.Form
$form.Text = 'Keyless — two PC setup'
$form.Size = New-Object Drawing.Size(940,650)
$form.StartPosition = 'CenterScreen'
$intro = New-Object Windows.Forms.Label
$intro.Text = 'Discover your LAN, name two Windows PCs, choose the keyboard/mouse host, then install.'
$intro.SetBounds(16,16,890,35)
$form.Controls.Add($intro)
$grid = New-Object Windows.Forms.DataGridView
$grid.SetBounds(16,60,890,285)
$grid.AllowUserToAddRows = $false
$grid.AllowUserToDeleteRows = $false
$grid.AutoSizeColumnsMode = 'Fill'
$grid.SelectionMode = 'FullRowSelect'
$grid.MultiSelect = $false
$check = New-Object Windows.Forms.DataGridViewCheckBoxColumn
$check.Name = 'Install'; [void]$grid.Columns.Add($check)
foreach($name in @('Nickname','Computer','IP','MAC','Status')) { [void]$grid.Columns.Add($name,$name) }
foreach($name in @('Computer','IP','MAC','Status')) { $grid.Columns[$name].ReadOnly = $true }
$form.Controls.Add($grid)
$scan = New-Object Windows.Forms.Button
$scan.Text = 'Discover PCs'; $scan.SetBounds(16,360,140,35); $form.Controls.Add($scan)
$hostButton = New-Object Windows.Forms.Button
$hostButton.Text = 'Use selected as host'; $hostButton.SetBounds(170,360,190,35); $form.Controls.Add($hostButton)
$hostLabel = New-Object Windows.Forms.Label
$hostLabel.SetBounds(375,368,520,30); $form.Controls.Add($hostLabel)
$install = New-Object Windows.Forms.Button
$install.Text = 'Install selected (max 2)'; $install.SetBounds(16,405,225,40); $form.Controls.Add($install)
$export = New-Object Windows.Forms.Button
$export.Text = 'Create portable setup folder'; $export.SetBounds(255,405,240,40); $form.Controls.Add($export)
$log = New-Object Windows.Forms.TextBox
$log.Multiline=$true; $log.ReadOnly=$true; $log.ScrollBars='Vertical'; $log.SetBounds(16,460,890,140); $form.Controls.Add($log)
function Write-Status([string]$message) { $log.AppendText("$message`r`n"); [Windows.Forms.Application]::DoEvents() }
$script:hostIP = ''
$script:localIPs = @('127.0.0.1')
function Add-PC($computer,$ip,$mac,$status,$selected=$false) {
    [void]$grid.Rows.Add($selected,$computer,$computer,$ip,$mac,$status)
}
function Selected-PCs {
    $grid.EndEdit() | Out-Null
    @($grid.Rows | Where-Object { $_.Cells['Install'].Value -eq $true })
}
$scan.Add_Click({
    $scan.Enabled=$false
    try {
        $grid.Rows.Clear()
        Write-Status 'Discovering IPv4 neighbors. A missing PC can be asleep or block discovery.'
        $adapters = @(Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254.*' -and $_.AddressState -eq 'Preferred' })
        $script:localIPs = @('127.0.0.1') + @($adapters.IPAddress)
        $primary = $adapters | Select-Object -First 1
        if(!$primary) { throw 'No active IPv4 LAN adapter found.' }
        Add-PC $env:COMPUTERNAME $primary.IPAddress '' 'This PC' $true
        $script:hostIP=$primary.IPAddress; $hostLabel.Text="Default host: $script:hostIP"
        # Probe at most the local /24 on each adapter, with asynchronous pings.
        # Wider subnets are intentionally bounded to keep setup quick.
        $probes=@()
        foreach($prefix in @($adapters.IPAddress | ForEach-Object { $_ -replace '\.\d+$','' } | Select-Object -Unique)) {
            foreach($i in 1..254) {
                $ip="$prefix.$i"
                if($ip -in $script:localIPs) { continue }
                $ping=New-Object Net.NetworkInformation.Ping
                $probes+= [pscustomobject]@{ IP=$ip; Ping=$ping; Task=$ping.SendPingAsync($ip,350) }
            }
        }
        foreach($probe in $probes) { try { $null=$probe.Task.GetAwaiter().GetResult() } catch {} finally { $probe.Ping.Dispose() } }
        $neighbors=@(Get-NetNeighbor -AddressFamily IPv4 | Where-Object { $_.State -in @('Reachable','Stale','Delay','Probe','Permanent') -and $_.LinkLayerAddress -ne 'FF-FF-FF-FF-FF-FF' -and $_.IPAddress -notin $script:localIPs })
        foreach($neighbor in @($neighbors | Sort-Object IPAddress -Unique)) {
            $ip=$neighbor.IPAddress; $computer=$ip
            try { $lookup=[Net.Dns]::GetHostEntryAsync($ip); if($lookup.Wait(150)) { $computer=$lookup.Result.HostName } } catch {}
            Add-PC $computer $ip $neighbor.LinkLayerAddress 'Discovered; OS not verified'
        }
        Write-Status 'Discovery complete. Check exactly two PCs. Remote installation requires existing WinRM access and administrator credentials. Nicknames are editable.'
    } catch { Write-Status $_.Exception.Message } finally { $scan.Enabled=$true }
})
$hostButton.Add_Click({
    if($grid.CurrentRow) { $script:hostIP=[string]$grid.CurrentRow.Cells['IP'].Value; $hostLabel.Text="Default host: $script:hostIP" }
})
# The same installer is used locally and over an authenticated WinRM session.
$installer = {
    param($exeBytes,$hostAddress,$role,$nickname,$setupText)
    $ErrorActionPreference='Stop'
    $root=Join-Path $env:LOCALAPPDATA 'Keyless'
    New-Item -ItemType Directory -Path $root -Force | Out-Null
    [IO.File]::WriteAllBytes((Join-Path $root 'keyless.exe'),$exeBytes)
    [IO.File]::WriteAllText((Join-Path $root 'setup.ps1'),$setupText)
    @{nickname=$nickname;host=$hostAddress;role=$role} | ConvertTo-Json | Set-Content (Join-Path $root 'device.json')
    $arguments=if($role -eq 'host'){'host'}else{"client $hostAddress"}
    $shell=New-Object -ComObject WScript.Shell
    $shortcut=$shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Desktop')) 'Keyless.lnk'))
    $shortcut.TargetPath=Join-Path $root 'keyless.exe'; $shortcut.Arguments=$arguments; $shortcut.WorkingDirectory=$root; $shortcut.Save()
    $os=Get-CimInstance Win32_OperatingSystem
    "Installed $nickname ($role) for $env:USERNAME at $root | $($os.Caption). Open the Keyless desktop shortcut in the signed-in desktop."
}
function Validate-Selection {
    $rows=@(Selected-PCs)
    if($rows.Count -ne 2) { throw 'Select exactly two PCs.' }
    if($script:hostIP -notin @($rows | ForEach-Object { [string]$_.Cells['IP'].Value })) { throw 'The default host must be one of the two selected PCs.' }
    return $rows
}
$install.Add_Click({
    $install.Enabled=$false
    try {
        $rows=@(Validate-Selection)
        $bytes=[IO.File]::ReadAllBytes($SourceExe)
        $setupText=[IO.File]::ReadAllText($PSCommandPath)
        foreach($row in $rows) {
            $ip=[string]$row.Cells['IP'].Value; $nickname=[string]$row.Cells['Nickname'].Value
            $role=if($ip -eq $script:hostIP){'host'}else{'client'}
            if($ip -in $script:localIPs) { Write-Status (& $installer $bytes $script:hostIP $role $nickname $setupText) }
            else {
                Write-Status "Connecting to $ip through existing Windows remoting…"
                $credential=Get-Credential -Message "Administrator account on $ip (installation is per-user)"
                if(!$credential) { throw 'Remote installation canceled.' }
                $session=$null
                try {
                    $session=New-PSSession -ComputerName ([string]$row.Cells['Computer'].Value) -Credential $credential
                    Write-Status (Invoke-Command -Session $session -ScriptBlock $installer -ArgumentList $bytes,$script:hostIP,$role,$nickname,$setupText)
                } finally { if($session) { Remove-PSSession $session } }
            }
        }
        Write-Status 'Installation complete. Start the host shortcut first, then the client shortcut. No startup tasks or firewall rules were added.'
    } catch { Write-Status "Installation stopped: $($_.Exception.Message). If remoting is unavailable, use Create portable setup folder and copy it to the second PC." } finally { $install.Enabled=$true }
})
$export.Add_Click({
    try {
        $rows=@(Validate-Selection)
        $dialog=New-Object Windows.Forms.FolderBrowserDialog
        if($dialog.ShowDialog() -ne 'OK') { return }
        $root=Join-Path $dialog.SelectedPath 'Keyless Setup'
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        Copy-Item -LiteralPath $SourceExe -Destination (Join-Path $root 'keyless.exe') -Force
        # Fixed commands contain only validated IPv4 addresses discovered above.
        [IO.File]::WriteAllText((Join-Path $root 'Start host.cmd'),"@echo off`r`ncd /d `"%~dp0`"`r`nkeyless.exe host`r`npause`r`n")
        [IO.File]::WriteAllText((Join-Path $root 'Start second PC.cmd'),"@echo off`r`ncd /d `"%~dp0`"`r`nkeyless.exe client $script:hostIP`r`npause`r`n")
        Write-Status "Portable folder ready: $root. Copy it to both PCs. Run Start host on the chosen host and Start second PC on the other."
    } catch { Write-Status $_.Exception.Message }
})
Write-Status 'Click Discover PCs to begin. Discovery shows network-visible devices, including devices that may not run Windows.'
[void]$form.ShowDialog()
