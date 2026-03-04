$ErrorActionPreference = 'Stop'

$files = @(
  'InfiniteRefill/运维/30组的号.txt',
  'InfiniteRefill/运维/30组的号x200x2.txt',
  'InfiniteRefill/运维/50组的号.txt'
)

$set = New-Object 'System.Collections.Generic.HashSet[string]'
foreach ($f in $files) {
  Get-Content -LiteralPath $f | ForEach-Object {
    $k = $_.Trim()
    if ($k -and $k.StartsWith('k_')) {
      [void]$set.Add($k)
    }
  }
}

$keys = @($set)
$sha = [System.Security.Cryptography.SHA256]::Create()
$hashes = foreach ($k in $keys) {
  $bytes = [System.Text.Encoding]::UTF8.GetBytes($k)
  ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') }) -join ''
}
$hashes = $hashes | Sort-Object -Unique

$rows = $hashes | ForEach-Object { "('$_')" }
$vals = [string]::Join(",`r`n", $rows)

$sql = @"
DELETE FROM keep_hashes_tmp;
INSERT OR IGNORE INTO keep_hashes_tmp(key_hash) VALUES
$vals;
SELECT COUNT(*) AS keep_count FROM keep_hashes_tmp;
"@

Set-Content -LiteralPath 'InfiniteRefill/server/tmp_keep_hashes.sql' -Value $sql -Encoding UTF8

Write-Output ("raw_keys=" + $keys.Count)
Write-Output ("unique_hashes=" + $hashes.Count)
Write-Output "sql_file=InfiniteRefill/server/tmp_keep_hashes.sql"
