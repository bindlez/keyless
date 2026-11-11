param([Parameter(Mandatory=$true)][string]$SourceExe,[switch]$SmokeTest)
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
$manual = New-Object Windows.Forms.TextBox
$manual.SetBounds(510,415,240,25); $form.Controls.Add($manual)
$add = New-Object Windows.Forms.Button
$add.Text='Add IP / name'; $add.SetBounds(760,405,145,40); $form.Controls.Add($add)
$log = New-Object Windows.Forms.TextBox
$log.Multiline=$true; $log.ReadOnly=$true; $log.ScrollBars='Vertical'; $log.SetBounds(16,460,890,140); $form.Controls.Add($log)
function Write-Status([string]$message) { $log.AppendText("$message`r`n"); [Windows.Forms.Application]::DoEvents() }
$script:hostIP = ''
$script:localIPs = @('127.0.0.1')
function Add-PC($computer,$ip,$mac,$status,$selected=$false) {
    if(@($grid.Rows | Where-Object { [string]$_.Cells['IP'].Value -eq $ip }).Count) { return }
    [void]$grid.Rows.Add($selected,$computer,$computer,$ip,$mac,$status)
}
$add.Add_Click({
    try {
        $value=$manual.Text.Trim()
        if(!$value) { throw 'Enter the other PC IP address or computer name.' }
        $addresses=@([Net.Dns]::GetHostAddresses($value) | Where-Object { $_.AddressFamily -eq [Net.Sockets.AddressFamily]::InterNetwork })
        if(!$addresses.Count) { throw 'No IPv4 address found for that computer.' }
        foreach($address in $addresses) {
            Add-PC $value $address.IPAddressToString '' 'Manually added; reachability not verified'
        }
        Write-Status 'Added. Check this PC and the other PC, then choose the host. Manual entry does not bypass network isolation.'
    } catch { Write-Status $_.Exception.Message }
})
function Selected-PCs {
    $grid.EndEdit() | Out-Null
    @($grid.Rows | Where-Object { $_.Cells['Install'].Value -eq $true })
}
$scan.Add_Click({
    $scan.Enabled=$false
    try {
        $grid.Rows.Clear()
        Write-Status 'Discovering IPv4 neighbors. A missing PC can be asleep or block discovery.'
        # .NET avoids requiring access to the Windows CIM provider. Prefer
        # real LAN interfaces with a gateway over WSL/VPN virtual interfaces.
        $adapters = @(foreach($nic in [Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces()) {
            if($nic.OperationalStatus -ne 'Up' -or $nic.NetworkInterfaceType -eq 'Loopback') { continue }
            $properties=$nic.GetIPProperties()
            $gateway=@($properties.GatewayAddresses | Where-Object { $_.Address.AddressFamily -eq 'InterNetwork' -and $_.Address.ToString() -ne '0.0.0.0' }).Count -gt 0
            foreach($unicast in $properties.UnicastAddresses) {
                if($unicast.Address.AddressFamily -ne 'InterNetwork' -or $unicast.Address.ToString() -like '169.254.*') { continue }
                [pscustomobject]@{IPAddress=$unicast.Address.ToString(); Name=$nic.Name; Gateway=$gateway; MAC=$nic.GetPhysicalAddress().ToString()}
            }
        })
        $adapters=@($adapters | Sort-Object -Property @{Expression='Gateway';Descending=$true},Name)
        $script:localIPs = @('127.0.0.1') + @($adapters.IPAddress)
        $primary = $adapters | Select-Object -First 1
        if(!$primary) { throw 'No active IPv4 LAN adapter found.' }
        Add-PC $env:COMPUTERNAME $primary.IPAddress '' 'This PC' $true
        $script:hostIP=$primary.IPAddress; $hostLabel.Text="Default host: $script:hostIP"
        foreach($adapter in $adapters) { Write-Status "Adapter: $($adapter.Name) — $($adapter.IPAddress), gateway: $($adapter.Gateway)" }
        # Probe at most the local /24 on each adapter, with asynchronous pings.
        # Wider subnets are intentionally bounded to keep setup quick.
        $probes=@()
        $scanAdapters=@($adapters | Where-Object { $_.Gateway })
        if(!$scanAdapters.Count) { $scanAdapters=$adapters }
        foreach($prefix in @($scanAdapters.IPAddress | ForEach-Object { $_ -replace '\.\d+$','' } | Select-Object -Unique)) {
            foreach($i in 1..254) {
                $ip="$prefix.$i"
                if($ip -in $script:localIPs) { continue }
                $ping=New-Object Net.NetworkInformation.Ping
                $probes+= [pscustomobject]@{ IP=$ip; Ping=$ping; Task=$ping.SendPingAsync($ip,350) }
            }
        }
        $found=@{}
        foreach($probe in $probes) {
            try {
                $reply=$probe.Task.GetAwaiter().GetResult()
                if($reply.Status -eq 'Success') { $found[$probe.IP]=[pscustomobject]@{IPAddress=$probe.IP;LinkLayerAddress='';Status='Responded to ping'} }
            } catch {} finally { $probe.Ping.Dispose() }
        }
        # arp.exe works even when the NetNeighbor CIM provider is unavailable.
        foreach($line in @(& arp.exe -a)) {
            if($line -match '^\s*(\d+\.\d+\.\d+\.\d+)\s+([0-9a-fA-F-]{17})\s+') {
                $ip=$Matches[1]; $mac=$Matches[2]
                if($ip -in $script:localIPs -or $mac -eq 'ff-ff-ff-ff-ff-ff' -or $ip -match '^(22[4-9]|23\d)\.') { continue }
                $found[$ip]=[pscustomobject]@{IPAddress=$ip;LinkLayerAddress=$mac;Status='LAN neighbor; may be cached'}
            }
        }
        foreach($neighbor in @($found.Values | Sort-Object IPAddress)) {
            $ip=$neighbor.IPAddress; $computer=$ip
            try { $lookup=[Net.Dns]::GetHostEntryAsync($ip); if($lookup.Wait(150)) { $computer=$lookup.Result.HostName } } catch {}
            Add-PC $computer $ip $neighbor.LinkLayerAddress $neighbor.Status
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
if($SmokeTest) { Write-Output 'Keyless setup window initialized successfully.'; $form.Dispose(); return }
[void]$form.ShowDialog()
