<#
  Configure repository-local Git hooks path.
  Usage: .\scripts\bootstrap-hooks.ps1
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Resolve repo root (script directory/..)
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $RepoRoot

Write-Host "[bootstrap] Setting Git hooks path to .githooks"
git config core.hooksPath .githooks

Write-Host "[bootstrap] Done. Git will use hooks in .githooks"

