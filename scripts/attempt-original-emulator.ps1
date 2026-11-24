#Requires -Version 7.0
<#
Experimental vendor setup on a disposable windows-2022 runner, not a boot/parity test.
The only payload selected is the EXE advertised by Microsoft Download Center 43719.
VS2013 setup options: https://learn.microsoft.com/en-us/previous-versions/visualstudio/visual-studio-2013/e2h7fzkw(v=vs.120)#using-command-line-parameters
No prerequisite overrides, elevation requests, feature changes, reboot, or XDE launch.
#>
[CmdletBinding()]
param(
    [string] $OutputDirectory = (Join-Path $PSScriptRoot '..\target\original-emulator'),
    [ValidateRange(30, 600)] [int] $TimeoutSeconds = 600
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
            throw 'The vendor redirect no longer names the exact selected EXE.'
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

function Get-InstallerMetadata([string] $Html) {
    # Current Download Center embeds its file metadata in escaped React JSON.
    # Decode data only; never execute page scripts or accept an alternate mirror.
    $decoded = [Net.WebUtility]::HtmlDecode($Html).Replace('\/', '/').Replace('\u002F', '/').Replace('\u002f', '/').Replace('\u0026', '&').Replace('\u0022', '"')
    $decoded = $decoded.Replace('\"', '"').Replace('\"', '"')
    $pattern = '\{[^{}]*"name"\s*:\s*"windowsphone81sdkupdate1\.exe"[^{}]*\}'
    $entries = @([regex]::Matches($decoded, $pattern, 'IgnoreCase') | ForEach-Object {
        $_.Value | ConvertFrom-Json
    } | Sort-Object -Property url -Unique)
    if ($entries.Count -ne 1) { throw 'Microsoft did not advertise exactly one matching EXE metadata entry.' }
    $entry = $entries[0]
    if ([int64]$entry.size -le 0 -or [int64]$entry.size -gt 8MB) { throw 'Unexpected bootstrapper size in vendor metadata.' }
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
    $catalog = Get-InstallerMetadata ([Text.Encoding]::UTF8.GetString($page.Bytes))
    $result.vendor_metadata = $catalog
    $result.page_sha256 = (Get-FileHash -LiteralPath $pagePath -Algorithm SHA256).Hash.ToLowerInvariant()
    $result.page_redirects = $page.RedirectChain
    $result.stage = 'download_bootstrapper'
    Save-Result
    $download = Get-MicrosoftContent ([uri]$catalog.url) 'download.microsoft.com' 'windowsphone81sdkupdate1.exe'
    if ($download.Bytes.Length -ne [int64]$catalog.size) { throw 'The bootstrapper length differs from the authentic page metadata.' }
    $installerPath = Join-Path $workRoot 'windowsphone81sdkupdate1.exe'
    [IO.File]::WriteAllBytes($installerPath, $download.Bytes)
    $result.download_url = $download.Url
    $result.download_redirects = $download.RedirectChain
    $result.download_bytes = $download.Bytes.Length
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
    $result.stage = 'vendor_setup'
    $result.status = 'installer_running'
    $installerLog = Join-Path $logsRoot 'setup.log'
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $installerPath
    $start.WorkingDirectory = $workRoot
    $start.UseShellExecute = $false
    foreach ($argument in @('/quiet', '/norestart', '/log', $installerLog)) { $start.ArgumentList.Add($argument) }
    $result.installer_arguments = @('/quiet', '/norestart', '/log', $installerLog)
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
        @("## Original Windows Phone reference setup", "", "Outcome: **$($result.status)**", "", "Stage: $($result.stage)", "", 'No emulator launch, reboot or parity validation was performed. See the retained result, vendor logs and before/after probes.') |
            Add-Content -LiteralPath $env:GITHUB_STEP_SUMMARY -Encoding utf8
    }
    # Remove only this invocation's GUID-named temporary payload directory.
    Remove-Item -LiteralPath $workRoot -Recurse -Force -ErrorAction SilentlyContinue
}
exit $scriptExitCode
