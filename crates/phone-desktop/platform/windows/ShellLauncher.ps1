<#
Reviewable Shell Launcher v2 deployment through Microsoft's AssignedAccess CSP.
The default action only displays the XML. Apply/Restore require the operator's
existing LocalSystem deployment context; this script never elevates itself,
installs a service, creates an account, configures autologon, or signs anyone out.
#>
[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'High')]
param(
    [ValidateSet('Inspect', 'Apply', 'Restore')][string]$Action = 'Inspect',
    [string]$ConfigurationPath = (Join-Path $PSScriptRoot 'ShellLauncher.xml'),
    [string]$BackupPath = (Join-Path $PSScriptRoot 'ShellLauncher.previous.json'),
    [switch]$ReplaceExisting
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ($Action -eq 'Inspect') {
    Get-Content -LiteralPath $ConfigurationPath -Raw
    Write-Output 'No policy changed. Apply is explicit, requires an eligible Windows edition and LocalSystem deployment context, and takes effect at the next sign-in.'
    return
}
if (-not [System.Security.Principal.WindowsIdentity]::GetCurrent().IsSystem) {
    throw 'The device-scoped MDM Bridge requires LocalSystem. Use your managed deployment environment; this script will not elevate itself.'
}
$instances = @(Get-CimInstance -Namespace 'root\cimv2\mdm\dmmap' -ClassName 'MDM_AssignedAccess')
if ($instances.Count -ne 1) { throw 'Expected exactly one AssignedAccess CSP instance.' }
$instance = $instances[0]

if ($Action -eq 'Apply') {
    if (Test-Path -LiteralPath $BackupPath) { throw 'The backup path already exists. Preserve it and choose a new backup path.' }
    $raw = Get-Content -LiteralPath $ConfigurationPath -Raw
    if ($raw.Length -gt 65536) { throw 'The configuration exceeds the supported size.' }
    $readerSettings = [System.Xml.XmlReaderSettings]::new()
    $readerSettings.DtdProcessing = [System.Xml.DtdProcessing]::Prohibit
    $readerSettings.XmlResolver = $null
    $reader = [System.Xml.XmlReader]::Create([System.IO.StringReader]::new($raw), $readerSettings)
    try {
        $document = [System.Xml.XmlDocument]::new()
        $document.XmlResolver = $null
        $document.Load($reader)
    } finally { $reader.Dispose() }
    if ($document.DocumentElement.LocalName -ne 'ShellLauncherConfiguration' -or
        $document.DocumentElement.NamespaceURI -ne 'http://schemas.microsoft.com/ShellLauncher/2018/Configuration') {
        throw 'This is not a Shell Launcher configuration document.'
    }
    if ($instance.ShellLauncher -and -not $ReplaceExisting) {
        throw 'Shell Launcher already has policy. Review it and explicitly use -ReplaceExisting to replace it with the supplied complete policy.'
    }
    $encoded = [System.Net.WebUtility]::HtmlEncode($raw)
    if ($PSCmdlet.ShouldProcess($env:COMPUTERNAME, 'Apply the reviewed per-account Shell Launcher policy at next sign-in')) {
        $backup = @{
            Version = 1; ComputerName = $env:COMPUTERNAME
            PreviousShellLauncher = $instance.ShellLauncher
            AppliedShellLauncher = $encoded
        } | ConvertTo-Json -Depth 3
        # Exclusive creation prevents overwriting the only recovery record.
        $stream = [System.IO.File]::Open($BackupPath, [System.IO.FileMode]::CreateNew, [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)
        try {
            $bytes = [System.Text.UTF8Encoding]::new($false).GetBytes($backup)
            $stream.Write($bytes, 0, $bytes.Length)
            $stream.Flush($true)
        } finally { $stream.Dispose() }
        $instance.ShellLauncher = $encoded
        Set-CimInstance -CimInstance $instance | Out-Null
        Write-Output "Policy submitted. Keep $BackupPath. Sign in with the configured account to validate the session. Other accounts retain the XML's Explorer default profile."
    }
    return
}

$previous = Get-Content -LiteralPath $BackupPath -Raw | ConvertFrom-Json
if ($previous.Version -ne 1 -or $previous.ComputerName -cne $env:COMPUTERNAME) {
    throw 'This recovery record does not belong to this computer or supported format.'
}
if ($instance.ShellLauncher -cne $previous.AppliedShellLauncher) {
    throw 'Shell Launcher policy changed after this deployment. Restore will not overwrite another policy; review it manually.'
}
if ($PSCmdlet.ShouldProcess($env:COMPUTERNAME, 'Restore the exact saved Shell Launcher policy at next sign-in')) {
    $instance.ShellLauncher = $previous.PreviousShellLauncher
    Set-CimInstance -CimInstance $instance | Out-Null
    Write-Output 'The saved Shell Launcher policy was restored. Sign out and back in for the shell change; the recovery record was retained.'
}
