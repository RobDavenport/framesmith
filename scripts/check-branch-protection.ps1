[CmdletBinding()]
param(
    [string]$Owner = "RobDavenport",
    [string]$Repo = "framesmith",
    [string]$Branch = "main",
    [string]$RequiredCheck = "Windows Checks",
    [string]$ProtectionJsonPath,
    [string]$GitHubToken = $env:GITHUB_TOKEN,
    [switch]$RequirePullRequest
)

$ErrorActionPreference = "Stop"

function Fail {
    param([string]$Message)
    Write-Error $Message
    exit 1
}

function Read-ProtectionJson {
    if ($ProtectionJsonPath) {
        if (-not (Test-Path -LiteralPath $ProtectionJsonPath)) {
            Fail "Branch protection JSON file not found: $ProtectionJsonPath"
        }

        return Get-Content -LiteralPath $ProtectionJsonPath -Raw | ConvertFrom-Json
    }

    if (-not $GitHubToken) {
        Fail "GITHUB_TOKEN is required unless -ProtectionJsonPath points to a saved branch-protection response."
    }

    $headers = @{
        "Accept"               = "application/vnd.github+json"
        "Authorization"        = "Bearer $GitHubToken"
        "User-Agent"           = "framesmith-readiness-check"
        "X-GitHub-Api-Version" = "2022-11-28"
    }
    $uri = "https://api.github.com/repos/$Owner/$Repo/branches/$Branch/protection"

    return Invoke-RestMethod -Uri $uri -Headers $headers -Method Get
}

$protection = Read-ProtectionJson

if (-not $protection.required_status_checks) {
    Fail "Branch protection does not require status checks."
}

$statusChecks = $protection.required_status_checks
$contexts = @()

if ($statusChecks.contexts) {
    $contexts += @($statusChecks.contexts)
}

if ($statusChecks.checks) {
    foreach ($check in @($statusChecks.checks)) {
        if ($check.context) {
            $contexts += $check.context
        }
    }
}

$contexts = @($contexts | Where-Object { $_ } | Select-Object -Unique)
$acceptedNames = @($RequiredCheck, "CI / $RequiredCheck")
$hasRequiredCheck = $false

foreach ($name in $acceptedNames) {
    if ($contexts -contains $name) {
        $hasRequiredCheck = $true
    }
}

if (-not $hasRequiredCheck) {
    Fail "Required status check '$RequiredCheck' was not found. Found: $($contexts -join ', ')"
}

if ($statusChecks.strict -ne $true) {
    Fail "Branch protection does not require branches to be up to date before merging."
}

if ($RequirePullRequest -and -not $protection.required_pull_request_reviews) {
    Fail "Pull request review/merge policy is not enabled."
}

if ($protection.allow_force_pushes -and $protection.allow_force_pushes.enabled -eq $true) {
    Fail "Branch protection allows force pushes."
}

if ($protection.allow_deletions -and $protection.allow_deletions.enabled -eq $true) {
    Fail "Branch protection allows branch deletion."
}

Write-Host "Branch protection verified."
Write-Host "Repository: $Owner/$Repo"
Write-Host "Branch: $Branch"
Write-Host "Required checks: $($contexts -join ', ')"
Write-Host "Strict up-to-date requirement: $($statusChecks.strict)"
Write-Host "Pull request policy present: $([bool]$protection.required_pull_request_reviews)"
