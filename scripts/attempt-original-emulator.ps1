#Requires -Version 7.0
<#
Experimental vendor setup on a disposable windows-2022 runner, not a boot/parity test.
Only the EXE or offline ISO advertised by Microsoft Download Center 43719 is selected.
VS2013 setup options: https://learn.microsoft.com/en-us/previous-versions/visualstudio/visual-studio-2013/e2h7fzkw(v=vs.120)#using-command-line-parameters
No prerequisite overrides, elevation requests, feature changes, reboot, or XDE launch.
#>
[CmdletBinding()]
param(
    [string] $OutputDirectory = (Join-Path $PSScriptRoot '..\target\original-emulator'),
    [ValidateSet('web', 'offline-iso')] [string] $Mode = 'web',
    [ValidateRange(30, 900)] [int] $TimeoutSeconds = 600
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'This attempt requires the disposable Windows workflow host.' }

$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
$logsRoot = Join-Path $outputRoot 'logs'
[void][IO.Directory]::CreateDirectory($logsRoot)
# Payloads and installer scratch files are deliberately outside the artifact directory.
$workRoot = Join-Path ([IO.Path]::GetTempPath()) ('phone-original-sdk-' + [guid]::NewGuid().ToString('N'))
$setupTemp = Join-Path $workRoot 'temp'
[void][IO.Directory]::CreateDirectory($setupTemp)
$utf8 = [Text.UTF8Encoding]::new($false)
$clock = [Diagnostics.Stopwatch]::new()
$setupProcess = $null
$mountedIsoPath = $null
$scriptExitCode = 1
$result = [ordered]@{
    schema = 1
    started_at_utc = [DateTime]::UtcNow.ToString('o')
    head_sha = $env:GITHUB_SHA
    run_id = $env:GITHUB_RUN_ID
    run_attempt = $env:GITHUB_RUN_ATTEMPT
    runner_image = $env:ImageOS
    runner_image_version = $env:ImageVersion
    status = 'preparing'
    stage = 'before_probe'
    mode = $Mode
    budget_seconds = $TimeoutSeconds
    source_page = 'https://www.microsoft.com/en-us/download/details.aspx?id=43719'
    setup_option_source = 'https://learn.microsoft.com/en-us/previous-versions/visualstudio/visual-studio-2013/e2h7fzkw(v=vs.120)#using-command-line-parameters'
    target_guest_package_version = '8.10.14141.167'
    original_emulator_launched = $false
    guest_boot_verified = $false
    parity_verified = $false
    errors = [Collections.Generic.List[string]]::new()
}

function Save-Result {
    [IO.File]::WriteAllText((Join-Path $outputRoot 'result.json'), ($result | ConvertTo-Json -Depth 12), $utf8)
}

function Invoke-Probe([string] $Phase) {
    $probePath = Join-Path $outputRoot ($Phase + '.json')
    $stderrPath = Join-Path $outputRoot ($Phase + '-stderr.txt')
    & python (Join-Path $PSScriptRoot 'probe-original-emulator.py') --json --timeout 20 2> $stderrPath |
        Set-Content -LiteralPath $probePath -Encoding utf8NoBOM
    if ($LASTEXITCODE -ne 0) { throw "The $Phase read-only probe exited with $LASTEXITCODE." }
    return Get-Content -LiteralPath $probePath -Raw | ConvertFrom-Json
}

function Get-RemainingSeconds {
    $remaining = $TimeoutSeconds - $clock.Elapsed.TotalSeconds
    if ($remaining -le 0) { throw [TimeoutException]::new('The combined download/setup budget expired.') }
    return [Math]::Max(1, [int][Math]::Floor($remaining))
}

function Get-MicrosoftContent([uri] $Uri, [string] $AllowedHost, [string] $ExpectedFile = '') {
    $chain = [Collections.Generic.List[string]]::new()
    for ($redirect = 0; $redirect -le 5; $redirect++) {
        if ($Uri.Scheme -ne 'https' -or $Uri.DnsSafeHost -ne $AllowedHost -or
            -not $Uri.IsDefaultPort -or $Uri.UserInfo -or $Uri.Fragment) {
            throw "Refusing a non-vendor HTTPS download location: $Uri"
        }
        if ($ExpectedFile -and [IO.Path]::GetFileName($Uri.AbsolutePath) -ine $ExpectedFile) {
            throw 'The vendor redirect no longer names the exact selected file.'
        }
        $chain.Add($Uri.AbsoluteUri)
        $handler = [Net.Http.HttpClientHandler]::new()
        $handler.AllowAutoRedirect = $false
        $client = [Net.Http.HttpClient]::new($handler)
        $client.Timeout = [TimeSpan]::FromSeconds([Math]::Min(90, (Get-RemainingSeconds)))
        $client.MaxResponseContentBufferSize = 8MB
        $response = $null
        try {
            $response = $client.GetAsync($Uri).GetAwaiter().GetResult()
            if ([int]$response.StatusCode -ge 300 -and [int]$response.StatusCode -lt 400) {
                if (-not $response.Headers.Location) { throw 'The vendor redirect has no Location.' }
                $Uri = [uri]::new($Uri, $response.Headers.Location)
                continue
            }
            [void]$response.EnsureSuccessStatusCode()
            return [pscustomobject]@{
                Bytes = $response.Content.ReadAsByteArrayAsync().GetAwaiter().GetResult()
                Url = $Uri.AbsoluteUri
                RedirectChain = @($chain)
                Status = [int]$response.StatusCode
            }
        } finally {
            if ($response) { $response.Dispose() }
            $client.Dispose()
        }
    }
    throw 'Too many Microsoft download redirects.'
}

function Save-MicrosoftIso([uri] $Uri, [string] $Destination, [int64] $ExpectedBytes) {
    # ResponseHeadersRead and a bounded buffer prevent a ~941 MiB memory allocation.
    # The cancellation token bounds the body stream as well as the response headers.
    $chain = [Collections.Generic.List[string]]::new()
    for ($redirect = 0; $redirect -le 5; $redirect++) {
        if ($Uri.Scheme -ne 'https' -or $Uri.DnsSafeHost -ne 'download.microsoft.com' -or
            -not $Uri.IsDefaultPort -or $Uri.UserInfo -or $Uri.Fragment -or
            [IO.Path]::GetFileName($Uri.AbsolutePath) -ine 'windowsphone81sdkupdate1.iso') {
            throw "Refusing an unexpected offline ISO location: $Uri"
        }
        $chain.Add($Uri.AbsoluteUri)
        $handler = [Net.Http.HttpClientHandler]::new()
        $handler.AllowAutoRedirect = $false
        $client = [Net.Http.HttpClient]::new($handler)
        $client.Timeout = [Threading.Timeout]::InfiniteTimeSpan
        $cancel = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds((Get-RemainingSeconds)))
        $request = [Net.Http.HttpRequestMessage]::new([Net.Http.HttpMethod]::Get, $Uri)
        $response = $null; $inputStream = $null; $outputStream = $null
        try {
            $response = $client.SendAsync($request, [Net.Http.HttpCompletionOption]::ResponseHeadersRead, $cancel.Token).GetAwaiter().GetResult()
            if ([int]$response.StatusCode -ge 300 -and [int]$response.StatusCode -lt 400) {
                if (-not $response.Headers.Location) { throw 'The vendor ISO redirect has no Location.' }
                $Uri = [uri]::new($Uri, $response.Headers.Location)
                continue
            }
            [void]$response.EnsureSuccessStatusCode()
            if ($null -ne $response.Content.Headers.ContentLength -and $response.Content.Headers.ContentLength -ne $ExpectedBytes) {
                throw 'The ISO response length differs from the authentic catalog metadata.'
            }
            $inputStream = $response.Content.ReadAsStreamAsync($cancel.Token).GetAwaiter().GetResult()
            $outputStream = [IO.File]::Create($Destination)
            $buffer = [byte[]]::new(1MB)
            [int64]$received = 0
            while (($count = $inputStream.ReadAsync($buffer, 0, $buffer.Length, $cancel.Token).GetAwaiter().GetResult()) -gt 0) {
                $received += $count
                if ($received -gt $ExpectedBytes) { throw 'The ISO stream exceeds the exact catalog size.' }
                # Task completion is not function output; only return metadata.
                [void]$outputStream.WriteAsync($buffer, 0, $count, $cancel.Token).GetAwaiter().GetResult()
            }
            if ($received -ne $ExpectedBytes) { throw 'The ISO stream ended before the exact catalog size.' }
            return [pscustomobject]@{ Bytes = $received; Url = $Uri.AbsoluteUri; RedirectChain = @($chain); Status = [int]$response.StatusCode }
        } finally {
            if ($outputStream) { $outputStream.Dispose() }
            if ($inputStream) { $inputStream.Dispose() }
            if ($response) { $response.Dispose() }
            $request.Dispose(); $cancel.Dispose(); $client.Dispose()
        }
    }
    throw 'Too many Microsoft ISO redirects.'
}

function Get-InstallerMetadata([string] $Html, [string] $FileName) {
    # Current Download Center embeds its file metadata in escaped React JSON.
    # Decode data only; never execute page scripts or accept an alternate mirror.
    $decoded = [Net.WebUtility]::HtmlDecode($Html).Replace('\/', '/').Replace('\u002F', '/').Replace('\u002f', '/').Replace('\u0026', '&').Replace('\u0022', '"')
    $decoded = $decoded.Replace('\"', '"').Replace('\"', '"')
    $pattern = '\{[^{}]*"name"\s*:\s*"' + [regex]::Escape($FileName) + '"[^{}]*\}'
    $entries = @([regex]::Matches($decoded, $pattern, 'IgnoreCase') | ForEach-Object {
        $_.Value | ConvertFrom-Json
    } | Sort-Object -Property url -Unique)
    if ($entries.Count -ne 1) { throw 'Microsoft did not advertise exactly one matching file metadata entry.' }
    $entry = $entries[0]
    if ($FileName -eq 'windowsphone81sdkupdate1.iso') {
        if ([int64]$entry.size -ne 986316800) { throw 'The advertised ISO size changed; review the vendor catalog before another attempt.' }
    } elseif ([int64]$entry.size -le 0 -or [int64]$entry.size -gt 8MB) {
        throw 'Unexpected bootstrapper size in vendor metadata.'
    }
    return $entry
}

function Stop-OwnedSetup {
    if ($setupProcess -and -not $setupProcess.HasExited) {
        $result.owned_process_tree_stop_requested = $true
        $setupProcess.Kill($true)
        $result.owned_root_exited_after_stop = $setupProcess.WaitForExit(10000)
    }
}

Save-Result
try {
    $before = Invoke-Probe 'before'
    $result.before_assessment = $before.assessment
    $clock.Start()
    $result.stage = 'download_metadata'
    Save-Result
    $page = Get-MicrosoftContent ([uri]$result.source_page) 'www.microsoft.com'
    $pagePath = Join-Path $outputRoot 'download-page.html'
    [IO.File]::WriteAllBytes($pagePath, $page.Bytes)
    $selectedFile = if ($Mode -eq 'offline-iso') { 'windowsphone81sdkupdate1.iso' } else { 'windowsphone81sdkupdate1.exe' }
    $catalog = Get-InstallerMetadata ([Text.Encoding]::UTF8.GetString($page.Bytes)) $selectedFile
    $result.vendor_metadata = $catalog
    $result.page_sha256 = (Get-FileHash -LiteralPath $pagePath -Algorithm SHA256).Hash.ToLowerInvariant()
    $result.page_redirects = $page.RedirectChain
    $installerDirectory = $workRoot
    if ($Mode -eq 'offline-iso') {
        $result.stage = 'download_iso'
        Save-Result
        $isoPath = Join-Path $workRoot $selectedFile
        $download = Save-MicrosoftIso ([uri]$catalog.url) $isoPath ([int64]$catalog.size)
        $result.download_bytes = $download.Bytes
        $result.iso_sha256 = (Get-FileHash -LiteralPath $isoPath -Algorithm SHA256).Hash.ToLowerInvariant()
        $result.stage = 'mount_iso_read_only'
        $result.download_url = $download.Url
        $result.download_redirects = $download.RedirectChain
        Save-Result
        # The marker lets the parent clean up this owned image even after a timeout.
        $mountedIsoPath = $isoPath
        [ordered]@{ image_path = $isoPath; work_root = $workRoot; storage_type = 'ISO'; access = 'ReadOnly' } |
            ConvertTo-Json | Set-Content (Join-Path $outputRoot 'mounted-iso.json') -Encoding utf8NoBOM
        $diskImage = Mount-DiskImage -ImagePath $isoPath -StorageType ISO -Access ReadOnly -PassThru -ErrorAction Stop
        $volumes = @($diskImage | Get-Volume | Where-Object DriveLetter)
        if ($volumes.Count -ne 1) { throw 'The ISO did not expose exactly one mounted volume; inspect the retained inventory before selecting setup.' }
        $installerDirectory = $volumes[0].DriveLetter + ':\'
        $layout = @(Get-ChildItem -LiteralPath $installerDirectory -Recurse -File | Sort-Object FullName | ForEach-Object {
            [ordered]@{ path = [IO.Path]::GetRelativePath($installerDirectory, $_.FullName); bytes = $_.Length }
        })
        [ordered]@{ root = $installerDirectory; label = $volumes[0].FileSystemLabel; files = $layout } |
            ConvertTo-Json -Depth 6 | Set-Content (Join-Path $outputRoot 'iso-inventory.json') -Encoding utf8NoBOM
        $result.iso_file_count = $layout.Count
        $result.iso_volume_label = $volumes[0].FileSystemLabel
        $result.stage = 'select_iso_setup'
        Save-Result
        # This exact root EXE was observed in the official ISO directory. Never
        # substitute Preparation.exe, individual MSIs, an autorun command or XDE.
        $installerPath = Join-Path $installerDirectory 'MobileTools_EmulatorWP81GDR1.exe'
        $rootExecutables = @(Get-ChildItem -LiteralPath $installerDirectory -File -Filter '*.exe')
        if ($rootExecutables.Count -ne 1 -or -not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
            throw 'The offline root setup selection is ambiguous; review iso-inventory.json before executing anything.'
        }
        $result.iso_setup_relative_path = 'MobileTools_EmulatorWP81GDR1.exe'
    } else {
        $result.stage = 'download_bootstrapper'
        Save-Result
        $download = Get-MicrosoftContent ([uri]$catalog.url) 'download.microsoft.com' $selectedFile
        if ($download.Bytes.Length -ne [int64]$catalog.size) { throw 'The bootstrapper length differs from the authentic page metadata.' }
        $installerPath = Join-Path $workRoot $selectedFile
        [IO.File]::WriteAllBytes($installerPath, $download.Bytes)
        $result.download_url = $download.Url
        $result.download_redirects = $download.RedirectChain
        $result.download_bytes = $download.Bytes.Length
    }
    $result.bootstrapper_sha256 = (Get-FileHash -LiteralPath $installerPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $result.stage = 'verify_authenticode'
    Save-Result
    $signature = Get-AuthenticodeSignature -LiteralPath $installerPath
    $result.authenticode = [ordered]@{
        status = $signature.Status.ToString()
        message = $signature.StatusMessage
        subject = if ($signature.SignerCertificate) { $signature.SignerCertificate.Subject } else { $null }
        issuer = if ($signature.SignerCertificate) { $signature.SignerCertificate.Issuer } else { $null }
        thumbprint = if ($signature.SignerCertificate) { $signature.SignerCertificate.Thumbprint } else { $null }
        timestamp_subject = if ($signature.TimeStamperCertificate) { $signature.TimeStamperCertificate.Subject } else { $null }
    }
    Save-Result
    if ($signature.Status -ne 'Valid' -or -not $signature.SignerCertificate -or
        $signature.SignerCertificate.Subject -notmatch '(^|,\s*)O=Microsoft Corporation(,|$)') {
        throw 'The EXE does not have a valid Authenticode signature from Microsoft Corporation.'
    }
    $version = [Diagnostics.FileVersionInfo]::GetVersionInfo($installerPath)
    # PE/Burn/MUX versions need not equal the guest/package version above.
    $result.bootstrapper_version = [ordered]@{
        file_version = $version.FileVersion
        product_version = $version.ProductVersion
        product_name = $version.ProductName
        original_filename = $version.OriginalFilename
    }
    if ($Mode -eq 'offline-iso' -and ($version.OriginalFilename -ine 'MobileTools_EmulatorWP81GDR1.exe' -or
        $version.ProductName -ne 'Windows Phone 8.1 Update 1 SDK and Emulators - ENU')) {
        throw 'The signed offline EXE is not the observed Windows Phone 8.1 Update SDK setup product.'
    }
    $result.stage = 'vendor_setup'
    $result.status = 'installer_running'
    $installerLog = Join-Path $logsRoot 'setup.log'
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $installerPath
    $start.WorkingDirectory = $installerDirectory
    $start.UseShellExecute = $false
    foreach ($argument in @('/quiet', '/norestart', '/log', $installerLog)) { $start.ArgumentList.Add($argument) }
    $result.installer_arguments = @('/quiet', '/norestart', '/log', $installerLog)
    $result.installer_working_directory = $installerDirectory
    $start.Environment['TEMP'] = $setupTemp
    $start.Environment['TMP'] = $setupTemp
    # A vendor bootstrapper needs no Actions/checkout credentials.
    foreach ($key in @($start.Environment.Keys)) {
        if ($key -match '(?i)TOKEN|PASSWORD|SECRET|CREDENTIAL|AUTHORIZATION') { [void]$start.Environment.Remove($key) }
    }
    $remaining = Get-RemainingSeconds
    $setupProcess = [Diagnostics.Process]::Start($start)
    $result.installer_pid = $setupProcess.Id
    Save-Result
    if (-not $setupProcess.WaitForExit($remaining * 1000)) {
        $result.status = 'installer_timeout'
        Stop-OwnedSetup
        throw [TimeoutException]::new('Vendor setup exceeded the remaining combined download/setup budget.')
    }
    $result.installer_exit_code = $setupProcess.ExitCode
    $unsignedCode = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int32]$setupProcess.ExitCode), 0)
    $result.installer_exit_hex = '0x{0:X8}' -f $unsignedCode
    $result.restart_required = $setupProcess.ExitCode -eq 3010
    $result.status = if ($setupProcess.ExitCode -eq 0) { 'setup_completed' } elseif ($setupProcess.ExitCode -eq 3010) { 'setup_completed_restart_required' } else { 'vendor_setup_failed' }
    if ($setupProcess.ExitCode -in @(0, 3010)) { $scriptExitCode = 0 }
} catch {
    $result.errors.Add($_.Exception.Message)
    if ($result.status -ne 'installer_timeout') { $result.status = 'attempt_failed' }
    Write-Warning $_.Exception.Message
} finally {
    $clock.Stop()
    $result.download_setup_elapsed_seconds = [Math]::Round($clock.Elapsed.TotalSeconds, 3)
    try { Stop-OwnedSetup } catch { $result.errors.Add('Owned-process cleanup: ' + $_.Exception.Message) }
    if ($setupProcess) { $setupProcess.Dispose() }
    if ($mountedIsoPath) {
        try {
            $ownedImage = Get-DiskImage -ImagePath $mountedIsoPath -ErrorAction Stop
            if ($ownedImage.Attached) { Dismount-DiskImage -ImagePath $mountedIsoPath -ErrorAction Stop }
            $result.iso_dismounted = -not (Get-DiskImage -ImagePath $mountedIsoPath -ErrorAction Stop).Attached
            if (-not $result.iso_dismounted) { throw 'The owned ISO is still attached.' }
        } catch { $result.errors.Add('Owned-ISO cleanup: ' + $_.Exception.Message); $scriptExitCode = 1 }
    }
    try {
        $after = Invoke-Probe 'after'
        $result.after_assessment = $after.assessment
        $inventory = @()
        foreach ($path in @($after.installations.xde) + @($after.installations.images)) {
            $file = Get-Item -LiteralPath $path
            $item = [ordered]@{ path = $file.FullName; bytes = $file.Length; last_write_utc = $file.LastWriteTimeUtc.ToString('o') }
            if ($file.Extension -ieq '.exe') {
                $item.file_version = $file.VersionInfo.FileVersion
                $item.sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            }
            $inventory += $item
        }
        $result.installed_inventory = $inventory
        $result.reference_files_detected = @($after.installations.xde).Count -gt 0 -and @($after.installations.images).Count -gt 0
        if ($result.reference_files_detected) {
            Write-Host 'Original XDE and image files detected. Boot configuration must be reviewed before any launch.'
        } elseif ($scriptExitCode -eq 0) {
            $result.status = 'setup_completed_reference_not_detected'
            $scriptExitCode = 1
        }
    } catch { $result.errors.Add('After-probe inventory: ' + $_.Exception.Message); $scriptExitCode = 1 }
    try {
        # Keep only setup logs, never EXE/ISO/VHD/font/package payloads.
        $copied = @(); $total = 0; $index = 0
        foreach ($file in @(Get-ChildItem -LiteralPath $setupTemp -Recurse -File -Filter '*.log' | Sort-Object FullName)) {
            if ($index -ge 128 -or $file.Length -gt 8MB -or $total + $file.Length -gt 32MB) { continue }
            $name = 'vendor-{0:D3}-{1}' -f $index, $file.Name
            Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $logsRoot $name)
            $copied += [ordered]@{ source = $file.FullName; artifact = 'logs/' + $name; bytes = $file.Length }
            $total += $file.Length; $index++
        }
        $result.additional_logs = $copied
    } catch { $result.errors.Add('Setup-log collection: ' + $_.Exception.Message) }
    $result.completed_at_utc = [DateTime]::UtcNow.ToString('o')
    $result.qualification = 'Experimental original SDK installation attempt only. File presence, setup success and Hyper-V services do not prove guest boot or Windows Phone parity.'
    Save-Result
    if ($env:GITHUB_STEP_SUMMARY) {
        @("## Original Windows Phone reference setup", "", "Mode: $Mode", "", "Outcome: **$($result.status)**", "", "Stage: $($result.stage)", "", 'No emulator launch, reboot or parity validation was performed. See the retained result, vendor logs and before/after probes.') |
            Add-Content -LiteralPath $env:GITHUB_STEP_SUMMARY -Encoding utf8
    }
    # Remove only this invocation's GUID-named temporary payload directory.
    Remove-Item -LiteralPath $workRoot -Recurse -Force -ErrorAction SilentlyContinue
}
exit $scriptExitCode
